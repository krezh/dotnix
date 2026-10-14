pub mod handlers;

use std::sync::Arc;
use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::{OutputInfo, OutputState};
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::reexports::calloop::signals::{Signal, Signals};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopSignal};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::Shm;
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_callback, wl_output, wl_pointer, wl_shm, wl_surface};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::{wp_fractional_scale_manager_v1, wp_fractional_scale_v1};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

use crate::animations::clock::ClockOverlay;
use crate::animations::{create_animation, Animation, ANIMATION_NAMES};
use crate::canvas::{Canvas, Viewport};
use crate::config::AppConfig;
use crate::font::FontRenderer;
use crate::rng::Rng;

type RenderResult = Result<(), Box<dyn std::error::Error>>;

struct FrameSchedule {
    next: Instant,
    callback_pending: bool,
}

impl FrameSchedule {
    fn new(now: Instant) -> Self {
        Self { next: now, callback_pending: false }
    }

    fn deadline(&self, configured: bool) -> Option<Instant> {
        (configured && !self.callback_pending).then_some(self.next)
    }

    /// Keeps the cadence while slightly late, so render time does not push every later frame back.
    fn committed(&mut self, rendered_at: Instant, interval: Duration) {
        let next = self.next + interval;
        self.next = if next >= rendered_at { next } else { rendered_at + interval };
        self.callback_pending = true;
    }
}

struct PreviousScene {
    animation: Box<dyn Animation>,
    name: &'static str,
}

struct Scene {
    animation: Box<dyn Animation>,
    name: &'static str,
    initial_index: usize,
    generation: u64,
    origin: Instant,
    started: Instant,
    last_update: Instant,
    previous: Option<PreviousScene>,
}

impl Scene {
    fn new(config: &AppConfig, rng: &mut Rng, now: Instant) -> Self {
        let name = ANIMATION_NAMES.iter().copied()
            .find(|name| name.eq_ignore_ascii_case(&config.animation))
            .unwrap_or_else(|| *rng.choice(ANIMATION_NAMES));
        let initial_index = ANIMATION_NAMES.iter().position(|candidate| *candidate == name).unwrap();
        Self {
            animation: create_animation(name, rng),
            name,
            initial_index,
            generation: 0,
            origin: now,
            started: now,
            last_update: now,
            previous: None,
        }
    }

    fn advance(&mut self, now: Instant, config: &AppConfig, rng: &mut Rng, width: u32, height: u32) {
        let generation = if config.cycle == 0 { 0 } else {
            now.duration_since(self.origin).as_secs() / config.cycle
        };
        if generation != self.generation {
            let index = (self.initial_index + (generation % ANIMATION_NAMES.len() as u64) as usize)
                % ANIMATION_NAMES.len();
            let name = ANIMATION_NAMES[index];
            let previous = PreviousScene {
                animation: std::mem::replace(&mut self.animation, create_animation(name, rng)),
                name: self.name,
            };
            self.previous = (config.transition_ms > 0).then_some(previous);
            self.name = name;
            self.generation = generation;
            self.started = self.origin + Duration::from_secs(generation * config.cycle);
            self.last_update = self.started;
        }
        if now.duration_since(self.started) >= Duration::from_millis(config.transition_ms) {
            self.previous = None;
        }
        if now > self.last_update {
            self.animation.update(now.duration_since(self.last_update).as_secs_f32() * config.speed, width, height);
            self.last_update = now;
        }
    }
}

struct Transition {
    started: Instant,
    from_scene: bool,
    pixels: Option<Vec<u32>>,
}

pub struct OutputSurface {
    pub output: wl_output::WlOutput,
    pub layer_surface: LayerSurface,
    pub surface: wl_surface::WlSurface,
    label: String,
    logical_size: (u32, u32),
    configured_size: (u32, u32),
    output_size: (u32, u32),
    /// The output's top-left corner in the compositor's logical layout.
    position: (i32, i32),
    frame_interval: Duration,
    buffer_size: (u32, u32),
    integer_scale: i32,
    preferred_scale: Option<u32>,
    fractional_scale: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
    viewport: Option<wp_viewport::WpViewport>,
    pub configured: bool,
    pool: Option<SlotPool>,
    pub frame_callback: Option<wl_callback::WlCallback>,
    schedule: FrameSchedule,
    scene: Option<Scene>,
    scene_generation: u64,
    clock: ClockOverlay,
    transition: Option<Transition>,
    presented: bool,
}

