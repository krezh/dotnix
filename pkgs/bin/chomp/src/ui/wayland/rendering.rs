//! Rendering logic for output surfaces

use anyhow::Result;
use wayland_client::protocol::wl_shm;

use crate::render::{Rect, Selection};

use super::output::OutputSurface;

/// Translates global rectangle coordinates to local output coordinates.
pub fn translate_rect_to_local(rect: Rect, offset_x: i32, offset_y: i32) -> Rect {
    Rect::new(
        rect.x - offset_x,
        rect.y - offset_y,
        rect.width,
        rect.height,
    )
}

/// Creates a local selection from a global rectangle by translating coordinates.
pub fn create_local_selection(global_rect: Rect, offset_x: i32, offset_y: i32) -> Selection {
    let local_rect = translate_rect_to_local(global_rect, offset_x, offset_y);
    Selection::from_rect(local_rect)
}

/// Renders the current selection state to a specific output surface.
pub fn draw_output(
    output_surface: &mut OutputSurface,
    selection: &Selection,
    qh: &wayland_client::QueueHandle<super::App>,
) -> Result<()> {
    if !output_surface.configured {
        return Ok(());
    }

    // Skip if waiting for frame callback (vsync synchronization)
    if output_surface.waiting_for_frame {
        return Ok(());
    }

    let width = output_surface.width as i32;
    let height = output_surface.height as i32;
    let offset_x = output_surface.x;
    let offset_y = output_surface.y;
    let stride = width * 4;

    // Check if we have renderer and pool
    if output_surface.renderer.is_none() || output_surface.pool.is_none() {
        return Ok(());
    }

    let renderer = output_surface.renderer.as_ref().unwrap();
    let pool = output_surface.pool.as_mut().unwrap();

    // Use double buffering to prevent flickering
    let (buffer, canvas) = match pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)
    {
        Ok(buffer) => buffer,
        Err(e) => {
            log::warn!(
                "Failed to create buffer: {}. Resizing pool.",
                e
            );
            // Pool might be exhausted, resize it
            pool.resize((width * height * 4 * 2) as usize)?;
            pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)?
        }
    };

    log::debug!("Got buffer, canvas ptr: {:p}", canvas.as_ptr());

    // Determine if this output currently has a selection
    let output_rect = Rect::new(offset_x, offset_y, width, height);
    let has_selection_now = selection.get_rect()
        .map(|rect| rect.intersects(&output_rect))
        .unwrap_or(false);

    // Skip rendering ONLY if:
    // 1. Not first render AND
    // 2. State hasn't changed AND
    // 3. Currently has NO selection (if it has selection, always render as rect changes)
    let skip_render = !output_surface.needs_render
        && has_selection_now == output_surface.last_had_selection
        && !has_selection_now;

    if skip_render {
        return Ok(());
    }

    // Update state
    output_surface.last_had_selection = has_selection_now;
    output_surface.needs_render = false;

    // Check if we need to render a selection on this output
    let has_selection = if let Some(rect) = selection.get_rect() {
        let output_rect = Rect::new(offset_x, offset_y, width, height);

        if rect.intersects(&output_rect) {
            // Calculate the intersection rectangle (clipped to this output)
            let clip_x = rect.x.max(offset_x);
            let clip_y = rect.y.max(offset_y);
            let clip_right = (rect.x + rect.width).min(offset_x + width);
            let clip_bottom = (rect.y + rect.height).min(offset_y + height);
            let clip_width = clip_right - clip_x;
            let clip_height = clip_bottom - clip_y;

            log::debug!(
                "RENDERING SELECTION at ({},{}) {}x{} - global rect: ({},{}) {}x{}, clipped: ({},{}) {}x{}",
                offset_x,
                offset_y,
                width,
                height,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                clip_x,
                clip_y,
                clip_width,
                clip_height
            );

            // Translate the ENTIRE global selection to output-local coordinates
            let local_rect_x = rect.x - offset_x;
            let local_rect_y = rect.y - offset_y;

            log::debug!(
                "Creating local selection: global rect ({},{}) {}x{} -> local rect ({},{}) {}x{}",
                rect.x, rect.y, rect.width, rect.height,
                local_rect_x, local_rect_y, rect.width, rect.height
            );

            // Create a selection with the full rectangle translated to local coords
            let local_selection = create_local_selection(rect, offset_x, offset_y);

            // Get frozen buffer data and stride if available
            let frozen_data = output_surface.frozen_buffer.as_ref()
                .map(|img| (img.data.as_slice(), img.stride as i32));

            // Render directly to buffer
            renderer.render_to_buffer(&local_selection, canvas, frozen_data)?;
            true
        } else {
            log::debug!("SKIPPING - no intersection");
            false
        }
    } else {
        false
    };

    // If no selection on this output, render dimmed overlay only (or snap target if present)
    if !has_selection {
        // Render dimmed overlay without selection
        log::debug!("RENDERING DIMMED ONLY");
        let empty_selection = Selection::new();
        let frozen_data = output_surface.frozen_buffer.as_ref()
            .map(|img| (img.data.as_slice(), img.stride as i32));
        renderer.render_to_buffer(&empty_selection, canvas, frozen_data)?;
    }

    // Ensure all rendering is complete before commit (prevents tearing)
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

    // Request frame callback for vsync synchronization
    let callback = output_surface.surface.frame(qh, ());
    output_surface.frame_callback = Some(callback);
    output_surface.waiting_for_frame = true;

    // Attach and commit
    log::debug!(
        "Committing buffer to surface at offset ({},{}) with damage {}x{}",
        offset_x,
        offset_y,
        width,
        height
    );
    output_surface
        .surface
        .attach(Some(buffer.wl_buffer()), 0, 0);
    output_surface.surface.damage_buffer(0, 0, width, height);
    output_surface.surface.commit();

    Ok(())
}
