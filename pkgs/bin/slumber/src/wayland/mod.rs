pub mod handlers;

use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::{OutputInfo, OutputState};
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::reexports::calloop::signals::{Signal, Signals};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopSignal};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::Shm;
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_callback, wl_output, wl_pointer, wl_shm, wl_surface};
use wayland_client::{Connection, QueueHandle};

use crate::animations::{create_animation, Animation};
use crate::canvas::Canvas;
use crate::config::AppConfig;
use crate::rng::Rng;

pub struct OutputSurface {
    pub output: wl_output::WlOutput,
    pub layer_surface: LayerSurface,
    pub surface: wl_surface::WlSurface,
    pub width: u32,
    pub height: u32,
    pub configured: bool,
    pub pool: Option<SlotPool>,
    pub frame_callback: Option<wl_callback::WlCallback>,
    pub waiting_for_frame: bool,
    pub animation: Box<dyn Animation>,
    pub needs_clear: bool,
    pub last_tick: Instant,
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

    pub config: AppConfig,
    pub surfaces: Vec<OutputSurface>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub rng: Rng,
    pub start_time: Instant,
    pub last_tick: Instant,
    pub last_pointer_pos: Option<(f64, f64)>,
    pub cycle_timer: f32,
    pub total_time: f32,
    pub font_renderer: crate::font::FontRenderer,
    pub current_anim_index: usize,
    pub exit: bool,
}

impl App {
    pub fn render_surface(&mut self, idx: usize, qh: &QueueHandle<Self>, dt: f32) -> Result<(), Box<dyn std::error::Error>> {
        let surface = &mut self.surfaces[idx];
        if !surface.configured || surface.width == 0 || surface.height == 0 {
            return Ok(());
        }

        let width = surface.width as i32;
        let height = surface.height as i32;

        let pool = match &mut surface.pool {
            Some(p) => p,
            None => {
                let pool_size = (width * height * 4 * 2) as usize;
                surface.pool = Some(SlotPool::new(pool_size, &self.shm_state)?);
                surface.pool.as_mut().unwrap()
            }
        };

        let stride = width * 4;
        let buffer = match pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888) {
            Ok((buf, _)) => buf,
            Err(_) => {
                let pool_size = (width * height * 4 * 2) as usize;
                pool.resize(pool_size)?;
                let (buf, _) = pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)?;
                buf
            }
        };

        {
            let canvas_slice = buffer.canvas(pool).ok_or("Buffer slot is still in use by compositor")?;
            let (_, u32_slice, _) = unsafe { canvas_slice.align_to_mut::<u32>() };
            if u32_slice.len() >= (width * height) as usize {
                let mut canvas = Canvas::new(
                    &mut u32_slice[0..(width * height) as usize],
                    surface.width,
                    surface.height,
                );
                if surface.needs_clear {
                    canvas.clear(self.config.theme.crust);
                    surface.needs_clear = false;
                }
                let effective_dt = dt * self.config.speed;
                surface.animation.update(effective_dt, surface.width, surface.height);
                surface.animation.render(&mut canvas, &self.config.theme);
                if self.config.show_clock {
                    crate::animations::clock::draw_clock_overlay(
                        &mut canvas,
                        &self.font_renderer,
                        &self.config.theme,
                        self.total_time,
                    );
                }
            }
        }

        buffer.attach_to(&surface.surface)?;
        surface.surface.damage_buffer(0, 0, width, height);

        let cb = surface.surface.frame(qh, ());
        surface.frame_callback = Some(cb);
        surface.waiting_for_frame = true;
        surface.surface.commit();

        Ok(())
    }

    pub fn advance_time(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.1);
        self.last_tick = now;
        self.total_time = now.duration_since(self.start_time).as_secs_f32();
        if self.config.cycle > 0 {
            self.cycle_timer += dt;
            if self.cycle_timer >= self.config.cycle as f32 {
                self.cycle_timer = 0.0;
                self.cycle_animation();
            }
        }
    }

    pub fn cycle_animation(&mut self) {
        self.current_anim_index = (self.current_anim_index + 1) % crate::animations::ANIMATION_NAMES.len();
        let next_anim = crate::animations::ANIMATION_NAMES[self.current_anim_index];
        for surface in &mut self.surfaces {
            surface.animation = create_animation(next_anim, &mut self.rng, &self.config.font);
            surface.needs_clear = true;
        }
    }

    pub fn tick(&mut self) {
        self.advance_time();

        let qh = self.qh.clone();
        for i in 0..self.surfaces.len() {
            if self.surfaces[i].configured && !self.surfaces[i].waiting_for_frame {
                let now = Instant::now();
                let dt = now.duration_since(self.surfaces[i].last_tick).as_secs_f32().min(0.05);
                self.surfaces[i].last_tick = now;
                let _ = self.render_surface(i, &qh, dt);
            }
        }
    }

    pub fn add_output_surface(&mut self, output: &wl_output::WlOutput, _info: &OutputInfo) {
        let surface = self.compositor_state.create_surface(&self.qh);
        let layer_surface = self.layer_shell.create_layer_surface(
            &self.qh,
            surface.clone(),
            Layer::Overlay,
            Some("slumber-screensaver"),
            Some(output),
        );

        layer_surface.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(0, 0);
        surface.commit();

        let anim = create_animation(&self.config.animation, &mut self.rng, &self.config.font);

        self.surfaces.push(OutputSurface {
            output: output.clone(),
            layer_surface,
            surface,
            width: 0,
            height: 0,
            configured: false,
            pool: None,
            frame_callback: None,
            waiting_for_frame: false,
            animation: anim,
            needs_clear: false,
            last_tick: Instant::now(),
        });
    }
}

