//! Draws lists of sprites with wgpu: every shape, picture and glyph is one instance of one pipeline.

use std::{cell::RefCell, ffi::c_void, ptr::NonNull};

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use tracing::info;

pub const ATLAS: u32 = 2048;
const FIRST_ROOM: usize = 8192;

const SHADER: &str = r#"
struct Screen { size: vec2<f32>, pad: vec2<f32> }
const GAMMA_1_8 = vec4<f32>(0.14805, -0.89459, 1.47591, -0.32467);

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var nearest: sampler;
@group(0) @binding(3) var blended: sampler;

struct In {
    @builtin(vertex_index) index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) params: vec4<f32>,
    @location(4) clip: vec4<f32>,
}
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) texel: vec2<f32>,
    @location(2) @interpolate(flat) rect: vec4<f32>,
    @location(3) @interpolate(flat) uv: vec4<f32>,
    @location(4) @interpolate(flat) color: vec4<f32>,
    @location(5) @interpolate(flat) params: vec4<f32>,
    @location(6) @interpolate(flat) clip: vec4<f32>,
}

@vertex fn vs(v: In) -> Out {
    let at = vec2<f32>(f32(v.index & 1u), f32(v.index >> 1u));
    var pos = v.rect.xy + at * v.rect.zw;
    if (v.params.w != 0.0) {
        let middle = v.rect.xy + v.rect.zw * 0.5;
        let arm = pos - middle;
        let s = sin(v.params.w);
        let c = cos(v.params.w);
        pos = middle + vec2<f32>(arm.x * c - arm.y * s, arm.x * s + arm.y * c);
    }
    var out: Out;
    out.position = vec4<f32>(pos.x / screen.size.x * 2.0 - 1.0, 1.0 - pos.y / screen.size.y * 2.0, 0.0, 1.0);
    out.local = pos;
    out.texel = v.uv.xy + at * v.uv.zw;
    out.rect = v.rect;
    out.uv = v.uv;
    out.color = v.color;
    out.params = v.params;
    out.clip = v.clip;
    return out;
}

fn corner_distance(local: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>, radius: f32, power: f32) -> f32 {
    let r = min(radius, min(half_size.x, half_size.y));
    let q = abs(local - center) - half_size + r;
    let out = max(q, vec2<f32>(0.0));
    let bend = pow(pow(out.x, power) + pow(out.y, power), 1.0 / power);
    return bend + min(max(q.x, q.y), 0.0) - r;
}

fn ink(coverage: f32, color: vec4<f32>) -> f32 {
    let shade = color.rgb / max(color.a, 0.0001);
    let brightness = dot(shade, vec3<f32>(0.30, 0.59, 0.11));
    let contrast = saturate(4.0 * (0.75 - brightness));
    let a = coverage * (contrast + 1.0) / (coverage * contrast + 1.0);
    let correction = (GAMMA_1_8.x * brightness + GAMMA_1_8.y) * a + (GAMMA_1_8.z * brightness + GAMMA_1_8.w);
    return a + a * (1.0 - a) * correction;
}

