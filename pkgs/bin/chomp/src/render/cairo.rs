use anyhow::{Context, Result};
use cairo::{Context as CairoContext, Format, ImageSurface};
use std::cell::RefCell;

use super::pixel::blit;
use super::selection::{Rect, Selection};
use super::{ModePaletteLayout, PaletteAction, ReplayPaletteState};
use crate::config::FontWeight;

/// The frozen screen a selection is drawn on.
///
/// `dimmed` is the same pixels with the overlay's dim already applied, prepared
/// once when the screen is frozen so that dragging a selection only copies
/// rectangles instead of blending the whole screen each frame.
#[derive(Clone, Copy)]
pub struct FrozenFrame<'a> {
    pub pixels: &'a [u8],
    pub dimmed: Option<&'a [u8]>,
    pub stride: i32,
}
/// Operation represented by the region-selection overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPurpose {
    Screenshot,
    Recording,
    Ocr,
}

/// Context displayed while the user selects a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionHud {
    pub purpose: SelectionPurpose,
    pub to_clipboard: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LabelRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn dimension_label_rect(
    selection: Rect,
    label_width: f64,
    label_height: f64,
    surface_width: i32,
    surface_height: i32,
) -> LabelRect {
    const MARGIN: f64 = 8.0;
    let max_x = (f64::from(surface_width) - label_width - MARGIN).max(MARGIN);
    let x = (f64::from(selection.x) + (f64::from(selection.width) - label_width) / 2.0)
        .clamp(MARGIN, max_x);
    let below = f64::from(selection.y + selection.height) + MARGIN;
    let above = f64::from(selection.y) - label_height - MARGIN;
    let max_y = (f64::from(surface_height) - label_height - MARGIN).max(MARGIN);
    let y = if below + label_height <= f64::from(surface_height) - MARGIN {
        below
    } else if above >= MARGIN {
        above
    } else {
        (f64::from(selection.y) + MARGIN).clamp(MARGIN, max_y)
    };

    LabelRect {
        x,
        y,
        width: label_width,
        height: label_height,
    }
}

pub(crate) fn selection_hud_pointer_near(pointer: (f64, f64), surface_width: i32) -> bool {
    const PANEL_Y: f64 = 16.0;
    const PANEL_HEIGHT: f64 = 58.0;
    const GUARD_WIDTH: f64 = 420.0;
    const PROXIMITY: f64 = 48.0;

    let surface_width = f64::from(surface_width);
    let guard_width = GUARD_WIDTH.min(surface_width);
    let panel_x = (surface_width - guard_width) / 2.0;

    pointer.0 >= panel_x - PROXIMITY
        && pointer.0 <= panel_x + guard_width + PROXIMITY
        && pointer.1 >= (PANEL_Y - PROXIMITY).max(0.0)
        && pointer.1 <= PANEL_Y + PANEL_HEIGHT + PROXIMITY
}

const MIN_TEXT_WIDTH: i32 = 48;
const MIN_TEXT_HEIGHT: i32 = 24;

/// Color representation
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');

        let (r, g, b) = if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            (r, g, b)
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16)? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16)? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16)? * 17;
            (r, g, b)
        } else {
            anyhow::bail!("Invalid hex color format");
        };

        Ok(Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: 1.0,
        })
    }
}

pub struct RenderConfig {
    pub border_color: Color,
    pub panel_background_color: Color,
    pub text_color: Color,
    pub border_weight: u32,
    pub border_radius: u32,
    pub dim_opacity: f64,
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: cairo::FontWeight,
}

impl RenderConfig {
    pub fn new(
        border_color: &str,
        panel_background_color: &str,
        text_color: &str,
        border_weight: u32,
        border_radius: u32,
        dim_opacity: f64,
        font_family: String,
        font_size: u32,
        font_weight: FontWeight,
    ) -> Result<Self> {
        Ok(Self {
            border_color: Color::from_hex(border_color)?,
            panel_background_color: Color::from_hex(panel_background_color)?,
            text_color: Color::from_hex(text_color)?,
            border_weight,
            border_radius,
            dim_opacity,
            font_family,
            font_size: font_size as f64,
            font_weight: font_weight.to_cairo(),
        })
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            border_color: Color::from_hex("#89B4FA").unwrap(),
            panel_background_color: Color::from_hex("#1E1E2E").unwrap(),
            text_color: Color::from_hex("#CDD6F4").unwrap(),
            border_weight: 2,
            border_radius: 0,
            dim_opacity: 0.5,
            font_family: "Inter Nerd Font".to_string(),
            font_size: 18.0,
            font_weight: cairo::FontWeight::Bold,
        }
    }
}

