//! Pictures of windows, for a switcher to show: each captured through ext-image-copy-capture on a
//! Wayland connection of the worker driving [`Capturer`]: into images on the GPU the frontend
//! [`share`]s, which it scales as it draws them, or else into shared memory, then made small.
//! Every window is captured as it appears; while watched, each is captured again whenever its
//! content changes, as the compositor holds each capture until it does.

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    os::fd::{AsFd, AsRawFd, FromRawFd, RawFd},
    ptr::NonNull,
    time::{Duration, Instant},
};

use tracing::debug;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, delegate_noop,
    event_created_child,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::WlBuffer,
        wl_registry::WlRegistry,
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
    },
};
use wayland_protocols::ext::{
    foreign_toplevel_list::v1::client::{
        ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
        ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
    },
    image_capture_source::v1::client::{
        ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
        ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1, Options},
        ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
    },
};

use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
};

use crate::{
    picture::{Frame, Live, Picture},
    share::{self, ARGB8888, Shared, XRGB8888},
    wake,
};

/// Pictures are made no bigger than this, twice what a card shows so they stay sharp at a scale
/// of two.
const SHOT_W: u32 = 472;
const SHOT_H: u32 = 266;
/// Tries again after a capture fails, such as as a window changes size, before waiting for the
/// next time captures are wanted.
const RETRIES: u32 = 3;
/// A window shown live is captured no more often than this, as the compositor copies it whole
/// each time; a card is smooth at it.
const LIVE_EVERY: Duration = Duration::from_micros(16_667);
/// Where in each part of the window a shrunk pixel is averaged from.
const SAMPLES: [(f32, f32); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];

/// What a turn of the [`Capturer`] brought, each more than the one before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum News {
    Nothing,
    /// Pictures already handed out show something new, which only has to be drawn again.
    Moved,
    /// Windows came, went or got their first picture, so the [`Shots`] are handed out again.
    Changed,
}

/// The pictures taken, by each window's identifier as [`key`] writes it, and the identifier of
/// each window by its app and title, for a compositor whose identifiers cannot be matched.
#[derive(Clone, Default)]
pub struct Shots {
    pub pictures: HashMap<String, Picture>,
    pub named: HashMap<(String, String), String>,
}

impl Shots {
    pub fn of(&self, id: &str, app: &str, title: &str) -> Option<&Picture> {
        self.pictures.get(&key(id)).or_else(|| {
            let id = self.named.get(&(app.to_owned(), title.to_owned()))?;
            self.pictures.get(id)
        })
    }
}

/// An identifier as compared: hex digits in lower case, without a prefix or leading zeros.
pub fn key(id: &str) -> String {
    let id = id.trim().to_lowercase();
    let id = id.trim_start_matches("0x").trim_start_matches('0');
    id.to_owned()
}

/// Shared memory the compositor captures into, mapped so a capture is read where it lies.
struct Buffer {
    map: NonNull<u8>,
    length: usize,
    buffer: WlBuffer,
    size: (u32, u32),
    format: wl_shm::Format,
}

