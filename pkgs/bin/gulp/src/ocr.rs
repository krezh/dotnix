//! OCR functionality using Wayland screencopy protocols and Tesseract

use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba};
use wayland_client::{
    protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool},
    Connection, Dispatch, QueueHandle,
    delegate_noop,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::selection::Rect;

/// Represents captured image data with metadata
pub struct CapturedImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: wl_shm::Format,
}

impl CapturedImage {
    /// Crop the image to a specific region
    pub fn crop(&self, rect: Rect) -> Result<CapturedImage> {
        let rect_width = rect.width.min((self.width as i32) - rect.x) as u32;
        let rect_height = rect.height.min((self.height as i32) - rect.y) as u32;

        log::debug!(
            "Cropping {}x{} region from {}x{} image (stride: {})",
            rect_width, rect_height, self.width, self.height, self.stride
        );

        let expected_size = (rect_width * rect_height * 4) as usize;
        let mut cropped_data = Vec::with_capacity(expected_size);

        for y in 0..rect_height {
            let src_y = (rect.y as u32 + y).min(self.height - 1);
            let src_offset = (src_y * self.stride + rect.x as u32 * 4) as usize;
            let row_size = (rect_width * 4) as usize;

            if src_offset + row_size <= self.data.len() {
                cropped_data.extend_from_slice(&self.data[src_offset..src_offset + row_size]);
            } else {
                log::warn!(
                    "Row {} out of bounds: offset={}, row_size={}, buffer_len={}",
                    y, src_offset, row_size, self.data.len()
                );
                // Pad with zeros for incomplete rows
                cropped_data.resize(cropped_data.len() + row_size, 0);
            }
        }

        log::debug!("Cropped buffer size: {}, expected: {}", cropped_data.len(), expected_size);

        Ok(CapturedImage {
            data: cropped_data,
            width: rect_width,
            height: rect_height,
            stride: rect_width * 4,
            format: self.format,
        })
    }

    /// Perform OCR on this image
    pub fn ocr(&self) -> Result<String> {
        log::info!("Running OCR on {}x{} image", self.width, self.height);

        // Convert ARGB to RGBA for image library
        let rgba_buffer = convert_argb_to_rgba(&self.data);

        let expected_size = (self.width * self.height * 4) as usize;
        log::debug!("Buffer size: {}, expected: {}", rgba_buffer.len(), expected_size);

        if rgba_buffer.len() != expected_size {
            anyhow::bail!(
                "Buffer size mismatch: got {} bytes, expected {} bytes ({}x{} * 4)",
                rgba_buffer.len(),
                expected_size,
                self.width,
                self.height
            );
        }

        let img = ImageBuffer::<Rgba<u8>, _>::from_raw(self.width, self.height, rgba_buffer)
            .context("Failed to create image from buffer")?;

        // Save to temporary file for Tesseract
        let temp_path = "/tmp/gulp_ocr.png";
        img.save(temp_path)?;

        let mut tess = tesseract::Tesseract::new(None, Some("eng"))?;
        tess = tess.set_image(temp_path)?;
        let text = tess.get_text()?.trim().to_string();

        let _ = std::fs::remove_file(temp_path);

        log::info!("OCR completed, extracted {} characters", text.len());

        Ok(text)
    }
}

/// Internal state for tracking screencopy events
struct CaptureState {
    width: Option<u32>,
    height: Option<u32>,
    stride: Option<u32>,
    format: Option<wl_shm::Format>,
    ready: bool,
    failed: bool,
}

impl CaptureState {
    fn new() -> Self {
        Self {
            width: None,
            height: None,
            stride: None,
            format: None,
            ready: false,
            failed: false,
        }
    }

    fn is_complete(&self) -> bool {
        self.ready || self.failed
    }