pub struct Renderer {
    config: RenderConfig,
    width: i32,
    height: i32,
    cached_text: RefCell<Option<(i32, i32, String)>>, // (width, height, rendered_text)
}

impl Renderer {
    pub fn new(width: i32, height: i32, config: RenderConfig) -> Self {
        Self {
            config,
            width,
            height,
            cached_text: RefCell::new(None),
        }
    }

    /// Renders the bounded mode palette with a centered pop-in entrance.
    pub fn render_mode_select(
        &self,
        buffer: &mut [u8],
        keybinds: &crate::config::KeybindsConfig,
        style: &crate::config::ModeSelectConfig,
        is_recording: bool,
        supports_window_capture: bool,
        replay_status: Option<&crate::replay::ReplayStatus>,
        replay_configured: bool,
        hovered_action: Option<PaletteAction>,
        intro_progress: f64,
    ) -> Result<()> {
        let stride = self.width * 4;
        let surface = unsafe {
            ImageSurface::create_for_data_unsafe(
                buffer.as_mut_ptr(),
                Format::ARgb32,
                self.width,
                self.height,
                stride,
            )?
        };
        let ctx = CairoContext::new(&surface).context("Failed to create Cairo context")?;

        ctx.set_operator(cairo::Operator::Source);
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        ctx.paint()?;
        ctx.set_operator(cairo::Operator::Over);

        let progress = intro_progress.clamp(0.0, 1.0);
        let replay_state = ReplayPaletteState {
            visible: replay_configured || replay_status.is_some(),
            can_save: replay_status.as_ref().map_or(false, |s| s.can_save()),
        };
        let layout = ModePaletteLayout::new(
            self.width,
            self.height,
            style.control_height,
            is_recording,
            supports_window_capture,
            replay_state,
        );
        let background = Color::from_hex(&style.background_color)?;
        let description = Color::from_hex(&style.description_color)?;
        let surface_color = Color::from_hex(&style.surface_color)?;
        let key = Color::from_hex(&style.key_color)?;
        let recording = Color::from_hex(&style.recording_dot_color)?;
        let stop = Color::from_hex(&style.recording_highlight_color)?;
        let replay_color = Color::from_hex(&style.replay_color)?;

        let bounds = layout.bounds;
        let scale = 0.4 + 0.6 * progress;
        let center_x = bounds.x + bounds.width / 2.0;
        let center_y = bounds.y + bounds.height / 2.0;
        ctx.save()?;
        ctx.translate(center_x, center_y);
        ctx.scale(scale, scale);
        ctx.translate(-center_x, -center_y);
        self.draw_rounded_rectangle(&ctx, bounds.x, bounds.y, bounds.width, bounds.height, 14.0)?;
        ctx.set_source_rgba(
            background.r,
            background.g,
            background.b,
            style.background_opacity,
        );
        ctx.fill_preserve()?;
        ctx.set_source_rgba(
            self.config.border_color.r,
            self.config.border_color.g,
            self.config.border_color.b,
            style.border_opacity,
        );
        ctx.set_line_width(1.0);
        ctx.stroke()?;

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
        );
        ctx.set_font_size((self.config.font_size * 0.72).max(11.0));
        ctx.set_source_rgba(
            description.r,
            description.g,
            description.b,
            style.description_opacity * 0.72,
        );
        ctx.move_to(bounds.x + 16.0, layout.capture_heading_y + 13.0);
        ctx.show_text("Capture")?;

        if is_recording {
            ctx.set_source_rgba(recording.r, recording.g, recording.b, 1.0);
        }
        ctx.move_to(bounds.x + 16.0, layout.record_heading_y + 13.0);
        ctx.show_text(if is_recording {
            "Recording active"
        } else {
            "Record"
        })?;

