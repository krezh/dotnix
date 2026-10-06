pub mod generated {
    use wayland_client;
    use wayland_client::protocol::*;
    use wayland_protocols_wlr::foreign_toplevel::v1::client::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        use wayland_protocols_wlr::foreign_toplevel::v1::client::__interfaces::*;
        wayland_scanner::generate_interfaces!("./protocols/hyprland-toplevel-export-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("./protocols/hyprland-toplevel-export-v1.xml");
}

use anyhow::{Context, Result};
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop,
    globals::GlobalListContents,
    protocol::{wl_buffer, wl_registry, wl_shm},
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
};

use self::generated::{
    hyprland_toplevel_export_frame_v1::{self, HyprlandToplevelExportFrameV1},
    hyprland_toplevel_export_manager_v1::HyprlandToplevelExportManagerV1,
};
use crate::replay::hardware::HardwareFrame;

struct CaptureState {
    width: Option<u32>,
    height: Option<u32>,
    stride: Option<u32>,
    format: Option<wl_shm::Format>,
    ready: bool,
    failed: bool,
    y_inverted: bool,
    dmabuf_format: Option<u32>,
    timestamp_micros: Option<i64>,
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
            y_inverted: false,
            dmabuf_format: None,
            timestamp_micros: None,
        }
    }

    fn reset_frame(&mut self) {
        self.width = None;
        self.height = None;
        self.stride = None;
        self.format = None;
        self.ready = false;
        self.failed = false;
        self.y_inverted = false;
        self.dmabuf_format = None;
        self.timestamp_micros = None;
    }
}

impl Dispatch<HyprlandToplevelExportFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &HyprlandToplevelExportFrameV1,
        event: hyprland_toplevel_export_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            hyprland_toplevel_export_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                state.width = Some(width);
                state.height = Some(height);
                state.stride = Some(stride);
                if let WEnum::Value(format) = format {
                    state.format = Some(format);
                }
            }
            hyprland_toplevel_export_frame_v1::Event::Flags {
                flags: WEnum::Value(flags),
            } => {
                state.y_inverted =
                    flags.contains(hyprland_toplevel_export_frame_v1::Flags::YInvert);
            }
            hyprland_toplevel_export_frame_v1::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let seconds = (i64::from(tv_sec_hi) << 32) | i64::from(tv_sec_lo);
                state.timestamp_micros = Some(seconds * 1_000_000 + i64::from(tv_nsec) / 1_000);
                state.ready = true;
            }
            hyprland_toplevel_export_frame_v1::Event::Failed => state.failed = true,
            hyprland_toplevel_export_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                state.dmabuf_format = Some(format);
                state.width = Some(width);
                state.height = Some(height);
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

delegate_noop!(CaptureState: ignore wl_shm::WlShm);
delegate_noop!(CaptureState: ignore wl_buffer::WlBuffer);
delegate_noop!(CaptureState: ignore HyprlandToplevelExportManagerV1);

pub struct HyprlandToplevelCapture {
    event_queue: wayland_client::EventQueue<CaptureState>,
    state: CaptureState,
    manager: HyprlandToplevelExportManagerV1,
    dmabuf: ZwpLinuxDmabufV1,
}

impl HyprlandToplevelCapture {
    pub fn new(connection: &Connection) -> Result<Self> {
        use wayland_client::globals::registry_queue_init;

        let (globals, mut event_queue) =
            registry_queue_init::<CaptureState>(connection).context("Failed to init registry")?;
        let qh = event_queue.handle();
        let manager = globals
            .bind(&qh, 2..=2, ())
            .context("hyprland_toplevel_export_manager_v1 not available")?;
        let dmabuf = globals
            .bind(&qh, 3..=5, ())
            .context("zwp_linux_dmabuf_v1 not available")?;
        let mut state = CaptureState::new();
        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;

        Ok(Self {
            event_queue,
            state,
            manager,
            dmabuf,
        })
    }

    pub fn probe_hardware(&mut self, handle: u32) -> Result<(u32, u32, u32)> {
        self.state.reset_frame();
        let qh = self.event_queue.handle();
        let frame = self.manager.capture_toplevel(0, handle, &qh, ());
        let result = (|| {
            self.event_queue.roundtrip(&mut self.state)?;
            Ok((
                self.state.width.context("No toplevel width received")?,
                self.state.height.context("No toplevel height received")?,
                self.state
                    .dmabuf_format
                    .context("Hyprland did not offer DMA-BUF toplevel capture")?,
            ))
        })();
        frame.destroy();
        result
    }

    pub fn capture_hardware(
        &mut self,
        handle: u32,
        hardware: &HardwareFrame,
    ) -> Result<(i64, bool)> {
        use std::os::fd::BorrowedFd;

        self.state.reset_frame();
        let qh = self.event_queue.handle();
        let frame = self.manager.capture_toplevel(0, handle, &qh, ());
        let result = (|| {
            self.event_queue.roundtrip(&mut self.state)?;
            let width = self.state.width.context("No toplevel width received")?;
            let height = self.state.height.context("No toplevel height received")?;
            anyhow::ensure!(
                hardware.dimensions() == (width, height),
                "Toplevel dimensions changed to {}x{}",
                width,
                height
            );
            let format = self
                .state
                .dmabuf_format
                .context("Hyprland did not offer DMA-BUF toplevel capture")?;
            let descriptor = hardware.descriptor();
            anyhow::ensure!(
                descriptor.nb_layers == 1,
                "Multi-layer DMA-BUF is unsupported"
            );
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
            let result = (|| {
                frame.copy(&buffer, 1);
                while !self.state.ready && !self.state.failed {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                }
                anyhow::ensure!(!self.state.failed, "Hyprland rejected DMA-BUF capture");
                let timestamp = self
                    .state
                    .timestamp_micros
                    .context("Hyprland omitted the capture timestamp")?;
                Ok((timestamp, self.state.y_inverted))
            })();
            buffer.destroy();
            params.destroy();
            result
        })();
        frame.destroy();
        result
    }
}
