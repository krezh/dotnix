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
    pub fn render_to_buffer(&self, selection: &Selection, buffer: &mut [u8], frozen_buffer: Option<(&[u8], i32)>) -> Result<()> {
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
                        && src_offset + row_bytes <= frozen_data.len() {
                        // Copy in chunks for explicit completion
                        let dst_row = &mut buffer[dst_offset..dst_offset + row_bytes];
                        let src_row = &frozen_data[src_offset..src_offset + row_bytes];
                        for (dst_chunk, src_chunk) in dst_row.chunks_mut(4096).zip(src_row.chunks(4096)) {
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
            // Apply dimming over the frozen image, excluding selection area
            if let Some(rect) = selection_rect {
                // Save context state
                ctx.save()?;

                // Create inverse clip: dim everything EXCEPT the selection
                ctx.rectangle(0.0, 0.0, self.width as f64, self.height as f64);
                let (x, y, w, h) = rect.as_f64_tuple();
                ctx.rectangle(x, y, w, h);
                ctx.set_fill_rule(cairo::FillRule::EvenOdd);
                ctx.clip();

                // Apply dimming only to non-selection area
                ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                ctx.set_operator(cairo::Operator::Over);
                ctx.paint()?;

                // Restore context
                ctx.restore()?;

                log::debug!(
                    "Renderer drawing selection rect {} on surface {}x{} with frozen content",
                    rect.describe(),
                    self.width,
                    self.height
                );
            } else {
                // No selection - dim entire surface
                self.with_operator(&ctx, cairo::Operator::Over, |ctx| {
                    ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                    ctx.paint()?;
                    Ok(())
                })?;
            }
        } else {
            // No frozen buffer - use original transparent dimming approach
            if let Some(rect) = selection_rect {
                // Paint dimmed background
                self.with_operator(&ctx, cairo::Operator::Source, |ctx| {
                    ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                    ctx.paint()?;
                    Ok(())
                })?;

                // Clear selection area to transparency
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
                // No selection - paint dimmed background
                self.with_operator(&ctx, cairo::Operator::Source, |ctx| {
                    ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
                    ctx.paint()?;
                    Ok(())
                })?;
            }
        }

        // Draw selection rectangle border (if selecting)
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
