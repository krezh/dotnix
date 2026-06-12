use anyhow::{Context, Result};
use cairo::{Context as CairoContext, Format, ImageSurface};
use std::cell::RefCell;

use super::selection::{Rect, Selection};
use crate::config::FontWeight;

// Text display thresholds
const MIN_TEXT_WIDTH: i32 = 80;
const MIN_TEXT_HEIGHT: i32 = 40;

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
        border_weight: u32,
        border_radius: u32,
        dim_opacity: f64,
        font_family: String,
        font_size: u32,
        font_weight: FontWeight,
    ) -> Result<Self> {
        let border_color = Color::from_hex(border_color)?;

        Ok(Self {
            border_color,
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
            border_color: Color::from_hex("#FFFFFF").unwrap(),
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

    /// Renders the mode selector as a full-width bottom bar.
    pub fn render_mode_select(
        &self,
        buffer: &mut [u8],
        _frozen_buffer: Option<(&[u8], i32)>,
        keybinds: &crate::config::KeybindsConfig,
        style: &crate::config::ModeSelectConfig,
        is_recording: bool,
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

        // Transparent background — compositor content shows through outside the bar
        ctx.set_operator(cairo::Operator::Source);
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        ctx.paint()?;
        ctx.set_operator(cairo::Operator::Over);

        let bc = &self.config.border_color;
        let screen_w = self.width as f64;
        let screen_h = self.height as f64;

        // Resolve configurable colors, falling back gracefully on parse error
        let bg_color = Color::from_hex(&style.background_color).unwrap_or(Color { r: 0.05, g: 0.05, b: 0.08, a: 1.0 });
        let desc_color = Color::from_hex(&style.description_color).unwrap_or(Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
        let key_color = if style.key_color.is_empty() {
            *bc
        } else {
            Color::from_hex(&style.key_color).unwrap_or(*bc)
        };
        let dot_color = Color::from_hex(&style.recording_dot_color).unwrap_or(Color { r: 0.95, g: 0.25, b: 0.25, a: 1.0 });
        let rec_color = Color::from_hex(&style.recording_highlight_color).unwrap_or(Color { r: 0.95, g: 0.75, b: 0.20, a: 1.0 });

        let bar_h = style.bar_height as f64;
        let bar_y = screen_h - bar_h;

        // Bar background
        ctx.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, style.background_opacity);
        ctx.rectangle(0.0, bar_y, screen_w, bar_h);
        ctx.fill()?;

        // Top border
        ctx.set_source_rgba(bc.r, bc.g, bc.b, style.border_opacity);
        ctx.set_line_width(1.0);
        ctx.move_to(0.0, bar_y + 0.5);
        ctx.line_to(screen_w, bar_y + 0.5);
        ctx.stroke()?;

        ctx.select_font_face(
            &self.config.font_family,
            cairo::FontSlant::Normal,
            self.config.font_weight,
        );
        ctx.set_font_size(self.config.font_size);

        // Groups: each is a slice of (key_label, desc)
        let ss = [
            (format!("[{}]", keybinds.screenshot_area),   "Area"),
            (format!("[{}]", keybinds.screenshot_screen), "Screen"),
            (format!("[{}]", keybinds.screenshot_window), "Window"),
            (format!("[{}]", keybinds.ocr),               "OCR"),
        ];
        let rec = [
            (format!("[{}]", keybinds.record_area),   "Area"),
            (format!("[{}]", keybinds.record_screen), "Screen"),
            (format!("[{}]", keybinds.record_window), "Window"),
        ];
        let stop = [(format!("[{}]", keybinds.stop_recording), "Stop recording")];
        let quit = [("[Esc]".to_string(), "Quit")];

        // Show stop-recording group only when a recording is active
        let groups: &[&[(String, &str)]] = if is_recording {
            &[&ss, &rec, &stop, &quit]
        } else {
            &[&ss, &rec, &quit]
        };

        let key_desc_gap = 8.0_f64;
        let entry_gap    = 26.0_f64;
        let sep_pad      = 30.0_f64;
        // Dot drawn as a filled arc — font_size * 0.3 radius, plus gap after
        let dot_r   = self.config.font_size * 0.30;
        let dot_gap = if is_recording { dot_r * 2.0 + 10.0 } else { 0.0 };

        // Measure total content width
        let entry_width = |key: &str, desc: &str| -> f64 {
            let kw = ctx.text_extents(key).map(|e| e.width()).unwrap_or(0.0);
            let dw = ctx.text_extents(desc).map(|e| e.width()).unwrap_or(0.0);
            kw + key_desc_gap + dw
        };

        let mut total_w = 0.0_f64;
        for (g, group) in groups.iter().enumerate() {
            for (e, (key, desc)) in group.iter().enumerate() {
                // The stop group gets the dot prefix added to its first entry
                let extra = if is_recording && g == 2 && e == 0 { dot_gap } else { 0.0 };
                total_w += extra + entry_width(key, desc);
                if e + 1 < group.len() {
                    total_w += entry_gap;
                }
            }
            if g + 1 < groups.len() {
                total_w += sep_pad * 2.0 + 1.0;
            }
        }

        // Vertically center text in bar (use font ascent as baseline offset)
        let fe = ctx.font_extents()?;
        let text_y = bar_y + (bar_h + fe.ascent() - fe.descent()) / 2.0;

        let mut x = (screen_w - total_w) / 2.0;

        for (g, group) in groups.iter().enumerate() {
            // Red dot + "Stop recording" label for the stop group
            let is_stop_group = is_recording && g == 2;

            for (e, (key, desc)) in group.iter().enumerate() {
                // Recording indicator dot before the stop entry's key
                if is_stop_group && e == 0 {
                    let dot_cx = x + dot_r;
                    let dot_cy = text_y - fe.ascent() * 0.35;
                    ctx.set_source_rgba(dot_color.r, dot_color.g, dot_color.b, 1.0);
                    ctx.arc(dot_cx, dot_cy, dot_r, 0.0, std::f64::consts::TAU);
                    ctx.fill()?;
                    x += dot_gap;
                }

                // Key label
                if is_stop_group {
                    ctx.set_source_rgba(rec_color.r, rec_color.g, rec_color.b, 1.0);
                } else {
                    ctx.set_source_rgba(key_color.r, key_color.g, key_color.b, 1.0);
                }
                ctx.move_to(x, text_y);
                ctx.show_text(key)?;
                let kw = ctx.text_extents(key)?.width();
                x += kw + key_desc_gap;

                // Description
                if is_stop_group {
                    ctx.set_source_rgba(rec_color.r, rec_color.g, rec_color.b, 0.90);
                } else {
                    ctx.set_source_rgba(desc_color.r, desc_color.g, desc_color.b, style.description_opacity);
                }
                ctx.move_to(x, text_y);
                ctx.show_text(desc)?;
                let dw = ctx.text_extents(desc)?.width();
                x += dw;

                if e + 1 < group.len() {
                    x += entry_gap;
                }
            }

            // Group separator
            if g + 1 < groups.len() {
                x += sep_pad;
                let sep_h = bar_h * 0.45;
                let sep_y = bar_y + (bar_h - sep_h) / 2.0;
                ctx.set_source_rgba(1.0, 1.0, 1.0, style.separator_opacity);
                ctx.set_line_width(1.0);
                ctx.move_to(x + 0.5, sep_y);
                ctx.line_to(x + 0.5, sep_y + sep_h);
                ctx.stroke()?;
                x += 1.0 + sep_pad;
            }
        }

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
        frozen_buffer: Option<(&[u8], i32)>,
    ) -> Result<()> {
        let stride = self.width * 4;

        // Step 1: Complete frozen buffer copy ENTIRELY before creating Cairo surface
        let has_frozen = if let Some((frozen_data, frozen_stride)) = frozen_buffer {
            // Fast copy: handle stride differences efficiently
            if frozen_stride == stride {
                // Strides match - single memcpy with explicit completion
                let copy_len = buffer.len().min(frozen_data.len());
                // Use chunks to ensure completion (prevents compiler optimizations that might reorder)
                for (dst_chunk, src_chunk) in buffer[..copy_len]
                    .chunks_mut(4096)
                    .zip(frozen_data[..copy_len].chunks(4096))
                {
                    dst_chunk[..src_chunk.len()].copy_from_slice(src_chunk);
                }
            } else {
                // Strides differ - copy row by row with explicit completion
                let row_bytes = (self.width * 4) as usize;
                for y in 0..self.height as usize {
                    let dst_offset = y * stride as usize;
                    let src_offset = y * frozen_stride as usize;
                    if dst_offset + row_bytes <= buffer.len()
                        && src_offset + row_bytes <= frozen_data.len()
                    {
                        // Copy in chunks for explicit completion
                        let dst_row = &mut buffer[dst_offset..dst_offset + row_bytes];
                        let src_row = &frozen_data[src_offset..src_offset + row_bytes];
                        for (dst_chunk, src_chunk) in
                            dst_row.chunks_mut(4096).zip(src_row.chunks(4096))
                        {
                            dst_chunk[..src_chunk.len()].copy_from_slice(src_chunk);
                        }
                    }
                }
            }
            true
        } else {
            false
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

        // Check if we have a selection to avoid dimming that area
        let selection_rect = selection.get_rect().filter(|r| r.width > 0 && r.height > 0);

        if has_frozen {
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

        if rect.width > MIN_TEXT_WIDTH && rect.height > MIN_TEXT_HEIGHT {
            // Check if we need to regenerate the cached text
            let text = {
                let cached = self.cached_text.borrow();
                let dimensions_changed = match cached.as_ref() {
                    None => true,
                    Some((w, h, _)) => *w != rect.width || *h != rect.height,
                };

                if dimensions_changed {
                    drop(cached);
                    let text = format!("{}×{}", rect.width, rect.height);
                    *self.cached_text.borrow_mut() = Some((rect.width, rect.height, text.clone()));
                    text
                } else {
                    cached.as_ref().unwrap().2.clone()
                }
            };

            ctx.select_font_face(
                &self.config.font_family,
                cairo::FontSlant::Normal,
                self.config.font_weight,
            );
            ctx.set_font_size(self.config.font_size);

            let extents = ctx.text_extents(&text)?;
            let text_x = x + (w - extents.width()) / 2.0;
            let text_y = y + (h + extents.height()) / 2.0;

            ctx.fill()?;

            // Text
            ctx.set_source_rgb(1.0, 1.0, 1.0);
            ctx.move_to(text_x, text_y);
            ctx.show_text(&text)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF8800").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.533).abs() < 0.01);
        assert!((color.b - 0.0).abs() < 0.01);

        let color = Color::from_hex("#FFF").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 1.0).abs() < 0.01);
        assert!((color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_renderer_creation() {
        let config = RenderConfig::new(
            "#FFFFFF",
            2,
            0,
            0.5,
            "Inter Nerd Font".to_string(),
            18,
            FontWeight::Bold,
        )
        .unwrap();
        let renderer = Renderer::new(1920, 1080, config);
        assert_eq!(renderer.width, 1920);
        assert_eq!(renderer.height, 1080);
    }
}
