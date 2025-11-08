use anyhow::{Context, Result};
use cairo::{Context as CairoContext, Format, ImageSurface};

use crate::selection::Rect;
use crate::selection::Selection;

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
}

impl Renderer {
    pub fn new(
        width: i32,
        height: i32,
        border_color: &str,
        border_weight: u32,
        border_radius: u32,
        dim_opacity: f64,
        font_family: String,
        font_size: u32,
        font_weight: &str,
    ) -> Result<Self> {
        let border_color = Color::from_hex(border_color)?;

        let font_weight = match font_weight {
            "Normal" => cairo::FontWeight::Normal,
            "Bold" => cairo::FontWeight::Bold,
            _ => cairo::FontWeight::Bold,
        };

        Ok(Self {
            config: RenderConfig {
                border_color,
                border_weight,
                border_radius,
                dim_opacity,
                font_family,
                font_size: font_size as f64,
                font_weight,
            },
            width,
            height,
        })
    }

    /// Helper to execute operation with temporary operator setting
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

    /// Clear area (punch hole in dimming) with optional rounded corners
    fn clear_area(&self, ctx: &CairoContext, rect: Rect) -> Result<()> {
        let radius = self.config.border_radius as f64;
        if radius > 0.0 {
            self.draw_rounded_rectangle(
                ctx,
                rect.x as f64,
                rect.y as f64,
                rect.width as f64,
                rect.height as f64,
                radius,
            )?;
        } else {
            ctx.rectangle(
                rect.x as f64,
                rect.y as f64,
                rect.width as f64,
                rect.height as f64,
            );
        }
        ctx.fill()?;
        Ok(())
    }

    /// Render directly to provided buffer - optimized for speed
    pub fn render_to_buffer(&self, selection: &Selection, buffer: &mut [u8]) -> Result<()> {
        let stride = self.width * 4;

        // Create Cairo surface wrapping the existing buffer (NO allocation!)
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

        // Paint dimmed background - use Source operator to replace buffer contents
        self.with_operator(&ctx, cairo::Operator::Source, |ctx| {
            ctx.set_source_rgba(0.0, 0.0, 0.0, self.config.dim_opacity);
            ctx.paint()?;
            Ok(())
        })?;

        // Clear the selection area (punch hole in dimming) - only when user is selecting
        if let Some(rect) = selection.get_rect() {
            if rect.width > 0 && rect.height > 0 {
                log::debug!(
                    "Renderer drawing selection rect at ({},{}) size {}x{} on surface {}x{}",
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    self.width,
                    self.height
                );
                self.with_operator(&ctx, cairo::Operator::Clear, |ctx| {
                    self.clear_area(ctx, rect)
                })?;
            }
        }

        // Clear dimming and draw snap target preview (when hovering, not selecting)
        if selection.get_rect().is_none() {
            // Use animated snap target if available, otherwise fall back to static
            if let Some(snap_rect) = selection
                .get_animated_snap_target()
                .or_else(|| selection.get_snap_target())
            {
                // Clear the snap target area (punch hole in dimming)
                self.with_operator(&ctx, cairo::Operator::Clear, |ctx| {
                    self.clear_area(ctx, snap_rect)
                })?;

                // Draw the border
                self.draw_snap_target(&ctx, snap_rect)?;
            }
        }

        // Draw selection rectangle border (if selecting)
        if let Some(rect) = selection.get_rect() {
            self.draw_selection_border(&ctx, rect)?;
        }

        // Ensure all drawing is flushed to the buffer
        drop(ctx);
        surface.flush();

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

    fn draw_snap_target(&self, ctx: &CairoContext, rect: Rect) -> Result<()> {
        // Draw snap target with same style as selection border
        self.draw_selection_border(ctx, rect)?;
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

        if radius > 0.0 {
            // Draw rounded rectangle
            self.draw_rounded_rectangle(
                ctx,
                rect.x as f64,
                rect.y as f64,
                rect.width as f64,
                rect.height as f64,
                radius,
            )?;
        } else {
            // Draw regular rectangle
            ctx.rectangle(
                rect.x as f64,
                rect.y as f64,
                rect.width as f64,
                rect.height as f64,
            );
        }
        ctx.stroke()?;

        // Draw dimensions text if big enough
        if rect.width > 80 && rect.height > 40 {
            let text = format!("{}×{}", rect.width, rect.height);
            ctx.select_font_face(
                &self.config.font_family,
                cairo::FontSlant::Normal,
                self.config.font_weight,
            );
            ctx.set_font_size(self.config.font_size);

            let extents = ctx.text_extents(&text)?;
            let text_x = rect.x as f64 + (rect.width as f64 - extents.width()) / 2.0;
            let text_y = rect.y as f64 + (rect.height as f64 + extents.height()) / 2.0;

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
        let renderer = Renderer::new(
            1920,
            1080,
            "#FFFFFF",
            2,
            0,
            0.5,
            "Inter Nerd Font".to_string(),
            18,
            "Bold",
        );
        assert!(renderer.is_ok());
    }
}
