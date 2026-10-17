use std::{
    collections::HashMap,
    fs::File,
    io::{self, ErrorKind, Read, Write},
    os::{
        fd::{AsFd, AsRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use tracing::{debug, info, warn};
use wayland_backend::client::{ObjectId, WaylandError};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry::WlRegistry, wl_seat::WlSeat},
};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};

use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

use super::{
    Kind, Request, Settings, State, files,
    paste::{Keyboard, Shortcut},
    store::{Clip, Store},
};
use crate::{compositor, features::Feed, supervisor::Ended};

/// Offered by island's own selections, so it does not copy them back in.
const MARKER: &str = "application/x-island-clipboard";
/// Password managers mark their secrets with this; they are never kept.
const SECRET: &str = "x-kde-passwordManagerHint";
const URI_LIST: &str = "text/uri-list";
/// Text types, preferred first.
const TEXT: [&str; 5] = ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING", "STRING", "TEXT"];
/// How long an app gets to hand over what it copied.
const TRANSFER: Duration = Duration::from_secs(3);
/// How long after picking an entry the paste shortcut is pressed, so the island has let go of the
/// keyboard and the window it was over has it again.
const PASTE_DELAY: Duration = Duration::from_millis(150);

/// Tells published states apart across restarts of the worker.
static VERSION: AtomicU64 = AtomicU64::new(0);

struct Served {
    source: ExtDataControlSourceV1,
    data: Vec<(String, Arc<Vec<u8>>)>,
}

#[derive(Default)]
struct Watch {
    offers: HashMap<ObjectId, Vec<String>>,
    /// `Some(None)` when the clipboard emptied.
    selection: Option<Option<ExtDataControlOfferV1>>,
    served: Option<Served>,
    finished: bool,
    paste_at: Option<Instant>,
    /// Made on the first paste.
    keyboard: Option<Keyboard>,
}

