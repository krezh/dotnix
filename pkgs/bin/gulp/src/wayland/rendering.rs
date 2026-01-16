//! Rendering logic for output surfaces

use anyhow::Result;
use wayland_client::protocol::wl_shm;

use crate::selection::{Rect, Selection};

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
) -> Result<()> {
    if !output_surface.configured {
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

            // Render directly to buffer
            renderer.render_to_buffer(&local_selection, canvas)?;
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
        let snap_rect = selection.get_current_snap_target();

        if let Some(snap_rect) = snap_rect {
            let local_snap = translate_rect_to_local(snap_rect, offset_x, offset_y);

            log::debug!(
                "RENDERING SNAP TARGET: global ({},{}) {}x{} -> local ({},{}) {}x{}",
                snap_rect.x, snap_rect.y, snap_rect.width, snap_rect.height,
                local_snap.x, local_snap.y, local_snap.width, local_snap.height
            );

            let mut local_selection = Selection::new();
            local_selection.set_animated_snap_target(Some(local_snap));

            renderer.render_to_buffer(&local_selection, canvas)?;
        } else {
            // Render dimmed overlay
            log::debug!("RENDERING DIMMED ONLY");
            let empty_selection = Selection::new();
            renderer.render_to_buffer(&empty_selection, canvas)?;
        }
    }

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