impl OutputSurface {
    fn refresh_size(&mut self) -> RenderResult {
        let logical = configured_size(self.configured_size, self.output_size);
        let integer_scale = (self.integer_scale.max(1) as u32).checked_mul(120)
            .ok_or("integer output scale exceeds the supported range")?;
        let scale = if self.viewport.is_some() {
            self.preferred_scale.unwrap_or(integer_scale)
        } else {
            integer_scale
        };
        let buffer = scaled_size(logical, scale)?;
        if logical != self.logical_size || buffer != self.buffer_size {
            self.logical_size = logical;
            self.buffer_size = buffer;
            self.pool = None;
            if let Some(transition) = &mut self.transition {
                transition.pixels = None;
            }
        }
        Ok(())
    }

    fn deadline(&self) -> Option<Instant> {
        self.schedule.deadline(self.configured && self.buffer_size.0 > 0 && self.buffer_size.1 > 0)
    }
}

impl Drop for OutputSurface {
    fn drop(&mut self) {
        if let Some(scale) = &self.fractional_scale {
            scale.destroy();
        }
        if let Some(viewport) = &self.viewport {
            viewport.destroy();
        }
    }
}

fn configured_size(configure: (u32, u32), fallback: (u32, u32)) -> (u32, u32) {
    (if configure.0 == 0 { fallback.0 } else { configure.0 },
     if configure.1 == 0 { fallback.1 } else { configure.1 })
}

fn scaled_size(logical: (u32, u32), scale: u32) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    if scale == 0 {
        return Err("compositor supplied a zero fractional scale".into());
    }
    if logical.0 > i32::MAX as u32 || logical.1 > i32::MAX as u32 {
        return Err("logical surface dimensions exceed Wayland viewport limits".into());
    }
    let width = (logical.0 as u64 * scale as u64).div_ceil(120);
    let height = (logical.1 as u64 * scale as u64).div_ceil(120);
    if width > i32::MAX as u64 / 4 || height > i32::MAX as u64 {
        return Err(format!("buffer dimensions {width}x{height} exceed Wayland SHM limits").into());
    }
    Ok((width as u32, height as u32))
}

fn output_logical_size(info: &OutputInfo) -> (u32, u32) {
    if let Some((width, height)) = info.logical_size {
        return (width.max(0) as u32, height.max(0) as u32);
    }
    let Some(mode) = info.modes.iter().find(|mode| mode.current) else { return (0, 0) };
    let (mut width, mut height) = mode.dimensions;
    if matches!(info.transform, wl_output::Transform::_90 | wl_output::Transform::_270
        | wl_output::Transform::Flipped90 | wl_output::Transform::Flipped270) {
        std::mem::swap(&mut width, &mut height);
    }
    let scale = info.scale_factor.max(1) as u32;
    ((width.max(0) as u32).div_ceil(scale), (height.max(0) as u32).div_ceil(scale))
}

/// A whole number of the output's refresh periods, the one nearest the target frame rate, so every
/// frame stays on screen equally long instead of alternating between vblank counts.
fn frame_interval(fps: u32, refresh_mhz: i32) -> Duration {
    if refresh_mhz <= 0 {
        return Duration::from_nanos(1_000_000_000u64.div_ceil(fps as u64));
    }
    let refresh = refresh_mhz as f64 / 1000.0;
    Duration::from_secs_f64((refresh / fps as f64).round().max(1.0) / refresh)
}

fn refresh_mhz(info: &OutputInfo) -> i32 {
    info.modes.iter().find(|mode| mode.current).map_or(0, |mode| mode.refresh_rate)
}

