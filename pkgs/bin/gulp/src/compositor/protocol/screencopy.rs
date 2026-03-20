//! Wayland screencopy protocol handling

use anyhow::{Context, Result};
use wayland_client::{
    globals::GlobalListContents,
    protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool},
    Connection, Dispatch, QueueHandle,
    delegate_noop,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::capture::buffer::CapturedImage;
use super::shm::{create_shm_fd, read_shm_buffer};

// Capture timing constants
const MAX_CAPTURE_ATTEMPTS: u32 = 100;

/// Internal state for tracking screencopy events
pub(super) struct CaptureState {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub stride: Option<u32>,
    pub format: Option<wl_shm::Format>,
    pub ready: bool,
    pub failed: bool,
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
                "Screen capture timed out after {} attempts",
                MAX_CAPTURE_ATTEMPTS
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
                log::debug!("Buffer: {}x{}, stride: {}, format: {:?}", width, height, stride, format);
                state.width = Some(width);
                state.height = Some(height);
                state.stride = Some(stride);
                if let WEnum::Value(fmt) = format {
                    state.format = Some(fmt);
                }
            }
            zwlr_screencopy_frame_v1::Event::Flags { .. } => {
                log::debug!("Frame flags received");
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

/// Captures the entire content of a Wayland output using the zwlr-screencopy-v1 protocol.
pub fn capture_output(conn: &Connection, output: &wl_output::WlOutput) -> Result<CapturedImage> {
    let mut event_queue = conn.new_event_queue::<CaptureState>();
    let qh = event_queue.handle();

    // Bind protocols
    let (screencopy_manager, shm) = bind_protocols(conn, &qh)?;

    // Initialize capture
    let mut capture_state = CaptureState::new();
    let frame: ZwlrScreencopyFrameV1 = screencopy_manager.capture_output(0, output, &qh, ());

    // Get buffer info
    event_queue.roundtrip(&mut capture_state)?;

    let width = capture_state.width.context("No buffer width received")?;
    let height = capture_state.height.context("No buffer height received")?;
    let stride = capture_state.stride.context("No stride received")?;
    let format = capture_state.format.unwrap_or(wl_shm::Format::Argb8888);

    log::debug!("Capture buffer: {}x{}, stride: {}, format: {:?}", width, height, stride, format);

    // Create and attach buffer
    let size = (stride * height) as usize;
    let (buffer, pool, shm_fd) = create_wl_buffer(&shm, &qh, width, height, stride, format, size)?;

    // Start capture
    frame.copy(&buffer);

    // Wait for completion
    wait_for_capture(&mut event_queue, &mut capture_state)?;

    // Read the captured data BEFORE cleanup
    let data = read_shm_buffer(shm_fd, size)?;

    // Cleanup (order matters - buffer before pool)
    buffer.destroy();
    pool.destroy();
    frame.destroy();

    Ok(CapturedImage::new(data, width, height, stride, format))
}

/// Binds the required Wayland protocols for screen capture.
fn bind_protocols(
    conn: &Connection,
    qh: &QueueHandle<CaptureState>,
) -> Result<(ZwlrScreencopyManagerV1, wl_shm::WlShm)> {
    use wayland_client::globals::registry_queue_init;
    let (globals, _) = registry_queue_init::<CaptureState>(conn)
        .context("Failed to init registry")?;

    let screencopy_manager = globals
        .bind(qh, 1..=3, ())
        .context("zwlr_screencopy_manager_v1 not available")?;

    let shm = globals
        .bind(qh, 1..=1, ())
        .context("wl_shm not available")?;

    Ok((screencopy_manager, shm))
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
) -> Result<(wl_buffer::WlBuffer, wl_shm_pool::WlShmPool, i32)> {
    use std::os::fd::BorrowedFd;

    let shm_fd = create_shm_fd(size)?;
    // SAFETY: shm_fd was just created by create_shm_fd and is a valid file descriptor.
    // BorrowedFd does not take ownership, and shm_fd remains valid throughout this function.
    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(shm_fd) };

    let pool = shm.create_pool(borrowed_fd, size as i32, qh, ());
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
    const MAX_ATTEMPTS: u32 = MAX_CAPTURE_ATTEMPTS;

    for attempt in 0..MAX_ATTEMPTS {
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
