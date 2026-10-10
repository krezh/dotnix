use anyhow::{Context, Result};
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop,
    globals::GlobalListContents,
    protocol::{wl_buffer, wl_output, wl_registry},
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::render::Rect;
use crate::replay::hardware::HardwareFrame;

struct CaptureState {
    width: Option<u32>,
    height: Option<u32>,
    format: Option<u32>,
    offered: bool,
    ready: bool,
    failed: bool,
    y_inverted: bool,
    timestamp_micros: Option<i64>,
}

impl CaptureState {
    fn new() -> Self {
        Self {
            width: None,
            height: None,
            format: None,
            offered: false,
            ready: false,
            failed: false,
            y_inverted: false,
            timestamp_micros: None,
        }
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
        match event {
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                state.format = Some(format);
                state.width = Some(width);
                state.height = Some(height);
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => state.offered = true,
            zwlr_screencopy_frame_v1::Event::Flags {
                flags: WEnum::Value(flags),
            } => {
                state.y_inverted = flags.contains(zwlr_screencopy_frame_v1::Flags::YInvert);
            }
            zwlr_screencopy_frame_v1::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let seconds = (i64::from(tv_sec_hi) << 32) | i64::from(tv_sec_lo);
                state.timestamp_micros = Some(seconds * 1_000_000 + i64::from(tv_nsec) / 1_000);
                state.ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => state.failed = true,
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

impl Dispatch<ZwpLinuxDmabufV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpLinuxDmabufV1,
        _event: zwp_linux_dmabuf_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpLinuxBufferParamsV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpLinuxBufferParamsV1,
        _event: zwp_linux_buffer_params_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(CaptureState: ignore wl_buffer::WlBuffer);
delegate_noop!(CaptureState: ignore ZwlrScreencopyManagerV1);

pub struct VideoCapture {
    event_queue: wayland_client::EventQueue<CaptureState>,
    manager: ZwlrScreencopyManagerV1,
    dmabuf: ZwpLinuxDmabufV1,
}

impl VideoCapture {
    pub fn new(connection: &Connection) -> Result<Self> {
        use wayland_client::globals::registry_queue_init;

        let (globals, mut event_queue) =
            registry_queue_init::<CaptureState>(connection).context("Failed to init registry")?;
        let qh = event_queue.handle();
        let manager = globals
            .bind(&qh, 3..=3, ())
            .context("zwlr_screencopy_manager_v1 version 3 is unavailable")?;
        let dmabuf = globals
            .bind(&qh, 3..=5, ())
            .context("zwp_linux_dmabuf_v1 is unavailable")?;
        let mut state = CaptureState::new();
        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;
        Ok(Self {
            event_queue,
            manager,
            dmabuf,
        })
    }

    pub fn probe_region(
        &mut self,
        output: &wl_output::WlOutput,
        rect: Rect,
    ) -> Result<(u32, u32, u32)> {
        let (frame, state) = self.request_region(output, rect)?;
        let result = Ok((
            state.width.context("No capture width received")?,
            state.height.context("No capture height received")?,
            state
                .format
                .context("The compositor did not offer DMA-BUF screencopy")?,
        ));
        frame.destroy();
        result
    }

    pub fn capture_region(
        &mut self,
        output: &wl_output::WlOutput,
        rect: Rect,
        hardware: &HardwareFrame,
    ) -> Result<(i64, bool)> {
        use std::os::fd::BorrowedFd;

        let (frame, mut state) = self.request_region(output, rect)?;
        let result = (|| {
            let width = state.width.context("No capture width received")?;
            let height = state.height.context("No capture height received")?;
            anyhow::ensure!(
                hardware.dimensions() == (width, height),
                "Capture dimensions changed to {width}x{height}"
            );
            let format = state
                .format
                .context("The compositor did not offer DMA-BUF screencopy")?;
            let descriptor = hardware.descriptor();
            anyhow::ensure!(
                descriptor.nb_layers == 1,
                "Multi-layer DMA-BUF is unsupported"
            );
            let qh = self.event_queue.handle();
            let params = self.dmabuf.create_params(&qh, ());
            let layer = &descriptor.layers[0];
            for plane_index in 0..layer.nb_planes {
                let plane = &layer.planes[plane_index as usize];
                let object = &descriptor.objects[plane.object_index as usize];
                let fd = unsafe { BorrowedFd::borrow_raw(object.fd) };
                params.add(
                    fd,
                    plane_index as u32,
                    plane.offset as u32,
                    plane.pitch as u32,
                    (object.format_modifier >> 32) as u32,
                    object.format_modifier as u32,
                );
            }
            let buffer = params.create_immed(
                width as i32,
                height as i32,
                format,
                zwp_linux_buffer_params_v1::Flags::empty(),
                &qh,
                (),
            );
            frame.copy(&buffer);
            while !state.ready && !state.failed {
                self.event_queue.blocking_dispatch(&mut state)?;
            }
            buffer.destroy();
            params.destroy();
            anyhow::ensure!(!state.failed, "The compositor rejected DMA-BUF screencopy");
            Ok((
                state
                    .timestamp_micros
                    .context("The compositor omitted the capture timestamp")?,
                state.y_inverted,
            ))
        })();
        frame.destroy();
        result
    }

    fn request_region(
        &mut self,
        output: &wl_output::WlOutput,
        rect: Rect,
    ) -> Result<(ZwlrScreencopyFrameV1, CaptureState)> {
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
        let mut state = CaptureState::new();
        while !state.offered && !state.failed {
            self.event_queue.blocking_dispatch(&mut state)?;
        }
        anyhow::ensure!(!state.failed, "The compositor rejected the capture request");
        Ok((frame, state))
    }
}