/// One output's slice of the desktop all outputs span, in that output's buffer pixels. Outputs are
/// `(logical position, logical size)`; ones without a size yet are left out of the desktop.
fn span_viewport(
    outputs: impl Iterator<Item = ((i32, i32), (u32, u32))>,
    position: (i32, i32),
    logical_size: (u32, u32),
    buffer_size: (u32, u32),
) -> Option<Viewport> {
    if logical_size.0 == 0 || logical_size.1 == 0 {
        return None;
    }
    let (left, top, right, bottom) = outputs
        .filter(|(_, (width, height))| *width > 0 && *height > 0)
        .fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |(left, top, right, bottom), ((x, y), (width, height))| {
            (left.min(x), top.min(y), right.max(x + width as i32), bottom.max(y + height as i32))
        });
    if left >= right || top >= bottom {
        return None;
    }
    let scale_x = buffer_size.0 as f32 / logical_size.0 as f32;
    let scale_y = buffer_size.1 as f32 / logical_size.1 as f32;
    Some(Viewport {
        x: (position.0 - left) as f32 * scale_x,
        y: (position.1 - top) as f32 * scale_y,
        width: (right - left) as f32 * scale_x,
        height: (bottom - top) as f32 * scale_y,
    })
}

fn transition_weight(now: Instant, start: Instant, duration: Duration) -> u32 {
    if duration.is_zero() { return 256; }
    let t = (now.saturating_duration_since(start).as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t) * 256.0).round() as u32
}

fn mix_pixel(from: u32, to: u32, weight: u32) -> u32 {
    let inverse = 256 - weight;
    let rb = (((from & 0x00ff00ff) * inverse + (to & 0x00ff00ff) * weight) >> 8) & 0x00ff00ff;
    let ag = ((((from >> 8) & 0x00ff00ff) * inverse + ((to >> 8) & 0x00ff00ff) * weight) >> 8) & 0x00ff00ff;
    rb | (ag << 8)
}

pub struct App {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub layer_shell: LayerShell,
    pub loop_signal: LoopSignal,
    pub qh: QueueHandle<Self>,
    fractional_manager: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    viewporter: Option<wp_viewporter::WpViewporter>,
    pub config: AppConfig,
    pub surfaces: Vec<OutputSurface>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub rng: Rng,
    pub start_time: Instant,
    pub last_pointer_pos: Option<(f64, f64)>,
    font_renderer: Arc<FontRenderer>,
    shared_scene: Option<Scene>,
    error: Option<String>,
    pub exit: bool,
}

impl App {
    fn fail_surface(&mut self, idx: usize, error: impl std::fmt::Display) {
        let surface = &self.surfaces[idx];
        self.error = Some(format!("output {} (logical {}x{}, buffer {}x{}): {error}",
            surface.label, surface.logical_size.0, surface.logical_size.1,
            surface.buffer_size.0, surface.buffer_size.1));
        self.exit = true;
        self.loop_signal.stop();
    }

    fn refresh_surface(&mut self, idx: usize) {
        if let Err(error) = self.surfaces[idx].refresh_size() {
            self.fail_surface(idx, error);
        }
    }

    fn update_output_surface(&mut self, output: &wl_output::WlOutput, info: &OutputInfo) {
        if let Some(idx) = self.surfaces.iter().position(|surface| &surface.output == output) {
            let surface = &mut self.surfaces[idx];
            surface.integer_scale = info.scale_factor.max(1);
            surface.output_size = output_logical_size(info);
            surface.position = info.logical_position.unwrap_or_default();
            surface.frame_interval = frame_interval(self.config.fps, refresh_mhz(info));
            if let Some(name) = &info.name {
                surface.label.clone_from(name);
            }
            self.refresh_surface(idx);
        }
    }