@fragment fn fs(v: Out) -> @location(0) vec4<f32> {
    if (v.local.x < v.clip.x || v.local.y < v.clip.y || v.local.x > v.clip.z || v.local.y > v.clip.w) {
        discard;
    }
    let center = v.rect.xy + v.rect.zw * 0.5;
    let radius = v.params.x;
    let kind = v.params.y;
    let power = v.params.z;
    let pixel = screen.pad.x;
    if (kind < 0.5) {
        return v.color * clamp(0.5 - corner_distance(v.local, center, v.rect.zw * 0.5, radius, power) * pixel, 0.0, 1.0);
    } else if (kind < 1.5) {
        let sampled = textureSampleLevel(atlas, nearest, v.texel, 0.0);
        if (power < 0.5) {
            return v.color * ink(sampled.a, v.color);
        }
        if (radius > 0.0) {
            return sampled * v.color * clamp(0.5 - corner_distance(v.local, center, v.rect.zw * 0.5, radius, power) * pixel, 0.0, 1.0);
        }
        return sampled * v.color;
    } else if (kind < 2.5) {
        if (v.uv.z > 0.5) {
            let sigma = v.uv.x;
            let shape_half = v.rect.zw * 0.5 - 3.0 * sigma;
            let d = corner_distance(v.local, center, shape_half, radius, power);
            let t = clamp(d / (sigma * 1.41421356), -4.0, 4.0);
            let spread = tanh(1.1283792 * t + 0.1009 * t * t * t);
            let body = corner_distance(v.local, center - vec2<f32>(0.0, v.uv.y), shape_half, radius, power);
            return v.color * (0.5 - 0.5 * spread) * clamp(0.5 + body * pixel, 0.0, 1.0);
        }
        let blur = v.uv.x;
        let half_size = v.rect.zw * 0.5 - blur;
        let d = corner_distance(v.local, center, half_size, radius, power);
        let body = corner_distance(v.local, center - vec2<f32>(0.0, v.uv.y), half_size, radius, power);
        return v.color * (1.0 - smoothstep(-blur, blur, d)) * clamp(0.5 + body, 0.0, 1.0);
    }
    if (kind < 3.5) {
        let d = corner_distance(v.local, center, v.rect.zw * 0.5, radius, power);
        return v.color * clamp(0.5 + (0.5 - abs(d + 0.5)) * pixel, 0.0, 1.0);
    }
    if (kind < 4.5) {
        let a = v.rect.xy + v.uv.xy;
        let ab = v.rect.xy + v.uv.zw - a;
        let t = clamp(dot(v.local - a, ab) / max(dot(ab, ab), 0.0001), 0.0, 1.0);
        return v.color * clamp(0.5 + (radius - length(v.local - a - ab * t)) * pixel, 0.0, 1.0);
    }
    if (kind > 8.5) {
        let across = v.local.x - v.rect.x;
        var col = across;
        if (v.uv.x > 0.5) {
            col = v.rect.z - across;
        }
        let r = v.uv.z;
        let line = v.uv.w;
        let row = v.local.y - v.rect.y - line;
        let outside = length(vec2<f32>(col, r - row)) - r;
        let outer = clamp(0.5 + outside * pixel, 0.0, 1.0);
        if (v.uv.y > 0.5) {
            return v.color * (outer - clamp(0.5 + (outside - line) * pixel, 0.0, 1.0));
        }
        if (col < r && row > 0.0) {
            return v.color * outer;
        }
        return vec4<f32>(0.0);
    }
    if (kind > 7.5) {
        let light = textureSampleLevel(atlas, blended, v.texel, 0.0);
        return light * v.color * clamp(0.5 - corner_distance(v.local, center, v.rect.zw * 0.5, radius, power) * pixel, 0.0, 1.0);
    }
    if (kind > 6.5) {
        let spoke = v.local - center;
        let tau = 6.2831853;
        var turn = atan2(spoke.x, -spoke.y) / tau;
        if (turn < 0.0) {
            turn = turn + 1.0;
        }
        var d = abs(length(spoke) - v.uv.x) - radius;
        if (turn < v.uv.y || turn > v.uv.z) {
            let start = v.uv.x * vec2<f32>(sin(v.uv.y * tau), -cos(v.uv.y * tau));
            let stop = v.uv.x * vec2<f32>(sin(v.uv.z * tau), -cos(v.uv.z * tau));
            d = min(length(spoke - start), length(spoke - stop)) - radius;
        }
        return v.color * clamp(0.5 - d * pixel, 0.0, 1.0);
    }
    if (kind > 5.5) {
        let blur = v.uv.x;
        let d = corner_distance(v.local, center, v.rect.zw * 0.5 - blur, radius, power);
        return v.color * (1.0 - smoothstep(-blur, blur, d));
    }
    let along = clamp((v.local.x - v.rect.x) / max(v.rect.z, 0.0001), 0.0, 1.0);
    let edge = v.rect.y + mix(v.uv.x, v.uv.y, along);
    return v.color * clamp((v.local.y - edge) * pixel + 0.5, 0.0, 1.0);
}
"#;