impl Buffer {
    fn new(
        shm: &WlShm,
        (width, height): (u32, u32),
        format: wl_shm::Format,
        handle: &QueueHandle<Capture>,
    ) -> Option<Self> {
        let stride = width.checked_mul(4)?;
        let length = stride.checked_mul(height)?;
        // A new descriptor, which the file then owns.
        let fd = unsafe { libc::memfd_create(c"island-capture".as_ptr(), libc::MFD_CLOEXEC) };
        if fd < 0 {
            return None;
        }
        let file = unsafe { File::from_raw_fd(fd) };
        file.set_len(u64::from(length)).ok()?;
        // Read only, and unmapped as the buffer is dropped.
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length as usize,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return None;
        }
        let map = NonNull::new(map.cast::<u8>())?;
        let pool = shm.create_pool(file.as_fd(), i32::try_from(length).ok()?, handle, ());
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            format,
            handle,
            (),
        );
        pool.destroy();
        Some(Self {
            map,
            length: length as usize,
            buffer,
            size: (width, height),
            format,
        })
    }

    /// What was captured, made no bigger than a shot by averaging a few points of each part it
    /// stands for, blue first as shared memory has it, and its size.
    fn shrink(&self) -> (Vec<u8>, (u32, u32)) {
        let (width, height) = self.size;
        // The mapping is `length` bytes and lives as long as the buffer.
        let source = unsafe { std::slice::from_raw_parts(self.map.as_ptr(), self.length) };
        let scale = (SHOT_W as f32 / width as f32)
            .min(SHOT_H as f32 / height as f32)
            .min(1.0);
        let fit_w = ((width as f32 * scale).round() as u32).clamp(1, SHOT_W);
        let fit_h = ((height as f32 * scale).round() as u32).clamp(1, SHOT_H);
        let (step_x, step_y) = (width as f32 / fit_w as f32, height as f32 / fit_h as f32);
        let mut pixels = vec![0_u8; (fit_w * fit_h * 4) as usize];
        for fit_y in 0..fit_h {
            for fit_x in 0..fit_w {
                let mut sum = [0_u32; 4];
                for (dx, dy) in SAMPLES {
                    let x = (((fit_x as f32 + dx) * step_x) as u32).min(width - 1);
                    let y = (((fit_y as f32 + dy) * step_y) as u32).min(height - 1);
                    let at = ((y * width + x) * 4) as usize;
                    for (total, byte) in sum.iter_mut().zip(&source[at..at + 4]) {
                        *total += u32::from(*byte);
                    }
                }
                let pixel = sum.map(|total| (total / SAMPLES.len() as u32) as u8);
                let at = ((fit_y * fit_w + fit_x) * 4) as usize;
                pixels[at..at + 4].copy_from_slice(&pixel);
            }
        }
        (pixels, (fit_w, fit_h))
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.buffer.destroy();
        // Mapped by `new` with this length.
        unsafe { libc::munmap(self.map.as_ptr().cast(), self.length) };
    }
}

/// An image on the GPU the compositor captures into, which the frontend draws as it is.
struct SharedBuffer {
    shared: Shared,
    buffer: WlBuffer,
}

impl SharedBuffer {
    fn new(
        dmabuf: &ZwpLinuxDmabufV1,
        size: (u32, u32),
        fourcc: u32,
        modifiers: &[u64],
        handle: &QueueHandle<Capture>,
    ) -> Option<Self> {
        let shared = share::make(size, fourcc, modifiers)?;
        let params = dmabuf.create_params(handle, ());
        params.add(
            shared.fd.as_fd(),
            0,
            shared.offset,
            shared.stride,
            (shared.modifier >> 32) as u32,
            shared.modifier as u32,
        );
        let buffer = params.create_immed(
            size.0 as i32,
            size.1 as i32,
            fourcc,
            zwp_linux_buffer_params_v1::Flags::empty(),
            handle,
            (),
        );
        params.destroy();
        Some(Self { shared, buffer })
    }
}

impl Drop for SharedBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

/// Two images on the GPU per window: the compositor captures into one while the other is shown.
struct Shared2 {
    images: [SharedBuffer; 2],
    fourcc: u32,
    /// The one captured into next.
    writing: usize,
}

#[derive(Default)]
struct Session {
    session: Option<ExtImageCopyCaptureSessionV1>,
    source: Option<ExtImageCaptureSourceV1>,
    size: Option<(u32, u32)>,
    formats: Vec<wl_shm::Format>,
    /// Formats of the batch of constraints still arriving; they take over at its end.
    arriving: Vec<wl_shm::Format>,
    /// The DRM formats it captures into on the GPU, each with the layouts it takes.
    shared_formats: Vec<(u32, Vec<u64>)>,
    arriving_shared: Vec<(u32, Vec<u64>)>,
    ready: bool,
    frame: Option<ExtImageCopyCaptureFrameV1>,
    /// When the frame was asked for, which paces the next.
    asked: Option<Instant>,
    /// The compositor said the frame differs from the one before.
    damaged: bool,
    buffer: Option<Buffer>,
    shared: Option<Shared2>,
    /// Images on the GPU could not be made for it, so it is captured into shared memory.
    unshared: bool,
}