        if replay_state.visible {
            let (replay_text, replay_is_active) = match replay_status {
                Some(status) if status.can_save() => {
                    let text = format!(
                        "Replay ready · {:.1}s",
                        status.buffered_millis as f64 / 1000.0
                    );
                    (text, true)
                }
                Some(status) => match status.state {
                    crate::replay::ServiceState::WaitingForTarget => {
                        ("Replay waiting for target".to_string(), false)
                    }
                    crate::replay::ServiceState::Buffering => {
                        let text = format!(
                            "Replay buffering · {:.1}s",
                            status.buffered_millis as f64 / 1000.0
                        );
                        (text, false)
                    }
                    crate::replay::ServiceState::RetainingAfterExit => {
                        let text = format!(
                            "Replay retained · {:.1}s",
                            status.buffered_millis as f64 / 1000.0
                        );
                        (text, false)
                    }
                    crate::replay::ServiceState::Suspended => {
                        ("Replay suspended".to_string(), false)
                    }
                    crate::replay::ServiceState::Failed => ("Replay failed".to_string(), false),
                },
                None => ("Replay unavailable".to_string(), false),
            };

            if replay_is_active {
                ctx.set_source_rgba(replay_color.r, replay_color.g, replay_color.b, 1.0);
            } else {
                ctx.set_source_rgba(
                    description.r,
                    description.g,
                    description.b,
                    style.description_opacity * 0.72,
                );
            }
            ctx.move_to(bounds.x + 16.0, layout.replay_heading_y + 13.0);
            ctx.show_text(&replay_text)?;
        }

        for item in layout.items() {
            let hovered = hovered_action == Some(item.action);
            let action_color = if item.action == PaletteAction::StopRecording {
                stop
            } else if item.action == PaletteAction::SaveReplay {
                replay_color
            } else {
                key
            };
            let is_status_action = matches!(
                item.action,
                PaletteAction::StopRecording | PaletteAction::SaveReplay
            );
            let fill_color = if is_status_action {
                action_color
            } else {
                surface_color
            };
            let fill_alpha = match (is_status_action, hovered) {
                (true, true) => 0.24,
                (true, false) => 0.12,
                (false, true) => 0.96,
                (false, false) => 0.78,
            };
            self.draw_rounded_rectangle(
                &ctx,
                item.rect.x,
                item.rect.y,
                item.rect.width,
                item.rect.height,
                9.0,
            )?;
            ctx.set_source_rgba(fill_color.r, fill_color.g, fill_color.b, fill_alpha);
            ctx.fill_preserve()?;
            let border_color = if hovered { action_color } else { description };
            ctx.set_source_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                if hovered {
                    0.80
                } else {
                    style.control_border_opacity
                },
            );
            ctx.set_line_width(1.0);
            ctx.stroke()?;

            let (configured_key, full_label) = match item.action {
                PaletteAction::ScreenshotArea => (&keybinds.screenshot_area, "Area"),
                PaletteAction::ScreenshotScreen => (&keybinds.screenshot_screen, "Screen"),
                PaletteAction::ScreenshotWindow => (&keybinds.screenshot_window, "Window"),
                PaletteAction::Ocr => (&keybinds.ocr, "Text"),
                PaletteAction::RecordArea => (&keybinds.record_area, "Area"),
                PaletteAction::RecordScreen => (&keybinds.record_screen, "Screen"),
                PaletteAction::RecordWindow => (&keybinds.record_window, "Window"),
                PaletteAction::StopRecording => (&keybinds.stop_recording, "Stop recording"),
                PaletteAction::SaveReplay => (&keybinds.replay_save, "Save replay"),
            };

            let shifted = matches!(
                item.action,
                PaletteAction::RecordArea
                    | PaletteAction::RecordScreen
                    | PaletteAction::RecordWindow
            ) && configured_key
                .chars()
                .next()
                .is_some_and(char::is_uppercase);

            let degraded_label = if item.rect.width < 140.0 {
                match item.action {
                    PaletteAction::StopRecording => "Stop",
                    PaletteAction::SaveReplay => "Save",
                    _ => full_label,
                }
            } else {
                full_label
            };

