//! Wayland output enumeration and geometry queries

use anyhow::{Context, Result};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::{delegate_output, delegate_registry};
use wayland_client::{globals::registry_queue_init, protocol::wl_output, Connection};

use crate::render::selection::Rect;

/// Wayland output information: (output, name, x, y, width, height).
pub type OutputInfo = (wl_output::WlOutput, String, i32, i32, u32, u32);

/// Gets list of Wayland outputs with their geometry.
///
/// Returns a list of (output, name, x, y, width, height) tuples.
pub fn get_outputs(conn: &Connection) -> Result<Vec<OutputInfo>> {
    // Minimal state for output enumeration
    struct OutputEnumerator {
        registry_state: RegistryState,
        output_state: OutputState,
    }

    impl OutputHandler for OutputEnumerator {
        fn output_state(&mut self) -> &mut OutputState {
            &mut self.output_state
        }

        fn new_output(
            &mut self,
            _conn: &Connection,
            _qh: &wayland_client::QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
        }

        fn update_output(
            &mut self,
            _conn: &Connection,
            _qh: &wayland_client::QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
        }

        fn output_destroyed(
            &mut self,
            _conn: &Connection,
            _qh: &wayland_client::QueueHandle<Self>,
            _output: wl_output::WlOutput,
        ) {
        }
    }

    impl ProvidesRegistryState for OutputEnumerator {
        fn registry(&mut self) -> &mut RegistryState {
            &mut self.registry_state
        }

        smithay_client_toolkit::registry_handlers![OutputState];
    }

    delegate_output!(OutputEnumerator);
    delegate_registry!(OutputEnumerator);

    let (globals, mut event_queue) = registry_queue_init::<OutputEnumerator>(conn)
        .context("Failed to init registry")?;

    let registry_state = RegistryState::new(&globals);
    let output_state = OutputState::new(&globals, &event_queue.handle());

    let mut state = OutputEnumerator {
        registry_state,
        output_state,
    };

    // Dispatch events to populate output info
    event_queue
        .roundtrip(&mut state)
        .context("Failed to roundtrip")?;

    let outputs: Vec<_> = state
        .output_state
        .outputs()
        .filter_map(|output| {
            state.output_state.info(&output).map(|info| {
                let name = info.name.clone().unwrap_or_default();
                let (x, y) = info.logical_position.unwrap_or((0, 0));
                let (width, height) = info.logical_size.unwrap_or((1920, 1080));
                (output.clone(), name, x, y, width as u32, height as u32)
            })
        })
        .collect();

    if outputs.is_empty() {
        anyhow::bail!("No outputs found");
    }

    Ok(outputs)
}

/// Finds the output containing a given rectangle and returns local coordinates.
///
/// Searches through the outputs to find which one intersects with the given rectangle,
/// then translates the rectangle's coordinates from global (multi-monitor) space to
/// local (single output) space.
///
/// Returns a tuple of (output, local_rect) where local_rect has coordinates relative
/// to the output's top-left corner.
pub fn find_output_for_rect(
    outputs: &[OutputInfo],
    rect: Rect,
) -> Result<(&wl_output::WlOutput, Rect)> {
    let (output, _, offset_x, offset_y, _, _) = outputs
        .iter()
        .find(|(_, _, x, y, w, h)| {
            let output_rect = Rect::new(*x, *y, *w as i32, *h as i32);
            rect.intersects(&output_rect)
        })
        .context("Selection is not on any output")?;

    let local_rect = Rect::new(
        rect.x - offset_x,
        rect.y - offset_y,
        rect.width,
        rect.height,
    );

    Ok((output, local_rect))
}