impl Dispatch<WlRegistry, GlobalListContents> for Watch {
    fn event(_: &mut Self, _: &WlRegistry, _: <WlRegistry as Proxy>::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

delegate_noop!(Watch: ignore WlSeat);
delegate_noop!(Watch: ExtDataControlManagerV1);
delegate_noop!(Watch: ZwpVirtualKeyboardManagerV1);
delegate_noop!(Watch: ZwpVirtualKeyboardV1);

impl Dispatch<ExtDataControlDeviceV1, ()> for Watch {
    fn event(
        watch: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                watch.offers.insert(id.id(), Vec::new());
            }
            ext_data_control_device_v1::Event::Selection { id } => {
                if let Some(Some(skipped)) = watch.selection.replace(id) {
                    watch.offers.remove(&skipped.id());
                    skipped.destroy();
                }
            }
            ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                watch.offers.remove(&offer.id());
                offer.destroy();
            }
            ext_data_control_device_v1::Event::Finished => watch.finished = true,
            _ => {}
        }
    }

    event_created_child!(Watch, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ExtDataControlOfferV1, ()> for Watch {
    fn event(
        watch: &mut Self,
        offer: &ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_data_control_offer_v1::Event::Offer { mime_type } = event {
            watch.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl Dispatch<ExtDataControlSourceV1, ()> for Watch {
    fn event(
        watch: &mut Self,
        source: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let current = watch.served.as_ref().filter(|served| served.source == *source);
        match event {
            ext_data_control_source_v1::Event::Send { mime_type, fd } => {
                let data = current.and_then(|served| served.data.iter().find(|(mime, _)| *mime == mime_type));
                if let Some((_, data)) = data {
                    send(fd, data.clone());
                }
            }
            ext_data_control_source_v1::Event::Cancelled => {
                if current.is_some() {
                    watch.served = None;
                }
                source.destroy();
            }
            _ => {}
        }
    }
}

/// Off the worker, so a slow reader cannot hold it up.
fn send(fd: OwnedFd, data: Arc<Vec<u8>>) {
    thread::spawn(move || {
        if let Err(error) = File::from(fd).write_all(&data) {
            debug!("pasting stopped early: {error}");
        }
    });
}

fn readable(fd: &impl AsRawFd, timeout: Duration) -> bool {
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let millis = libc::c_int::try_from(timeout.as_millis()).unwrap_or(libc::c_int::MAX);
    (unsafe { libc::poll(&mut poll, 1, millis) }) > 0
}

/// `None` when it is larger than `limit` or the app takes too long.
fn receive(connection: &Connection, offer: &ExtDataControlOfferV1, mime: &str, limit: usize) -> io::Result<Option<Vec<u8>>> {
    let (mut reader, writer) = io::pipe()?;
    offer.receive(mime.to_owned(), writer.as_fd());
    drop(writer);
    connection.flush().map_err(io::Error::other)?;
    let deadline = Instant::now() + TRANSFER;
    let mut data = Vec::new();
    let mut chunk = [0; 64 * 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || !readable(&reader, left) {
            debug!(mime, "the app took too long to hand over its copy");
            return Ok(None);
        }
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(Some(data)),
            Ok(read) => {
                data.extend_from_slice(&chunk[..read]);
                if data.len() > limit {
                    debug!(mime, "the copy is larger than max_item_mb");
                    return Ok(None);
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// In order of preference; empty for island's own selections and password managers' secrets.
fn wanted(mimes: &[String], images: bool) -> Vec<(Kind, String)> {
    let has = |wanted: &str| mimes.iter().any(|mime| mime == wanted);
    if has(MARKER) || has(SECRET) {
        return Vec::new();
    }
    let mut wanted = Vec::new();
    if has(URI_LIST) {
        wanted.push((Kind::Files, URI_LIST.to_owned()));
    }
    if let Some(text) = TEXT.into_iter().find(|text| has(text)) {
        wanted.push((Kind::Text, text.to_owned()));
    }
    if images {
        let picture = mimes
            .iter()
            .find(|mime| *mime == "image/png")
            .or_else(|| mimes.iter().find(|mime| mime.starts_with("image/") && !mime.starts_with("image/svg")));
        if let Some(picture) = picture {
            wanted.push((Kind::Image, picture.clone()));
        }
    }
    wanted
}

/// The first of `wanted` that is useful: local files, text that is not blank, or a picture.
fn take(connection: &Connection, offer: &ExtDataControlOfferV1, wanted: Vec<(Kind, String)>, limit: usize) -> io::Result<Option<Clip>> {
    for (kind, mime) in wanted {
        let Some(data) = receive(connection, offer, &mime, limit)? else {
            continue;
        };
        let useful = match kind {
            Kind::Files => !files(&data).is_empty(),
            Kind::Text => std::str::from_utf8(&data).is_ok_and(|text| !text.trim().is_empty()),
            Kind::Image => !data.is_empty(),
        };
        if useful {
            return Ok(Some(Clip { kind, mime, data }));
        }
    }
    Ok(None)
}

fn offered(clip: Clip) -> Vec<(String, Arc<Vec<u8>>)> {
    match clip.kind {
        Kind::Text => {
            let data = Arc::new(clip.data);
            TEXT.into_iter().map(|mime| (mime.to_owned(), data.clone())).collect()
        }
        Kind::Image => vec![(clip.mime, Arc::new(clip.data))],
        Kind::Files => {
            let paths: Vec<String> = files(&clip.data).iter().map(|path| path.display().to_string()).collect();
            let uris = String::from_utf8_lossy(&clip.data).into_owned();
            let text = Arc::new(paths.join("\n").into_bytes());
            vec![
                (URI_LIST.to_owned(), Arc::new(clip.data)),
                ("x-special/gnome-copied-files".to_owned(), Arc::new(format!("copy\n{}", uris.trim_end()).into_bytes())),
                (TEXT[0].to_owned(), text.clone()),
                (TEXT[1].to_owned(), text),
            ]
        }
    }
}

struct Worker<'a> {
    connection: Connection,
    queue_handle: QueueHandle<Watch>,
    manager: ExtDataControlManagerV1,
    device: ExtDataControlDeviceV1,
    seat: WlSeat,
    keyboards: Option<ZwpVirtualKeyboardManagerV1>,
    store: Store,
    feed: &'a Feed<State>,
    settings: &'a Settings,
}

impl Worker<'_> {
    fn publish(&self, error: Option<String>) -> bool {
        let entries = match self.store.list() {
            Ok(entries) => entries,
            Err(error) => {
                warn!("cannot read the clipboard history: {error}");
                Vec::new()
            }
        };
        self.feed.publish(State {
            entries: Arc::new(entries),
            error,
            version: VERSION.fetch_add(1, Ordering::Relaxed) + 1,
        })
    }

    /// Also moves the entry to the top.
    fn serve(&self, watch: &mut Watch, id: i64) {
        let clip = match self.store.get(id) {
            Ok(Some(clip)) => clip,
            Ok(None) => return,
            Err(error) => return warn!("cannot read clipboard entry {id}: {error}"),
        };
        let source = self.manager.create_data_source(&self.queue_handle, ());
        let data = offered(clip);
        for (mime, _) in &data {
            source.offer(mime.clone());
        }
        source.offer(MARKER.to_owned());
        self.device.set_selection(Some(&source));
        if let Some(previous) = watch.served.replace(Served { source, data }) {
            previous.source.destroy();
        }
        if let Err(error) = self.store.touch(id) {
            warn!("cannot update clipboard entry {id}: {error}");
        }
        debug!(id, "serving a clipboard entry");
    }

    /// An emptied clipboard gets the newest entry back with `persist`.
    fn selection(&self, watch: &mut Watch, offer: Option<ExtDataControlOfferV1>) -> io::Result<bool> {
        let Some(offer) = offer else {
            if self.settings.persist && watch.served.is_none() {
                if let Ok(Some(newest)) = self.store.newest() {
                    debug!("the clipboard emptied; handing the newest entry back");
                    self.serve(watch, newest);
                }
            }
            return Ok(false);
        };
        let mimes = watch.offers.remove(&offer.id()).unwrap_or_default();
        let wanted = wanted(&mimes, self.settings.images);
        let limit = usize::try_from(self.settings.max_item_mb.saturating_mul(1024 * 1024)).unwrap_or(usize::MAX);
        let clip = if wanted.is_empty() { None } else { take(&self.connection, &offer, wanted, limit)? };
        offer.destroy();
        let Some(clip) = clip else {
            return Ok(false);
        };
        if let Err(error) = self.store.insert(&clip).and_then(|()| self.store.prune(self.settings)) {
            warn!("cannot keep the copy: {error}");
            return Ok(false);
        }
        debug!(kind = clip.kind.word(), mime = clip.mime, bytes = clip.data.len(), "kept a copy");
        Ok(true)
    }

    fn paste(&self, watch: &mut Watch) {
        let Some(keyboards) = &self.keyboards else {
            return warn!("cannot paste: the compositor offers no virtual keyboards");
        };
        let class = compositor::backend().and_then(|backend| backend.focused_class());
        let terminal = class
            .as_deref()
            .is_some_and(|class| self.settings.terminals.iter().any(|terminal| terminal.eq_ignore_ascii_case(class)));
        let keys = if terminal { &self.settings.terminal_paste } else { &self.settings.paste };
        let Ok(Some(shortcut)) = Shortcut::parse(keys) else {
            return;
        };
        if watch.keyboard.is_none() {
            match Keyboard::new(keyboards, &self.seat, &self.queue_handle) {
                Ok(keyboard) => watch.keyboard = Some(keyboard),
                Err(error) => return warn!("cannot make a virtual keyboard to paste with: {error}"),
            }
        }
        if let Some(keyboard) = &watch.keyboard {
            debug!(?class, keys, "pasting");
            keyboard.press(&shortcut);
        }
    }

    fn request(&self, watch: &mut Watch, request: Request) -> rusqlite::Result<()> {
        match request {
            Request::Select { id, paste } => {
                self.serve(watch, id);
                if paste {
                    watch.paste_at = Some(Instant::now() + PASTE_DELAY);
                }
            }
            Request::Delete(id) => self.store.delete(id)?,
            Request::Clear => {
                self.store.clear()?;
                info!("cleared the clipboard history");
            }
        }
        Ok(())
    }
}

fn drain(wake: &UnixStream) {
    let mut buffer = [0; 64];
    while matches!((&*wake).read(&mut buffer), Ok(read) if read > 0) {}
}

fn connect<'a>(feed: &'a Feed<State>, settings: &'a Settings) -> Result<(Worker<'a>, wayland_client::EventQueue<Watch>), String> {
    let connection = Connection::connect_to_env().map_err(|error| format!("cannot connect to Wayland: {error}"))?;
    let (globals, queue) = registry_queue_init::<Watch>(&connection).map_err(|error| error.to_string())?;
    let queue_handle = queue.handle();
    let manager: ExtDataControlManagerV1 = globals
        .bind(&queue_handle, 1..=1, ())
        .map_err(|_| "the compositor does not offer ext-data-control".to_owned())?;
    let seat: WlSeat = globals.bind(&queue_handle, 1..=7, ()).map_err(|_| "the compositor has no seat".to_owned())?;
    let device = manager.get_data_device(&seat, &queue_handle, ());
    let keyboards = globals.bind(&queue_handle, 1..=1, ()).ok();
    let store = Store::open()?;
    let worker = Worker {
        connection,
        queue_handle,
        manager,
        device,
        seat,
        keyboards,
        store,
        feed,
        settings,
    };
    Ok((worker, queue))
}

pub fn run(feed: &Feed<State>, requests: &Receiver<Request>, wake: &UnixStream, settings: &Settings) -> Ended {
    let (worker, mut queue) = match connect(feed, settings) {
        Ok(connected) => connected,
        Err(error) => {
            feed.publish(State {
                error: Some(error.clone()),
                version: VERSION.fetch_add(1, Ordering::Relaxed) + 1,
                ..State::default()
            });
            return Ended::Failed(error);
        }
    };
    info!("watching the clipboard");
    if !worker.publish(None) {
        return Ended::Unwanted;
    }
    let mut watch = Watch::default();
    loop {
        if let Err(error) = queue.dispatch_pending(&mut watch) {
            return Ended::Failed(error.to_string());
        }
        if watch.finished {
            return Ended::Failed("the compositor ended clipboard access".to_owned());
        }
        let mut changed = false;
        if let Some(offer) = watch.selection.take() {
            match worker.selection(&mut watch, offer) {
                Ok(kept) => changed |= kept,
                Err(error) => return Ended::Failed(error.to_string()),
            }
        }
        loop {
            match requests.try_recv() {
                Ok(request) => {
                    if let Err(error) = worker.request(&mut watch, request) {
                        warn!("cannot change the clipboard history: {error}");
                    }
                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ended::Unwanted,
            }
        }
        if changed && !worker.publish(None) {
            return Ended::Unwanted;
        }
        if watch.paste_at.is_some_and(|at| at <= Instant::now()) {
            watch.paste_at = None;
            worker.paste(&mut watch);
        }
        if let Err(error) = worker.connection.flush() {
            return Ended::Failed(error.to_string());
        }
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let mut polls = [
            libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: wake.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let timeout = watch.paste_at.map_or(-1, |at| {
            let left = at.saturating_duration_since(Instant::now()).as_millis();
            libc::c_int::try_from(left).unwrap_or(libc::c_int::MAX)
        });
        let ready = unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as libc::nfds_t, timeout) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() != ErrorKind::Interrupted {
                return Ended::Failed(error.to_string());
            }
            continue;
        }
        if polls[1].revents != 0 {
            drain(wake);
        }
        if polls[0].revents != 0 {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => return Ended::Failed(error.to_string()),
            }
        }
    }
}
