//! Wayland screencopy protocol handling

use anyhow::{Context, Result};
use std::os::fd::OwnedFd;
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    globals::GlobalListContents,
    protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool},
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use super::shm::{create_shm_fd, map_shm_buffer};
use crate::capture::buffer::{CapturedImage, PixelFormat};
use crate::render::Rect;

// How many times to poll the compositor for the frame before giving up.
const MAX_CAPTURE_POLLS: u32 = 100;

/// Internal state for tracking screencopy events
pub(super) struct CaptureState {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub stride: Option<u32>,
    pub format: Option<wl_shm::Format>,
    pub ready: bool,
    pub failed: bool,
    pub y_inverted: bool,
}

impl CaptureState {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            stride: None,
            format: None,
            ready: false,
            failed: false,
            y_inverted: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.ready || self.failed
    }

    pub fn to_result(&self) -> Result<()> {
        if self.failed {
            anyhow::bail!("Screen capture failed - compositor rejected the capture request");
        }
        if !self.ready {
            anyhow::bail!(
                "The compositor never reported the capture as ready, after {} polls",
                MAX_CAPTURE_POLLS
            );
        }
        Ok(())
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use wayland_client::WEnum;
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                log::debug!(
                    "Buffer: {}x{}, stride: {}, format: {:?}",
                    width,
                    height,
                    stride,
                    format
                );
                state.width = Some(width);
                state.height = Some(height);
                state.stride = Some(stride);
                if let WEnum::Value(fmt) = format {
                    state.format = Some(fmt);
                }
            }
            zwlr_screencopy_frame_v1::Event::Flags {
                flags: WEnum::Value(flags),
            } => {
                state.y_inverted = flags.contains(zwlr_screencopy_frame_v1::Flags::YInvert);
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                log::info!("Frame ready");
                state.ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                log::error!("Capture failed");
                state.failed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

// Delegate no-op for basic Wayland types
delegate_noop!(CaptureState: ignore wl_registry::WlRegistry);
delegate_noop!(CaptureState: ignore wl_shm::WlShm);
delegate_noop!(CaptureState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(CaptureState: ignore wl_buffer::WlBuffer);
delegate_noop!(CaptureState: ignore ZwlrScreencopyManagerV1);

/// A bound screencopy interface, reusable for any number of captures.
///
/// Binding the globals is a full registry enumeration and a roundtrip, so it is
/// done once per connection rather than once per captured output.
pub struct Screencopy {
    event_queue: wayland_client::EventQueue<CaptureState>,
    manager: ZwlrScreencopyManagerV1,
    shm: wl_shm::WlShm,
}

impl Screencopy {
    /// Binds the screencopy and shm globals on `conn`.
    pub fn new(conn: &Connection) -> Result<Self> {
        use wayland_client::globals::registry_queue_init;

        let (globals, event_queue) =
            registry_queue_init::<CaptureState>(conn).context("Failed to init registry")?;
        let qh = event_queue.handle();

        let manager = globals
            .bind(&qh, 1..=3, ())
            .context("zwlr_screencopy_manager_v1 not available")?;

        let shm = globals
            .bind(&qh, 1..=1, ())
            .context("wl_shm not available")?;

        Ok(Self {
            event_queue,
            manager,
            shm,
        })
    }

    pub fn capture(
        &mut self,
        output: &wl_output::WlOutput,
        transform: wl_output::Transform,
    ) -> Result<CapturedImage> {
        let qh = self.event_queue.handle();
        let frame = self.manager.capture_output(0, output, &qh, ());
        capture_frame(self, frame, transform)
    }

    pub fn capture_region(
        &mut self,
        output: &wl_output::WlOutput,
        rect: Rect,
        transform: wl_output::Transform,
    ) -> Result<CapturedImage> {
        let qh = self.event_queue.handle();
        let frame = self.manager.capture_output_region(
            0,
            output,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            &qh,
            (),
        );
        capture_frame(self, frame, transform)
    }
}

fn capture_frame(
    screencopy: &mut Screencopy,
    frame: ZwlrScreencopyFrameV1,
    transform: wl_output::Transform,
) -> Result<CapturedImage> {
    let event_queue = &mut screencopy.event_queue;
    let qh = event_queue.handle();
    let mut capture_state = CaptureState::new();
    event_queue.roundtrip(&mut capture_state)?;

    let width = capture_state.width.context("No buffer width received")?;
    let height = capture_state.height.context("No buffer height received")?;
    let stride = capture_state.stride.context("No stride received")?;
    let shm_format = capture_state
        .format
        .context("No supported pixel format received")?;
    let format = match shm_format {
        wl_shm::Format::Argb8888 => PixelFormat::Argb8888,
        wl_shm::Format::Xrgb8888 => PixelFormat::Xrgb8888,
        other => anyhow::bail!("Unsupported screencopy pixel format: {:?}", other),
    };

    let size = (stride * height) as usize;
    let (buffer, pool, shm_fd) = create_wl_buffer(
        &screencopy.shm,
        &qh,
        width,
        height,
        stride,
        shm_format,
        size,
    )?;
    frame.copy(&buffer);
    wait_for_capture(event_queue, &mut capture_state)?;
    let data = map_shm_buffer(&shm_fd, size)?;

    buffer.destroy();
    pool.destroy();
    frame.destroy();

    CapturedImage::new(data, width, height, stride, format)?
        .normalize(transform, capture_state.y_inverted)
}

/// Creates a Wayland buffer backed by shared memory.
fn create_wl_buffer(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<CaptureState>,
    width: u32,
    height: u32,
    stride: u32,
    format: wl_shm::Format,
    size: usize,
) -> Result<(wl_buffer::WlBuffer, wl_shm_pool::WlShmPool, OwnedFd)> {
    use std::os::fd::AsFd;

    let shm_fd = create_shm_fd(size)?;

    let pool = shm.create_pool(shm_fd.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        format,
        qh,
        (),
    );

    Ok((buffer, pool, shm_fd))
}

/// Waits for the screen capture operation to complete or timeout.
fn wait_for_capture(
    event_queue: &mut wayland_client::EventQueue<CaptureState>,
    capture_state: &mut CaptureState,
) -> Result<()> {
    for attempt in 0..MAX_CAPTURE_POLLS {
        if capture_state.is_complete() {
            return capture_state.to_result();
        }

        event_queue.roundtrip(capture_state)?;

        // Only sleep on subsequent attempts after the first few, with minimal delay
        if !capture_state.is_complete() && attempt > 5 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    capture_state.to_result()
}