pub fn run(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let shm_state = Shm::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;

    let mut event_loop: EventLoop<App> = EventLoop::try_new()?;
    let loop_signal = event_loop.get_signal();

    let rng = Rng::from_time();
    let start_time = Instant::now();
    let last_tick = Instant::now();

    let font_renderer = crate::font::FontRenderer::new(&config.font);
    let initial_anim_idx = crate::animations::ANIMATION_NAMES
        .iter()
        .position(|&name| name.eq_ignore_ascii_case(&config.animation))
        .unwrap_or(0);
    let mut app = App {
        registry_state,
        seat_state,
        output_state,
        compositor_state,
        shm_state,
        layer_shell,
        loop_signal,
        qh: qh.clone(),
        config: config.clone(),
        surfaces: Vec::new(),
        pointer: None,
        rng,
        start_time,
        last_tick,
        last_pointer_pos: None,
        cycle_timer: 0.0,
        total_time: 0.0,
        font_renderer,
        current_anim_index: initial_anim_idx,
        exit: false,
    };

    // Initialize initial outputs
    let outputs: Vec<_> = app.output_state.outputs().collect();
    for output in outputs {
        if let Some(info) = app.output_state.info(&output) {
            app.add_output_surface(&output, &info);
        }
    }

    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|e| format!("Failed to insert wayland source: {:?}", e))?;

    // Signal handlers for clean immediate exit
    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM, Signal::SIGUSR1])?;
    event_loop.handle().insert_source(signals, move |_sig, _meta, app: &mut App| {
        app.exit = true;
        app.loop_signal.stop();
    })?;

    // Timer for target framerate
    let frame_interval = Duration::from_micros((1_000_000.0 / config.fps as f64).round() as u64);
    let timer = Timer::immediate();
    event_loop.handle().insert_source(timer, move |_deadline, _, app: &mut App| {
        if app.exit {
            return TimeoutAction::Drop;
        }
        app.tick();
        TimeoutAction::ToDuration(frame_interval)
    })?;

    while !app.exit {
        event_loop.dispatch(Some(Duration::from_millis(2)), &mut app)?;
    }

    Ok(())
}