impl Session {
    /// The buffer to capture into next at `size`: the next of its images on the GPU, made as
    /// needed, or else its shared memory.
    fn target(
        &mut self,
        dmabuf: Option<&ZwpLinuxDmabufV1>,
        shm: &WlShm,
        size: (u32, u32),
        handle: &QueueHandle<Capture>,
    ) -> Option<WlBuffer> {
        if let Some(dmabuf) = dmabuf
            && !self.unshared
            && share::can_make()
        {
            let fits = self.shared.as_ref().is_some_and(|shared| {
                shared.images[0].shared.size == size
                    && self
                        .shared_formats
                        .iter()
                        .any(|(fourcc, _)| *fourcc == shared.fourcc)
            });
            if !fits {
                self.shared = [XRGB8888, ARGB8888].into_iter().find_map(|fourcc| {
                    let (_, modifiers) = self
                        .shared_formats
                        .iter()
                        .find(|(offered, _)| *offered == fourcc)?;
                    let make = || SharedBuffer::new(dmabuf, size, fourcc, modifiers, handle);
                    Some(Shared2 {
                        images: [make()?, make()?],
                        fourcc,
                        writing: 0,
                    })
                });
                self.unshared = self.shared.is_none();
            }
            if let Some(shared) = &self.shared {
                return Some(shared.images[shared.writing].buffer.clone());
            }
        }
        let format = [wl_shm::Format::Xrgb8888, wl_shm::Format::Argb8888]
            .into_iter()
            .find(|format| self.formats.contains(format))?;
        let fits = self
            .buffer
            .as_ref()
            .is_some_and(|buffer| buffer.size == size && buffer.format == format);
        if !fits {
            self.buffer = Buffer::new(shm, size, format, handle);
        }
        Some(self.buffer.as_ref()?.buffer.clone())
    }

    fn end(&mut self) {
        if let Some(frame) = self.frame.take() {
            frame.destroy();
        }
        self.buffer = None;
        self.shared = None;
        if let Some(session) = self.session.take() {
            session.destroy();
        }
        if let Some(source) = self.source.take() {
            source.destroy();
        }
        self.ready = false;
    }
}

struct Toplevel {
    handle: ExtForeignToplevelHandleV1,
    id: String,
    app: String,
    title: String,
    known: bool,
    session: Session,
    /// A capture is due as soon as one can be made.
    due: bool,
    /// Captures failed in a row, such as as the window changed size.
    retries: u32,
}

struct Capture {
    shm: WlShm,
    dmabuf: Option<ZwpLinuxDmabufV1>,
    sources: ExtForeignToplevelImageCaptureSourceManagerV1,
    copies: ExtImageCopyCaptureManagerV1,
    toplevels: Vec<Toplevel>,
    shots: Shots,
    /// What the turn so far brought.
    news: News,
    /// Someone watches, so each window is captured again as its content changes.
    live: bool,
    /// The frontend has offered to make images, so captures can be shown; those taken before
    /// are taken again.
    offered: bool,
    /// The windows kept live while watched, by [`key`]; all of them when none are named.
    wanted: Option<HashSet<String>>,
}

impl Capture {
    fn wants(&self, id: &str) -> bool {
        self.live
            && self
                .wanted
                .as_ref()
                .is_none_or(|wanted| wanted.contains(id))
    }

    fn toplevel(&mut self, handle: &ExtForeignToplevelHandleV1) -> Option<&mut Toplevel> {
        self.toplevels
            .iter_mut()
            .find(|toplevel| toplevel.handle == *handle)
    }

    fn by_key(&mut self, id: &str) -> Option<&mut Toplevel> {
        self.toplevels.iter_mut().find(|toplevel| toplevel.id == id)
    }