/// Everywhere, for a sprite nothing cuts off.
/// What became of a frame. Past `Shown`, nothing reached the compositor.
pub enum Presented {
    Shown,
    Skipped,
    /// The surface has to be set up again.
    Outdated,
    /// The surface has to be made again.
    Lost,
}

pub const NO_CLIP: [f32; 4] = [-1.0e6, -1.0e6, 1.0e6, 1.0e6];

/// Whether `at` is in a rect given as left, top, width and height.
pub fn inside(rect: [f32; 4], (x, y): (f32, f32)) -> bool {
    x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
}

/// What two clips, each left, top, right and bottom, have in common.
pub fn overlap(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sprite {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    /// Premultiplied.
    pub color: [f32; 4],
    /// Corner radius, kind, corner power and a turn about its middle.
    pub params: [f32; 4],
    /// Left, top, right and bottom.
    pub clip: [f32; 4],
}

pub const SHAPE: f32 = 0.0;
pub const IMAGE: f32 = 1.0;
const SHADOW: f32 = 2.0;
pub const RING: f32 = 3.0;
const LINE: f32 = 4.0;
const AREA: f32 = 5.0;
const GLOW: f32 = 6.0;
const ARC: f32 = 7.0;
const SOFT: f32 = 8.0;
const FILLET: f32 = 9.0;

impl Sprite {
    pub fn shape(rect: [f32; 4], color: [f32; 4], radius: f32, power: f32) -> Self {
        Self {
            rect,
            uv: [0.0; 4],
            color,
            params: [radius, SHAPE, power, 0.0],
            clip: NO_CLIP,
        }
    }

    pub fn ring(rect: [f32; 4], color: [f32; 4], radius: f32, power: f32) -> Self {
        Self {
            params: [radius, RING, power, 0.0],
            ..Self::shape(rect, color, radius, power)
        }
    }

    pub fn image(rect: [f32; 4], uv: [f32; 4], color: [f32; 4]) -> Self {
        Self {
            rect,
            uv,
            color,
            params: [0.0, IMAGE, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// A glyph's or a symbol's coverage in `color`, thickened for how light that is.
    pub fn mask(rect: [f32; 4], uv: [f32; 4], color: [f32; 4]) -> Self {
        Self {
            params: [0.0, IMAGE, 0.0, 0.0],
            ..Self::image(rect, uv, color)
        }
    }

    /// `rect` is the shadow's whole reach, `blur` past the shape, which it stays out from under
    /// after dropping `drop`.
    pub fn shadow(rect: [f32; 4], color: [f32; 4], radius: f32, blur: f32, drop: f32) -> Self {
        Self {
            rect,
            uv: [blur, drop, 0.0, 0.0],
            color,
            params: [radius, SHADOW, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// The shadow of `shape`, blurred as a Gaussian of `sigma`, `drop` below it and never under it.
    pub fn drop_shadow(
        [x, y, w, h]: [f32; 4],
        color: [f32; 4],
        radius: f32,
        sigma: f32,
        drop: f32,
    ) -> Self {
        let reach = 3.0 * sigma;
        Self {
            rect: [
                x - reach,
                y + drop - reach,
                w + 2.0 * reach,
                h + 2.0 * reach,
            ],
            uv: [sigma, drop, 1.0, 0.0],
            color,
            params: [radius, SHADOW, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// From `a` to `b`, with round ends.
    pub fn line(a: (f32, f32), b: (f32, f32), width: f32, color: [f32; 4]) -> Self {
        let pad = width / 2.0 + 1.0;
        let (x0, y0) = (a.0.min(b.0) - pad, a.1.min(b.1) - pad);
        let (x1, y1) = (a.0.max(b.0) + pad, a.1.max(b.1) + pad);
        Self {
            rect: [x0, y0, x1 - x0, y1 - y0],
            uv: [a.0 - x0, a.1 - y0, b.0 - x0, b.1 - y0],
            color,
            params: [width / 2.0, LINE, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// Filled from the slope between `a` and `b` down to `bottom`.
    pub fn area(a: (f32, f32), b: (f32, f32), bottom: f32, color: [f32; 4]) -> Self {
        let top = a.1.min(b.1) - 1.0;
        Self {
            rect: [a.0, top, b.0 - a.0, bottom - top],
            uv: [a.1 - top, b.1 - top, 0.0, 0.0],
            color,
            params: [0.0, AREA, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// Light with no edges, `blur` wide, filling `rect`.
    pub fn glow(rect: [f32; 4], color: [f32; 4], radius: f32, blur: f32) -> Self {
        Self {
            rect,
            uv: [blur, 0.0, 0.0, 0.0],
            color,
            params: [radius, GLOW, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// Round `rect`'s middle, `stroke` thick, from the share `from` of the way round from the top,
    /// clockwise, to `to`, with round ends.
    pub fn arc(rect: [f32; 4], stroke: f32, from: f32, to: f32, color: [f32; 4]) -> Self {
        Self {
            rect,
            uv: [rect[2] / 2.0 - stroke / 2.0, from, to, 0.0],
            color,
            params: [stroke / 2.0, ARC, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    /// A coarse picture smoothed up to fill `rect`.
    pub fn soft(rect: [f32; 4], uv: [f32; 4], color: [f32; 4], radius: f32, power: f32) -> Self {
        Self {
            rect,
            uv,
            color,
            params: [radius, SOFT, power, 0.0],
            clip: NO_CLIP,
        }
    }

    /// The hollow corner joining what hangs from an edge to that edge, `radius` across below
    /// `line`, opening away from the hanging shape; `right` is the corner on its right. With
    /// `edge`, only its line.
    pub fn fillet(
        rect: [f32; 4],
        color: [f32; 4],
        radius: f32,
        line: f32,
        right: bool,
        edge: bool,
    ) -> Self {
        let flag = |on: bool| if on { 1.0 } else { 0.0 };
        Self {
            rect,
            uv: [flag(right), flag(edge), radius, line],
            color,
            params: [0.0, FILLET, 2.0, 0.0],
            clip: NO_CLIP,
        }
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.params[0] = radius;
        self
    }

    pub fn moved(mut self, x: f32, y: f32) -> Self {
        self.rect[0] += x;
        self.rect[1] += y;
        if self.clip != NO_CLIP {
            self.clip[0] += x;
            self.clip[1] += y;
            self.clip[2] += x;
            self.clip[3] += y;
        }
        self
    }

    pub fn faded(mut self, by: f32) -> Self {
        self.color = self.color.map(|channel| channel * by);
        self
    }

    /// Keeps it inside `clip` as well as anything already cutting it off.
    pub fn within(mut self, clip: [f32; 4]) -> Self {
        self.clip = overlap(self.clip, clip);
        self
    }

    /// As [`Self::within`], for what may have nothing cutting it off.
    pub fn within_any(self, clip: Option<[f32; 4]>) -> Self {
        clip.map_or(self, |clip| self.within(clip))
    }
}

fn sprite_room(device: &wgpu::Device, sprites: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (sprites * size_of::<Sprite>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn bytes<T>(items: &[T]) -> &[u8] {
    // Sprites are plain floats, and the screen's size is too.
    unsafe { std::slice::from_raw_parts(items.as_ptr().cast(), std::mem::size_of_val(items)) }
}

#[derive(Debug)]
struct Display(NonNull<c_void>);
// Only ever handed to wgpu, which keeps it for the connection's life.
unsafe impl Send for Display {}
unsafe impl Sync for Display {}

impl HasDisplayHandle for Display {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(self.0));
        // The connection outlives the instance holding this.
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

pub struct Gpu {
    screen: wgpu::Buffer,
    size: [f32; 2],
    instance: wgpu::Instance,
    display: NonNull<c_void>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    pub atlas: wgpu::Texture,
    items: RefCell<wgpu::Buffer>,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    present_mode: wgpu::PresentMode,
}

fn surface(
    instance: &wgpu::Instance,
    display: NonNull<c_void>,
    window: NonNull<c_void>,
) -> Result<wgpu::Surface<'static>, String> {
    // The Wayland surface outlives this one: an island drops its wgpu surface first.
    unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                display,
            ))),
            raw_window_handle: RawWindowHandle::Wayland(WaylandWindowHandle::new(window)),
        })
    }
    .map_err(|error| format!("cannot draw on the surface: {error}"))
}

impl Gpu {
    /// Started from the first surface, as an adapter has to be able to draw on it. `size` is what
    /// sprites are placed in.
    pub fn new(
        display: NonNull<c_void>,
        window: NonNull<c_void>,
        size: [f32; 2],
    ) -> Result<(Self, wgpu::Surface<'static>), String> {
        let mut descriptor =
            wgpu::InstanceDescriptor::new_with_display_handle(Box::new(Display(display)));
        descriptor.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(descriptor);
        let first = surface(&instance, display, window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&first),
            ..Default::default()
        }))
        .map_err(|error| format!("no graphics adapter: {error}"))?;
        info!(adapter = adapter.get_info().name, "drawing with wgpu");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            ..Default::default()
        }))
        .map_err(|error| format!("cannot open the graphics device: {error}"))?;
        let caps = first.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| *format == wgpu::TextureFormat::Bgra8Unorm)
            .or_else(|| caps.formats.first().copied())
            .ok_or("the surface takes no format")?;
        let alpha_mode = if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            caps.alpha_modes[0]
        };
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: ATLAS,
                height: ATLAS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let smooth = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let screen = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&screen, 0, bytes(&[size[0], size[1], 0.0, 0.0]));
        let items = RefCell::new(sprite_room(&device, FIRST_ROOM));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4
        ];
        let blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<Sprite>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: blend,
                        alpha: blend,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: screen.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&smooth),
                },
            ],
        });
        Ok((
            Self {
                screen,
                size,
                instance,
                display,
                adapter,
                device,
                queue,
                pipeline,
                bind_group,
                atlas,
                items,
                format,
                alpha_mode,
                present_mode,
            },
            first,
        ))
    }

    pub fn surface(&self, window: NonNull<c_void>) -> Result<wgpu::Surface<'static>, String> {
        let made = surface(&self.instance, self.display, window)?;
        if !self.adapter.is_surface_supported(&made) {
            return Err("the graphics adapter cannot draw on this surface".to_owned());
        }
        Ok(made)
    }

    pub fn configure(&self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: self.present_mode,
                desired_maximum_frame_latency: 2,
                alpha_mode: self.alpha_mode,
                color_space: wgpu::SurfaceColorSpace::Auto,
                view_formats: vec![],
            },
        );
    }

    /// `scale` is how many of its pixels make one of the sprites' units.
    pub fn draw(&self, surface: &wgpu::Surface<'_>, sprites: &[Sprite], scale: f32) -> Presented {
        let mut items = self.items.borrow_mut();
        if items.size() < size_of_val(sprites) as u64 {
            *items = sprite_room(&self.device, sprites.len().next_power_of_two());
        }
        self.queue.write_buffer(
            &self.screen,
            0,
            bytes(&[self.size[0], self.size[1], scale, 0.0]),
        );
        self.queue.write_buffer(&items, 0, bytes(sprites));
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => return Presented::Outdated,
            wgpu::CurrentSurfaceTexture::Lost => return Presented::Lost,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return Presented::Skipped,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, items.slice(..));
            pass.draw(0..4, 0..sprites.len() as u32);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Presented::Shown
    }
}

#[cfg(test)]
#[path = "renderer_test.rs"]
mod tests;