    fn render_surface(&mut self, idx: usize, now: Instant) -> RenderResult {
        let viewport = if self.config.span_outputs {
            let surface = &self.surfaces[idx];
            span_viewport(
                self.surfaces.iter().map(|output| (output.position, output.logical_size)),
                surface.position, surface.logical_size, surface.buffer_size,
            )
        } else {
            None
        };
        let surface = &mut self.surfaces[idx];
        let (width, height) = surface.buffer_size;
        let scene = surface.scene.as_mut().or(self.shared_scene.as_mut()).unwrap();
        scene.advance(now, &self.config, &mut self.rng, width, height);
        let duration = Duration::from_millis(self.config.transition_ms);
        if !surface.presented {
            surface.transition = (!duration.is_zero()).then_some(Transition {
                started: now, from_scene: false, pixels: None,
            });
        } else if surface.scene_generation != scene.generation {
            surface.transition = (scene.previous.is_some()).then_some(Transition {
                started: scene.started, from_scene: true, pixels: None,
            });
        }
        surface.scene_generation = scene.generation;
        let elapsed = now.duration_since(self.start_time).as_secs_f32();
        let weight = surface.transition.as_ref()
            .map_or(256, |transition| transition_weight(now, transition.started, duration));
        if weight == 256 {
            surface.transition = None;
        }
        let pixel_count = (width as usize).checked_mul(height as usize).ok_or("SHM pixel count overflow")?;
        if let Some(transition) = &mut surface.transition {
            if transition.from_scene && transition.pixels.is_none() {
                if let Some(previous) = &mut scene.previous {
                    let mut pixels = vec![0; pixel_count];
                    let mut canvas = Canvas::new(&mut pixels, width, height);
                    if let Some(viewport) = viewport {
                        canvas = canvas.with_viewport(viewport);
                    }
                    previous.animation.render(&mut canvas, &self.config.theme);
                    if self.config.show_clock || previous.name == "clock" {
                        surface.clock.draw(&mut canvas, &self.config.theme, elapsed);
                    }
                    transition.pixels = Some(pixels);
                }
            }
        }
        if surface.pool.is_none() {
            let pool_size = pixel_count.checked_mul(8).ok_or("SHM pool size overflow")?;
            surface.pool = Some(SlotPool::new(pool_size, &self.shm_state)?);
        }
        let pool = surface.pool.as_mut().unwrap();
        let (buffer, bytes) = pool.create_buffer(width as i32, height as i32, width as i32 * 4, wl_shm::Format::Argb8888)?;
        // SHM slots are aligned, but reject an incompatible mapping rather than presenting stale bytes.
        let (prefix, pixels, suffix) = unsafe { bytes.align_to_mut::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() || pixels.len() != pixel_count {
            return Err("SHM buffer is not an aligned ARGB8888 pixel array".into());
        }
        let mut canvas = Canvas::new(pixels, width, height);
        if let Some(viewport) = viewport {
            canvas = canvas.with_viewport(viewport);
        }
        scene.animation.render(&mut canvas, &self.config.theme);
        if self.config.show_clock || scene.name == "clock" {
            surface.clock.draw(&mut canvas, &self.config.theme, elapsed);
        }
        if let Some(transition) = &surface.transition {
            if let Some(previous) = &transition.pixels {
                for (pixel, from) in pixels.iter_mut().zip(previous) {
                    *pixel = mix_pixel(*from, *pixel, weight);
                }
            } else {
                for pixel in pixels.iter_mut() {
                    *pixel = mix_pixel(0, *pixel, weight);
                }
            }
        }
        if let Some(viewport) = &surface.viewport {
            surface.surface.set_buffer_scale(1);
            viewport.set_destination(surface.logical_size.0 as i32, surface.logical_size.1 as i32);
        } else {
            surface.surface.set_buffer_scale(surface.integer_scale.max(1));
        }
        buffer.attach_to(&surface.surface)?;
        surface.surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.frame_callback = Some(surface.surface.frame(&self.qh, ()));
        surface.surface.commit();
        surface.schedule.committed(now, surface.frame_interval);
        surface.presented = true;
        Ok(())
    }

    fn tick(&mut self) {
        if self.exit { return; }
        let now = Instant::now();
        for idx in 0..self.surfaces.len() {
            if self.surfaces[idx].deadline().is_some_and(|deadline| deadline <= now) {
                if let Err(error) = self.render_surface(idx, now) {
                    self.fail_surface(idx, error);
                    break;
                }
            }
        }
    }

    fn next_timeout(&self, now: Instant) -> Option<Duration> {
        self.surfaces.iter().filter_map(OutputSurface::deadline)
            .min().map(|deadline| deadline.saturating_duration_since(now))
    }

    pub fn add_output_surface(&mut self, output: &wl_output::WlOutput, info: &OutputInfo) {
        if self.surfaces.iter().any(|surface| &surface.output == output) { return; }
        let surface = self.compositor_state.create_surface(&self.qh);
        let layer_surface = self.layer_shell.create_layer_surface(
            &self.qh, surface.clone(), Layer::Overlay, Some("slumber-screensaver"), Some(output),
        );
        let (fractional_scale, viewport) = match (&self.fractional_manager, &self.viewporter) {
            (Some(manager), Some(viewporter)) => (
                Some(manager.get_fractional_scale(&surface, &self.qh, surface.clone())),
                Some(viewporter.get_viewport(&surface, &self.qh, ())),
            ),
            _ => (None, None),
        };
        layer_surface.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(0, 0);
        surface.commit();
        let now = Instant::now();
        let scene = if self.config.sync_outputs { None } else {
            Some(Scene::new(&self.config, &mut self.rng, now))
        };
        self.surfaces.push(OutputSurface {
            output: output.clone(), layer_surface, surface,
            label: info.name.clone().unwrap_or_else(|| format!("{} {}", info.make, info.model)),
            logical_size: (0, 0), configured_size: (0, 0), output_size: output_logical_size(info),
            position: info.logical_position.unwrap_or_default(),
            frame_interval: frame_interval(self.config.fps, refresh_mhz(info)),
            buffer_size: (0, 0), integer_scale: info.scale_factor.max(1), preferred_scale: None,
            fractional_scale, viewport, configured: false, pool: None, frame_callback: None,
            schedule: FrameSchedule::new(now), scene, scene_generation: 0,
            clock: ClockOverlay::new(Arc::clone(&self.font_renderer), &self.config.time_format,
                &self.config.date_format, self.config.clock_drift),
            transition: None, presented: false,
        });
        self.refresh_surface(self.surfaces.len() - 1);
    }
}

pub fn run(config: AppConfig) -> RenderResult {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();
    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let shm_state = Shm::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let fractional_manager = globals.bind(&qh, 1..=1, ()).ok();
    let viewporter = globals.bind(&qh, 1..=1, ()).ok();
    let mut event_loop: EventLoop<App> = EventLoop::try_new()?;
    let loop_signal = event_loop.get_signal();
    let mut rng = Rng::from_time();
    let start_time = Instant::now();
    let font_renderer = Arc::new(FontRenderer::new(&config.font));
    let shared_scene = config.sync_outputs.then(|| Scene::new(&config, &mut rng, start_time));
    let mut app = App {
        registry_state, seat_state, output_state, compositor_state, shm_state, layer_shell,
        loop_signal, qh: qh.clone(), fractional_manager, viewporter, config,
        surfaces: Vec::new(), pointer: None, rng, start_time, last_pointer_pos: None,
        font_renderer, shared_scene, error: None, exit: false,
    };
    let outputs: Vec<_> = app.output_state.outputs().collect();
    for output in outputs {
        if let Some(info) = app.output_state.info(&output) {
            app.add_output_surface(&output, &info);
        }
    }
    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|error| format!("failed to insert Wayland event source: {error:?}"))?;
    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM, Signal::SIGUSR1])?;
    event_loop.handle().insert_source(signals, move |_sig, _meta, app: &mut App| {
        app.exit = true;
        app.loop_signal.stop();
    })?;
    while !app.exit {
        app.tick();
        if app.exit { break; }
        event_loop.dispatch(app.next_timeout(Instant::now()), &mut app)?;
    }
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_callback_never_bypasses_deadline() {
        let now = Instant::now();
        let interval = Duration::from_millis(25);
        let mut schedule = FrameSchedule::new(now);
        assert_eq!(schedule.deadline(true), Some(now));
        schedule.committed(now, interval);
        assert_eq!(schedule.deadline(true), None);
        schedule.callback_pending = false;
        assert_eq!(schedule.deadline(true), Some(now + interval));
        assert!(schedule.deadline(true).unwrap() > now + Duration::from_millis(4));
    }

    #[test]
    fn blocked_and_unconfigured_outputs_do_not_arm_deadlines() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::new(now);
        assert_eq!(schedule.deadline(false), None);
        schedule.committed(now, Duration::from_millis(16));
        assert_eq!(schedule.deadline(true), None);
        assert_eq!(schedule.deadline(false), None);
    }

    #[test]
    fn delayed_frames_do_not_catch_up_in_a_burst() {
        let now = Instant::now();
        let interval = Duration::from_millis(25);
        let mut schedule = FrameSchedule::new(now);
        schedule.committed(now, interval);
        schedule.callback_pending = false;
        let resumed = now + Duration::from_secs(5);
        schedule.committed(resumed, interval);
        schedule.callback_pending = false;
        assert_eq!(schedule.deadline(true), Some(resumed + interval));
    }

    #[test]
    fn render_latency_does_not_accumulate_into_the_cadence() {
        let start = Instant::now();
        let interval = Duration::from_millis(25);
        let mut schedule = FrameSchedule::new(start);
        for frame in 1..=10 {
            let rendered_at = schedule.deadline(true).unwrap() + Duration::from_millis(3);
            schedule.committed(rendered_at, interval);
            schedule.callback_pending = false;
            assert_eq!(schedule.deadline(true), Some(start + interval * frame));
        }
    }

    #[test]
    fn integer_and_fractional_sizes_round_up() {
        assert_eq!(scaled_size((1920, 1080), 120).unwrap(), (1920, 1080));
        assert_eq!(scaled_size((1920, 1080), 240).unwrap(), (3840, 2160));
        assert_eq!(scaled_size((1001, 801), 150).unwrap(), (1252, 1002));
        assert_eq!(scaled_size((1001, 801), 180).unwrap(), (1502, 1202));
        assert_eq!(scaled_size((0, 0), 150).unwrap(), (0, 0));
        assert!(scaled_size((100, 100), 0).is_err());
        assert!(scaled_size((u32::MAX, u32::MAX), 240).is_err());
    }

    #[test]
    fn zero_configure_dimensions_use_output_size_independently() {
        assert_eq!(configured_size((0, 0), (1920, 1080)), (1920, 1080));
        assert_eq!(configured_size((800, 0), (1920, 1080)), (800, 1080));
        assert_eq!(configured_size((0, 600), (1920, 1080)), (1920, 600));
        assert_eq!(configured_size((800, 600), (1920, 1080)), (800, 600));
    }

    #[test]
    fn transitions_preserve_premultiplied_channels() {
        assert_eq!(mix_pixel(0, 0x80402010, 128), 0x40201008);
        assert_eq!(mix_pixel(0x80402010, 0xff806040, 0), 0x80402010);
        assert_eq!(mix_pixel(0x80402010, 0xff806040, 256), 0xff806040);
        let now = Instant::now();
        let duration = Duration::from_millis(600);
        assert_eq!(transition_weight(now, now, duration), 0);
        assert_eq!(transition_weight(now + duration / 2, now, duration), 128);
        assert_eq!(transition_weight(now + duration, now, duration), 256);
        assert_eq!(transition_weight(now, now, Duration::ZERO), 256);
    }

    #[test]
    fn frame_interval_is_a_whole_number_of_refresh_periods() {
        let periods = |fps, refresh_mhz| frame_interval(fps, refresh_mhz).as_secs_f64() * refresh_mhz as f64 / 1000.0;
        assert!((periods(60, 143_998) - 2.0).abs() < 1e-6);
        assert!((periods(60, 239_970) - 4.0).abs() < 1e-6);
        assert!((periods(240, 59_940) - 1.0).abs() < 1e-6);
        assert_eq!(frame_interval(60, 0), Duration::from_nanos(16_666_667));
    }

    #[test]
    fn span_viewport_places_each_output_within_the_desktop() {
        let outputs = [((0, 0), (2560, 1440)), ((2560, 200), (1920, 1080)), ((9000, 0), (0, 0))];
        let viewport = |position, logical, buffer| span_viewport(outputs.into_iter(), position, logical, buffer);
        assert_eq!(viewport((0, 0), (2560, 1440), (2560, 1440)),
            Some(Viewport { x: 0.0, y: 0.0, width: 4480.0, height: 1440.0 }));
        assert_eq!(viewport((2560, 200), (1920, 1080), (2880, 1620)),
            Some(Viewport { x: 3840.0, y: 300.0, width: 6720.0, height: 2160.0 }));
        assert_eq!(viewport((9000, 0), (0, 0), (0, 0)), None);
    }
}