    /// When the next capture paced by [`LIVE_EVERY`] comes due, if one waits for that.
    fn next_due(&self) -> Option<Instant> {
        self.toplevels
            .iter()
            .filter(|toplevel| toplevel.due && toplevel.session.frame.is_none())
            .filter_map(|toplevel| toplevel.session.asked)
            .map(|asked| asked + LIVE_EVERY)
            .min()
    }

    /// Starts a capture of every window due one, no sooner than [`LIVE_EVERY`] after its last.
    fn start(&mut self, handle: &QueueHandle<Self>) {
        let now = Instant::now();
        let Self {
            toplevels,
            shm,
            dmabuf,
            sources,
            copies,
            ..
        } = self;
        for toplevel in toplevels {
            if !toplevel.known || !toplevel.due || toplevel.session.frame.is_some() {
                continue;
            }
            if toplevel
                .session
                .asked
                .is_some_and(|asked| now < asked + LIVE_EVERY)
            {
                continue;
            }
            let state = &mut toplevel.session;
            let Some(session) = state.session.clone() else {
                let source = sources.create_source(&toplevel.handle, handle, ());
                let session =
                    copies.create_session(&source, Options::empty(), handle, toplevel.id.clone());
                state.source = Some(source);
                state.session = Some(session);
                continue;
            };
            let (true, Some(size)) = (state.ready, state.size) else {
                continue;
            };
            let Some(buffer) = state.target(dmabuf.as_ref(), shm, size, handle) else {
                debug!(window = toplevel.id, "nothing to capture into");
                toplevel.due = false;
                continue;
            };
            let frame = session.create_frame(handle, toplevel.id.clone());
            frame.attach_buffer(&buffer);
            frame.damage_buffer(0, 0, size.0 as i32, size.1 as i32);
            frame.capture();
            state.frame = Some(frame);
            state.asked = Some(now);
            state.damaged = false;
            toplevel.due = false;
        }
    }

    fn taken(&mut self, id: &str) {
        let Self {
            toplevels,
            shots,
            live,
            news,
            ..
        } = self;
        let Some(toplevel) = toplevels.iter_mut().find(|toplevel| toplevel.id == id) else {
            return;
        };
        if let Some(frame) = toplevel.session.frame.take() {
            frame.destroy();
        }
        toplevel.retries = 0;
        toplevel.due = *live;
        let named = shots
            .named
            .get(&(toplevel.app.clone(), toplevel.title.clone()))
            .is_some_and(|named| named == id);
        if !toplevel.session.damaged && shots.pictures.contains_key(id) && named {
            return;
        }
        let frame = if let Some(shared) = &mut toplevel.session.shared {
            let written = &shared.images[shared.writing].shared;
            share::wait_written(&written.fd);
            let frame = Frame {
                image: written.image.clone(),
                size: written.size,
                opaque: shared.fourcc == XRGB8888,
            };
            shared.writing = 1 - shared.writing;
            frame
        } else {
            let Some(buffer) = &toplevel.session.buffer else {
                return;
            };
            let (pixels, size) = buffer.shrink();
            let Some(image) = share::upload(size, &pixels) else {
                return;
            };
            Frame {
                image,
                size,
                opaque: buffer.format == wl_shm::Format::Xrgb8888,
            }
        };
        if let Some(Picture::Live(shown)) = shots.pictures.get(id) {
            shown.show(frame);
            *news = (*news).max(News::Moved);
        } else {
            shots
                .pictures
                .insert(id.to_owned(), Picture::Live(Live::new(frame)));
            *news = News::Changed;
        }
        if !named {
            shots.named.insert(
                (toplevel.app.clone(), toplevel.title.clone()),
                id.to_owned(),
            );
            *news = News::Changed;
        }
    }

    fn failed(&mut self, id: &str) {
        let Some(toplevel) = self.by_key(id) else {
            return;
        };
        if let Some(frame) = toplevel.session.frame.take() {
            frame.destroy();
        }
        toplevel.session.buffer = None;
        toplevel.session.shared = None;
        toplevel.due = toplevel.retries < RETRIES;
        toplevel.retries += 1;
    }

