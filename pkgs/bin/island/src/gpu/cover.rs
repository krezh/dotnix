//! Surfaces over an output while something of the island is open, taking clicks outside it and,
//! for a panel with a field to type in, the keyboard.

use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shell::{WaylandSurface, wlr_layer::LayerSurface},
};
use wayland_client::protocol::wl_output::WlOutput;

use super::renderer::Gpu;
use crate::stage::SURFACE_W;

pub struct Cover {
    /// Goes before the layer it draws on.
    pub surface: Option<wgpu::Surface<'static>>,
    pub layer: LayerSurface,
    pub output: WlOutput,
    pub size: (u32, u32),
    pub scale: i32,
    region: Option<Vec<[i32; 4]>>,
}

/// `hole` taken out of `rect`.
fn cut(rect: [i32; 4], hole: [i32; 4]) -> Vec<[i32; 4]> {
    let [x, y, w, h] = rect;
    let [hx, hy, hw, hh] = hole;
    let (left, top) = (hx.max(x), hy.max(y));
    let (right, bottom) = ((hx + hw).min(x + w), (hy + hh).min(y + h));
    if left >= right || top >= bottom {
        return vec![rect];
    }
    [
        [x, y, w, top - y],
        [x, bottom, w, y + h - bottom],
        [x, top, left - x, bottom - top],
        [right, top, x + w - right, bottom - top],
    ]
    .into_iter()
    .filter(|part| part[2] > 0 && part[3] > 0)
    .collect()
}

impl Cover {
    pub fn new(layer: LayerSurface, output: WlOutput, scale: i32) -> Self {
        Self {
            layer,
            output,
            surface: None,
            size: (0, 0),
            scale,
            region: None,
        }
    }

    /// Draws at the whole `scale` from now on.
    pub fn rescale(&mut self, scale: i32, gpu: Option<&Gpu>) {
        if scale == self.scale {
            return;
        }
        self.scale = scale;
        self.layer.wl_surface().set_buffer_scale(scale);
        if let Some(gpu) = gpu.filter(|_| self.size.0 > 0) {
            self.draw(gpu);
        }
    }

    pub fn draw(&self, gpu: &Gpu) {
        if let Some(surface) = &self.surface {
            let scale = self.scale.max(1) as u32;
            gpu.configure(surface, self.size.0 * scale, self.size.1 * scale);
            gpu.draw(surface, &[], scale as f32);
        }
    }

    /// Everywhere but `holes`, which are in the island surface's coordinates.
    pub fn take_pointer_around(&mut self, holes: &[[i32; 4]], compositor: &CompositorState) {
        let (width, height) = (self.size.0 as i32, self.size.1 as i32);
        let shift = (width - SURFACE_W as i32) / 2;
        let wanted = holes
            .iter()
            .fold(vec![[0, 0, width, height]], |region, hole| {
                let hole = [hole[0] + shift, hole[1], hole[2], hole[3]];
                region
                    .into_iter()
                    .flat_map(|rect| cut(rect, hole))
                    .collect()
            });
        if self.region.as_ref() == Some(&wanted) {
            return;
        }
        let Ok(region) = Region::new(compositor) else {
            return;
        };
        for [x, y, w, h] in &wanted {
            region.add(*x, *y, *w, *h);
        }
        self.layer
            .wl_surface()
            .set_input_region(Some(region.wl_region()));
        self.layer.commit();
        self.region = Some(wanted);
    }
}