            ctx.select_font_face(
                &self.config.font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            let key_font_size = (self.config.font_size * 0.80).clamp(11.0, 14.0);
            ctx.set_font_size(key_font_size);
            let key_font = ctx.font_extents()?;

            let keycap_pad = if item.rect.width < 100.0 { 4.0 } else { 6.0 };
            let keycap_h = (item.rect.height - 12.0).clamp(18.0, 24.0);

            let key_ext = ctx.text_extents(configured_key)?;
            let key_w = (key_ext.x_advance() + keycap_pad * 2.0).max(keycap_h);

            let (shift_w, shift_text_width) = if shifted {
                let width = ctx.text_extents("Shift")?.x_advance();
                (width + keycap_pad * 2.0, width)
            } else {
                (0.0, 0.0)
            };

            let keycaps_gap = 4.0;
            let total_keycaps_w = if shifted {
                shift_w + keycaps_gap + key_w
            } else {
                key_w
            };

            ctx.select_font_face(
                &self.config.font_family,
                cairo::FontSlant::Normal,
                self.config.font_weight,
            );
            let label_font_size = (self.config.font_size * 0.90).clamp(11.0, 15.0);
            ctx.set_font_size(label_font_size);
            let label_font = ctx.font_extents()?;
            let label_ext = ctx.text_extents(degraded_label)?;

            let label_gap = 7.0;
            let show_label =
                total_keycaps_w + label_gap + label_ext.x_advance() <= item.rect.width - 10.0;
            let content_w = if show_label {
                total_keycaps_w + label_gap + label_ext.x_advance()
            } else {
                total_keycaps_w
            };

            let mut cur_x = item.rect.x + (item.rect.width - content_w) / 2.0;
            let keycap_y = item.rect.y + (item.rect.height - keycap_h) / 2.0;

            if shifted {
                self.draw_rounded_rectangle(&ctx, cur_x, keycap_y, shift_w, keycap_h, 4.0)?;
                ctx.set_source_rgba(
                    background.r,
                    background.g,
                    background.b,
                    if hovered { 0.96 } else { 0.88 },
                );
                ctx.fill_preserve()?;
                ctx.set_source_rgba(
                    action_color.r,
                    action_color.g,
                    action_color.b,
                    if hovered { 0.75 } else { 0.35 },
                );
                ctx.set_line_width(1.0);
                ctx.stroke()?;

                ctx.move_to(cur_x + 2.0, keycap_y + keycap_h);
                ctx.line_to(cur_x + shift_w - 2.0, keycap_y + keycap_h);
                ctx.set_source_rgba(
                    action_color.r,
                    action_color.g,
                    action_color.b,
                    if hovered { 0.90 } else { 0.50 },
                );
                ctx.set_line_width(1.5);
                ctx.stroke()?;

                ctx.select_font_face(
                    &self.config.font_family,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                );
                ctx.set_font_size(key_font_size);
                let text_x = cur_x + (shift_w - shift_text_width) / 2.0;
                let text_y = keycap_y + (keycap_h + key_font.ascent() - key_font.descent()) / 2.0;
                ctx.set_source_rgba(action_color.r, action_color.g, action_color.b, 1.0);
                ctx.move_to(text_x, text_y);
                ctx.show_text("Shift")?;

                cur_x += shift_w + keycaps_gap;
            }

            self.draw_rounded_rectangle(&ctx, cur_x, keycap_y, key_w, keycap_h, 4.0)?;
            ctx.set_source_rgba(
                background.r,
                background.g,
                background.b,
                if hovered { 0.96 } else { 0.88 },
            );
            ctx.fill_preserve()?;
            ctx.set_source_rgba(
                action_color.r,
                action_color.g,
                action_color.b,
                if hovered { 0.75 } else { 0.35 },
            );
            ctx.set_line_width(1.0);
            ctx.stroke()?;

            ctx.move_to(cur_x + 2.0, keycap_y + keycap_h);
            ctx.line_to(cur_x + key_w - 2.0, keycap_y + keycap_h);
            ctx.set_source_rgba(
                action_color.r,
                action_color.g,
                action_color.b,
                if hovered { 0.90 } else { 0.50 },
            );
            ctx.set_line_width(1.5);
            ctx.stroke()?;

            ctx.select_font_face(
                &self.config.font_family,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            ctx.set_font_size(key_font_size);
            let text_x = cur_x + (key_w - key_ext.x_advance()) / 2.0;
            let text_y = keycap_y + (keycap_h + key_font.ascent() - key_font.descent()) / 2.0;
            ctx.set_source_rgba(action_color.r, action_color.g, action_color.b, 1.0);
            ctx.move_to(text_x, text_y);
            ctx.show_text(configured_key)?;

            cur_x += key_w + label_gap;

            if show_label {
                ctx.select_font_face(
                    &self.config.font_family,
                    cairo::FontSlant::Normal,
                    self.config.font_weight,
                );
                ctx.set_font_size(label_font_size);
                let label_y = item.rect.y
                    + (item.rect.height + label_font.ascent() - label_font.descent()) / 2.0;
                ctx.set_source_rgba(
                    description.r,
                    description.g,
                    description.b,
                    if hovered {
                        1.0
                    } else {
                        style.description_opacity
                    },
                );
                ctx.move_to(cur_x, label_y);
                ctx.show_text(degraded_label)?;
            }
        }
        ctx.restore()?;
        ctx.target().flush();
        drop(ctx);
        surface.flush();
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Executes a drawing operation with a temporary Cairo operator setting.
    #[inline]
    fn with_operator<F>(&self, ctx: &CairoContext, operator: cairo::Operator, f: F) -> Result<()>
    where
        F: FnOnce(&CairoContext) -> Result<()>,
    {
        ctx.set_operator(operator);
        f(ctx)?;
        ctx.set_operator(cairo::Operator::Over);
        Ok(())
    }

    /// Clears a rectangular area in the dimming layer with optional rounded corners.
    fn clear_area(&self, ctx: &CairoContext, rect: Rect) -> Result<()> {
        let radius = self.config.border_radius as f64;
        let (x, y, w, h) = rect.as_f64_tuple();

        if radius > 0.0 {
            self.draw_rounded_rectangle(ctx, x, y, w, h, radius)?;
        } else {
            ctx.rectangle(x, y, w, h);
        }
        ctx.fill()?;
        Ok(())
    }

    /// Renders the selection overlay directly to the provided buffer with zero-copy optimization.
    pub fn render_to_buffer(
        &self,
        selection: &Selection,
        buffer: &mut [u8],
        frozen: Option<FrozenFrame<'_>>,
        hud: SelectionHud,
    ) -> Result<()> {
        let stride = self.width * 4;

        // Check if we have a selection to avoid dimming that area
        let selection_rect = selection.get_rect().filter(|r| r.width > 0 && r.height > 0);

        // Step 1: Complete frozen buffer copy ENTIRELY before creating Cairo surface.
        //
        // A pre-dimmed copy of the frozen screen turns the dim into two rectangle
        // copies — the dimmed screen, then the selected region restored from the
        // undimmed original — instead of a full-screen alpha blend on every frame.
        let (has_frozen, dim_is_baked) = match frozen {
            Some(frame) => {
                blit(
                    buffer,
                    stride as usize,
                    frame.dimmed.unwrap_or(frame.pixels),
                    frame.stride as usize,
                    Rect::new(0, 0, self.width, self.height),
                );

                if frame.dimmed.is_some() {
                    if let Some(rect) = selection_rect.and_then(|r| self.clamp_to_surface(r)) {
                        blit(
                            buffer,
                            stride as usize,
                            frame.pixels,
                            frame.stride as usize,
                            rect,
                        );
                    }
                }

                (true, frame.dimmed.is_some())
            }
            None => (false, false),
        };

        // Step 2: Ensure ALL memcpy operations are complete with compiler barrier
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);

        // Step 3: Now create Cairo surface - buffer is guaranteed complete
        // SAFETY: The buffer's lifetime is tied to the surface's usage within this function.
        // Buffer is fully populated at this point.
        let surface = unsafe {
            ImageSurface::create_for_data_unsafe(
                buffer.as_mut_ptr(),
                Format::ARgb32,
                self.width,
                self.height,
                stride,
            )?
        };

        let ctx = CairoContext::new(&surface).context("Failed to create Cairo context")?;

        if has_frozen && dim_is_baked {
            // The dim is already in the pixels; only the border and label remain.
        } else if has_frozen {
            if let Some(rect) = selection_rect {
                ctx.save()?;

                ctx.rectangle(0.0, 0.0, self.width as f64, self.height as f64);
                let (x, y, w, h) = rect.as_f64_tuple();
                ctx.rectangle(x, y, w, h);
                ctx.set_fill_rule(cairo::FillRule::EvenOdd);
                ctx.clip();

                ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                ctx.set_operator(cairo::Operator::Over);
                ctx.paint()?;

                ctx.restore()?;

                log::debug!(
                    "Renderer drawing selection rect {} on surface {}x{} with frozen content",
                    rect.describe(),
                    self.width,
                    self.height
                );
            } else {
                self.with_operator(&ctx, cairo::Operator::Over, |ctx| {
                    ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                    ctx.paint()?;
                    Ok(())
                })?;
            }
        } else {
            if let Some(rect) = selection_rect {
                self.with_operator(&ctx, cairo::Operator::Source, |ctx| {
                    ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                    ctx.paint()?;
                    Ok(())
                })?;

                self.with_operator(&ctx, cairo::Operator::Clear, |ctx| {
                    self.clear_area(ctx, rect)
                })?;

                log::debug!(
                    "Renderer drawing selection rect {} on surface {}x{}",
                    rect.describe(),
                    self.width,
                    self.height
                );
            } else {
                self.with_operator(&ctx, cairo::Operator::Source, |ctx| {
                    ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                    ctx.paint()?;
                    Ok(())
                })?;
            }
        }

        if let Some(rect) = selection.get_rect() {
            self.draw_selection_border(&ctx, rect)?;
        }
        if hud.visible {
            self.draw_selection_hud(&ctx, hud)?;
        }

        // Ensure all drawing operations are complete and flushed to the buffer
        // This is critical to prevent tearing
        ctx.target().flush();
        drop(ctx);
        surface.flush();

        // Force synchronization point - ensure all operations completed
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    fn draw_rounded_rectangle(
        &self,
        ctx: &CairoContext,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
    ) -> Result<()> {
        use std::f64::consts::PI;

        // Clamp radius to half the smallest dimension
        let radius = radius.min(width / 2.0).min(height / 2.0);

        // Start at top-left, just after the corner
        ctx.new_path();
        ctx.arc(x + radius, y + radius, radius, PI, 3.0 * PI / 2.0); // Top-left corner
        ctx.arc(
            x + width - radius,
            y + radius,
            radius,
            3.0 * PI / 2.0,
            2.0 * PI,
        ); // Top-right corner
        ctx.arc(
            x + width - radius,
            y + height - radius,
            radius,
            0.0,
            PI / 2.0,
        ); // Bottom-right corner
        ctx.arc(x + radius, y + height - radius, radius, PI / 2.0, PI); // Bottom-left corner
        ctx.close_path();

        Ok(())
    }

    /// Clips a rectangle to the surface, returning `None` if nothing is left.
    ///
    /// A selection can start on a neighbouring monitor, so its coordinates in this
    /// surface can be negative or run past the far edge.
    fn clamp_to_surface(&self, rect: Rect) -> Option<Rect> {
        let left = rect.x.max(0);
        let top = rect.y.max(0);
        let right = (rect.x + rect.width).min(self.width);
        let bottom = (rect.y + rect.height).min(self.height);

        (right > left && bottom > top).then(|| Rect::new(left, top, right - left, bottom - top))
    }

    fn draw_selection_border(&self, ctx: &CairoContext, rect: Rect) -> Result<()> {
        let weight = self.config.border_weight as f64;
        let radius = self.config.border_radius as f64;

        ctx.set_source_rgba(
            self.config.border_color.r,
            self.config.border_color.g,
            self.config.border_color.b,
            self.config.border_color.a,
        );
        ctx.set_line_width(weight);

        let (x, y, w, h) = rect.as_f64_tuple();
        if radius > 0.0 {
            self.draw_rounded_rectangle(ctx, x, y, w, h, radius)?;
        } else {
            ctx.rectangle(x, y, w, h);
        }
        ctx.stroke()?;

        if rect.width <= MIN_TEXT_WIDTH || rect.height <= MIN_TEXT_HEIGHT {
            return Ok(());
        }

        let dimensions_changed = self
            .cached_text
            .borrow()
            .as_ref()
            .is_none_or(|(width, height, _)| *width != rect.width || *height != rect.height);
        if dimensions_changed {
            *self.cached_text.borrow_mut() = Some((
                rect.width,
                rect.height,
                format!("{}×{}", rect.width, rect.height),
            ));
        }
        let cached = self.cached_text.borrow();
        let text = &cached.as_ref().expect("dimension text was initialized").2;

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            self.config.font_weight,
        );
        ctx.set_font_size(self.config.font_size);
        let extents = ctx.text_extents(text)?;
        let font = ctx.font_extents()?;
        let padding_x = 10.0;
        let padding_y = 6.0;
        let pill = dimension_label_rect(
            rect,
            extents.x_advance() + padding_x * 2.0,
            font.height() + padding_y * 2.0,
            self.width,
            self.height,
        );

        self.draw_rounded_rectangle(ctx, pill.x, pill.y, pill.width, pill.height, 7.0)?;
        ctx.set_source_rgba(
            self.config.panel_background_color.r,
            self.config.panel_background_color.g,
            self.config.panel_background_color.b,
            0.92,
        );
        ctx.fill_preserve()?;
        ctx.set_source_rgba(
            self.config.border_color.r,
            self.config.border_color.g,
            self.config.border_color.b,
            0.72,
        );
        ctx.set_line_width(1.0);
        ctx.stroke()?;

        let text_x = pill.x + (pill.width - extents.x_advance()) / 2.0;
        let text_y = pill.y + (pill.height + font.ascent() - font.descent()) / 2.0;
        ctx.set_source_rgb(
            self.config.text_color.r,
            self.config.text_color.g,
            self.config.text_color.b,
        );
        ctx.move_to(text_x, text_y);
        ctx.show_text(text)?;

        Ok(())
    }