    fn forget(&mut self, handle: &ExtForeignToplevelHandleV1) {
        let Some(index) = self
            .toplevels
            .iter()
            .position(|toplevel| toplevel.handle == *handle)
        else {
            return;
        };
        let mut toplevel = self.toplevels.remove(index);
        toplevel.session.end();
        toplevel.handle.destroy();
        self.shots.pictures.remove(&toplevel.id);
        self.shots.named.retain(|_, id| *id != toplevel.id);
        self.news = News::Changed;
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for Capture {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(Capture: ignore WlShm);
delegate_noop!(Capture: ignore WlShmPool);
delegate_noop!(Capture: ignore WlBuffer);
delegate_noop!(Capture: ignore ZwpLinuxDmabufV1);
delegate_noop!(Capture: ignore ZwpLinuxBufferParamsV1);
delegate_noop!(Capture: ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(Capture: ExtImageCaptureSourceV1);
delegate_noop!(Capture: ExtImageCopyCaptureManagerV1);

impl Dispatch<ExtForeignToplevelListV1, ()> for Capture {
    fn event(
        capture: &mut Self,
        _: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            capture.toplevels.push(Toplevel {
                handle: toplevel,
                id: String::new(),
                app: String::new(),
                title: String::new(),
                known: false,
                session: Session::default(),
                due: true,
                retries: 0,
            });
        }
    }

    event_created_child!(Capture, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for Capture {
    fn event(
        capture: &mut Self,
        handle: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_handle_v1::Event::Closed = event {
            capture.forget(handle);
            return;
        }
        let Some(toplevel) = capture.toplevel(handle) else {
            return;
        };
        let mut first = false;
        match event {
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                toplevel.id = key(&identifier);
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => toplevel.app = app_id,
            ext_foreign_toplevel_handle_v1::Event::Title { title } => toplevel.title = title,
            ext_foreign_toplevel_handle_v1::Event::Done => {
                first = !toplevel.known;
                toplevel.known = true;
            }
            _ => {}
        }
        if first {
            capture.news = News::Changed;
        }
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, String> for Capture {
    fn event(
        capture: &mut Self,
        _: &ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        id: &String,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(toplevel) = capture.by_key(id) else {
            return;
        };
        let session = &mut toplevel.session;
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                session.size = Some((width, height));
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => match format {
                WEnum::Value(format) => session.arriving.push(format),
                WEnum::Unknown(code) => debug!(window = id, code, "an shm format island lacks"),
            },
            ext_image_copy_capture_session_v1::Event::DmabufFormat { format, modifiers } => {
                let modifiers = modifiers
                    .chunks_exact(8)
                    .filter_map(|bytes| bytes.try_into().ok().map(u64::from_ne_bytes))
                    .collect();
                session.arriving_shared.push((format, modifiers));
            }
            ext_image_copy_capture_session_v1::Event::Done => {
                session.formats = std::mem::take(&mut session.arriving);
                session.shared_formats = std::mem::take(&mut session.arriving_shared);
                session.ready = true;
            }
            ext_image_copy_capture_session_v1::Event::Stopped => {
                debug!(window = id, "capture stopped");
                session.end();
                toplevel.due = false;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, String> for Capture {
    fn event(
        capture: &mut Self,
        _: &ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        id: &String,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_image_copy_capture_frame_v1::Event::Damage { width, height, .. }
                if width > 0 && height > 0 =>
            {
                if let Some(toplevel) = capture.by_key(id) {
                    toplevel.session.damaged = true;
                }
            }
            ext_image_copy_capture_frame_v1::Event::Ready => capture.taken(id),
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                debug!(window = id, ?reason, "a window could not be captured");
                capture.failed(id);
            }
            _ => {}
        }
    }
}

fn lost(error: &dyn std::fmt::Display) -> String {
    format!("lost the Wayland connection: {error}")
}

/// Captures windows on a Wayland connection of its own, driven by one worker: it takes a turn
/// whenever [`Capturer::wait`] returns.
pub struct Capturer {
    connection: Connection,
    queue: EventQueue<Capture>,
    handle: QueueHandle<Capture>,
    capture: Capture,
}

impl Capturer {
    pub fn connect() -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|error| format!("cannot connect to Wayland: {error}"))?;
        let (globals, queue) =
            registry_queue_init::<Capture>(&connection).map_err(|error| error.to_string())?;
        let handle = queue.handle();
        let missing = |what: &str| format!("the compositor does not offer {what}");
        let shm: WlShm = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| missing("wl_shm"))?;
        let dmabuf = globals.bind(&handle, 3..=5, ()).ok();
        let sources: ExtForeignToplevelImageCaptureSourceManagerV1 = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| missing("ext-foreign-toplevel-image-capture-source"))?;
        let copies: ExtImageCopyCaptureManagerV1 = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| missing("ext-image-copy-capture"))?;
        let _list: ExtForeignToplevelListV1 = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| missing("ext-foreign-toplevel-list"))?;
        Ok(Self {
            connection,
            queue,
            handle,
            capture: Capture {
                shm,
                dmabuf,
                sources,
                copies,
                toplevels: Vec::new(),
                shots: Shots::default(),
                news: News::Nothing,
                live: false,
                offered: false,
                wanted: None,
            },
        })
    }

    /// Watched, every window is captured now and again each time its content changes; not, each
    /// is captured only as it appears, and lets go of its images on the GPU but the one shown.
    pub fn watch(&mut self, watched: bool) {
        if watched == self.capture.live {
            return;
        }
        for toplevel in &mut self.capture.toplevels {
            if watched {
                toplevel.due = true;
                toplevel.retries = 0;
            } else if toplevel.session.frame.is_none() {
                toplevel.session.shared = None;
            }
        }
        self.capture.live = watched;
    }

    /// Only the windows of these identifiers are kept live while watched; all of them for none.
    pub fn want(&mut self, ids: Option<&[String]>) {
        let wanted: Option<HashSet<String>> = ids.map(|ids| ids.iter().map(|id| key(id)).collect());
        if wanted == self.capture.wanted {
            return;
        }
        self.capture.wanted = wanted;
        let capture = &mut self.capture;
        for at in 0..capture.toplevels.len() {
            if capture.wants(&capture.toplevels[at].id) {
                capture.toplevels[at].due = true;
            }
        }
    }

    pub fn shots(&self) -> &Shots {
        &self.capture.shots
    }

    /// The apps of the windows it knows, as the compositor names them.
    pub fn apps(&self) -> impl Iterator<Item = &str> {
        self.capture
            .toplevels
            .iter()
            .filter(|toplevel| toplevel.known)
            .map(|toplevel| toplevel.app.as_str())
    }

    /// Handles what arrived and starts the captures due.
    pub fn turn(&mut self) -> Result<News, String> {
        self.queue
            .dispatch_pending(&mut self.capture)
            .map_err(|error| lost(&error))?;
        if !self.capture.offered && share::can_make() {
            self.capture.offered = true;
            for toplevel in &mut self.capture.toplevels {
                toplevel.due = true;
            }
        }
        self.capture.start(&self.handle);
        self.connection.flush().map_err(|error| lost(&error))?;
        let news = std::mem::replace(&mut self.capture.news, News::Nothing);
        Ok(news)
    }

    /// Sleeps until the compositor sends something, `wake` can be read or a paced capture comes
    /// due; whether `wake` can.
    pub fn wait(&mut self, wake: RawFd) -> Result<bool, String> {
        let Some(read) = self.queue.prepare_read() else {
            return Ok(false);
        };
        wake::wait(read, wake, self.capture.next_due()).map_err(|error| lost(&error))
    }
}

#[cfg(test)]
#[path = "capture_test.rs"]
mod tests;