    fn to_result(&self) -> Result<()> {
        if self.failed {
            anyhow::bail!("Screen capture failed");
        }
        if !self.ready {
            anyhow::bail!("Screen capture timed out");
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

// Implement Dispatch for GlobalListContents
use wayland_client::globals::GlobalListContents;

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

/// Main entry point: captures a screen region and performs OCR
pub fn capture_and_ocr(
    conn: &Connection,
    outputs: &[(wl_output::WlOutput, String, i32, i32, u32, u32)],
    rect: Rect,
) -> Result<String> {
    log::info!(
        "Capturing region: {}x{} at ({},{})",
        rect.width,
        rect.height,
        rect.x,
        rect.y
    );

    // Find which output contains the selection
    let (output, _name, offset_x, offset_y, width, height) = outputs
        .iter()
        .find(|(_, _, x, y, w, h)| {
            let output_rect = Rect::new(*x, *y, *w as i32, *h as i32);
            rect.intersects(&output_rect)
        })
        .context("Selection is not on any output")?;

    log::info!(
        "Selection is on output at ({},{}) {}x{}",
        offset_x, offset_y, width, height
    );

    // Capture the output
    let captured = capture_output(conn, output)?;

    // Translate global coordinates to local coordinates
    let local_rect = Rect::new(
        rect.x - offset_x,
        rect.y - offset_y,
        rect.width,
        rect.height,
    );

    log::info!(
        "Translated to local coordinates: {}x{} at ({},{})",
        local_rect.width,
        local_rect.height,
        local_rect.x,
        local_rect.y
    );

    // Crop to the selected region
    let cropped = captured.crop(local_rect)?;

    // Perform OCR
    cropped.ocr()
}

/// Captures an entire output using zwlr-screencopy-v1 protocol
fn capture_output(conn: &Connection, output: &wl_output::WlOutput) -> Result<CapturedImage> {
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

    Ok(CapturedImage {
        data,
        width,
        height,
        stride,
        format,
    })
}

/// Binds required Wayland protocols
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

/// Creates a Wayland buffer backed by shared memory
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

/// Waits for the capture to complete
fn wait_for_capture(
    event_queue: &mut wayland_client::EventQueue<CaptureState>,
    capture_state: &mut CaptureState,
) -> Result<()> {
    const MAX_ATTEMPTS: u32 = 100;
    const SLEEP_MS: u64 = 10;

    for _ in 0..MAX_ATTEMPTS {
        if capture_state.is_complete() {
            return capture_state.to_result();
        }
        event_queue.roundtrip(capture_state)?;
        std::thread::sleep(std::time::Duration::from_millis(SLEEP_MS));
    }

    capture_state.to_result()
}

/// Creates a sealed shared memory file descriptor
fn create_shm_fd(size: usize) -> Result<i32> {
    use nix::fcntl::{FcntlArg, SealFlag};
    use nix::sys::memfd::{memfd_create, MFdFlags};
    use nix::unistd::ftruncate;
    use std::ffi::CStr;
    use std::os::fd::IntoRawFd;

    let name = CStr::from_bytes_with_nul(b"gulp-capture\0")?;
    let fd = memfd_create(
        name,
        MFdFlags::MFD_CLOEXEC | MFdFlags::MFD_ALLOW_SEALING,
    )?;

    ftruncate(&fd, size as i64)?;

    nix::fcntl::fcntl(
        &fd,
        FcntlArg::F_ADD_SEALS(
            SealFlag::F_SEAL_SHRINK | SealFlag::F_SEAL_GROW | SealFlag::F_SEAL_SEAL,
        ),
    )?;

    // Use into_raw_fd() to transfer ownership and prevent auto-close
    Ok(fd.into_raw_fd())
}

/// Reads data from a shared memory file descriptor
fn read_shm_buffer(fd: i32, size: usize) -> Result<Vec<u8>> {
    use nix::unistd::{lseek, read, Whence};
    use std::os::fd::BorrowedFd;

    let mut buffer = vec![0u8; size];
    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };

    // Seek to beginning
    lseek(&borrowed_fd, 0, Whence::SeekSet)?;

    let mut total_read = 0;
    while total_read < size {
        match read(&borrowed_fd, &mut buffer[total_read..]) {
            Ok(0) => break, // EOF
            Ok(n) => total_read += n,
            Err(e) => return Err(anyhow::anyhow!("Failed to read from shm fd: {}", e)),
        }
    }

    if total_read != size {
        anyhow::bail!("Incomplete read: got {} bytes, expected {}", total_read, size);
    }

    Ok(buffer)
}

/// Converts ARGB8888 pixel data to RGBA8888
fn convert_argb_to_rgba(buffer: &[u8]) -> Vec<u8> {
    let mut rgba_buffer = Vec::with_capacity(buffer.len());
    for chunk in buffer.chunks_exact(4) {
        let b = chunk[0];
        let g = chunk[1];
        let r = chunk[2];
        let a = chunk[3];
        rgba_buffer.extend_from_slice(&[r, g, b, a]);
    }
    rgba_buffer
}