    fn draw_selection_hud(&self, ctx: &CairoContext, hud: SelectionHud) -> Result<()> {
        let primary = match (hud.purpose, hud.to_clipboard) {
            (SelectionPurpose::Screenshot, true) => "Screenshot area · Copy to clipboard",
            (SelectionPurpose::Screenshot, false) => "Screenshot area",
            (SelectionPurpose::Recording, _) => "Record area",
            (SelectionPurpose::Ocr, _) => "OCR · Copy text to clipboard",
        };
        let guidance = if self.width < 560 {
            "Drag to select · Esc to cancel"
        } else {
            "Drag to select · Right-click or Esc to cancel"
        };

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            self.config.font_weight,
        );
        let primary_size = (self.config.font_size * 0.88).max(13.0);
        ctx.set_font_size(primary_size);
        let primary_width = ctx.text_extents(primary)?.x_advance();

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
        );
        let guidance_size = (self.config.font_size * 0.72).max(11.0);
        ctx.set_font_size(guidance_size);
        let guidance_width = ctx.text_extents(guidance)?.x_advance();

        let panel_width =
            (primary_width.max(guidance_width) + 32.0).min((f64::from(self.width) - 24.0).max(1.0));
        let panel_height = 58.0;
        let panel_x = (f64::from(self.width) - panel_width) / 2.0;
        let panel_y = 16.0;
        self.draw_rounded_rectangle(ctx, panel_x, panel_y, panel_width, panel_height, 12.0)?;
        ctx.set_source_rgba(
            self.config.panel_background_color.r,
            self.config.panel_background_color.g,
            self.config.panel_background_color.b,
            0.90,
        );
        ctx.fill_preserve()?;
        ctx.set_source_rgba(
            self.config.border_color.r,
            self.config.border_color.g,
            self.config.border_color.b,
            0.42,
        );
        ctx.set_line_width(1.0);
        ctx.stroke()?;

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            self.config.font_weight,
        );
        ctx.set_font_size(primary_size);
        ctx.set_source_rgba(
            self.config.border_color.r,
            self.config.border_color.g,
            self.config.border_color.b,
            1.0,
        );
        ctx.move_to(
            panel_x + (panel_width - primary_width) / 2.0,
            panel_y + 23.0,
        );
        ctx.show_text(primary)?;

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
        );
        ctx.set_font_size(guidance_size);
        ctx.set_source_rgba(
            self.config.text_color.r,
            self.config.text_color.g,
            self.config.text_color.b,
            0.76,
        );
        ctx.move_to(
            panel_x + (panel_width - guidance_width) / 2.0,
            panel_y + 44.0,
        );
        ctx.show_text(guidance)?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "cairo_test.rs"]
mod tests;
