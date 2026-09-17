//! Wayland output enumeration and geometry queries

use anyhow::{Context, Result};
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use wayland_client::{Connection, globals::registry_queue_init, protocol::wl_output};

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

    smithay_client_toolkit::delegate_dispatch2!(OutputEnumerator);
    delegate_registry!(OutputEnumerator);

    let (globals, mut event_queue) =
        registry_queue_init::<OutputEnumerator>(conn).context("Failed to init registry")?;

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

/// Finds every output a rectangle covers, with each output's global geometry.
///
/// A selection can span monitors, and one capture only ever covers one output,
/// so the caller assembles the region from all of them. Ordered by how much of
/// the rectangle each output holds, so the first is the one it mostly sits on.
pub fn find_outputs_for_rect<'a>(
    outputs: &'a [OutputInfo],
    rect: Rect,
) -> Result<Vec<(&'a wl_output::WlOutput, Rect)>> {
    let mut covering: Vec<_> = outputs
        .iter()
        .filter_map(|info| {
            let geometry = output_rect(info);
            let overlap = rect.intersection(&geometry)?;

            Some((overlap.area(), &info.0, geometry))
        })
        .collect();

    if covering.is_empty() {
        anyhow::bail!("Selection {} is not on any output", rect.describe());
    }

    covering.sort_by_key(|(area, ..)| std::cmp::Reverse(*area));

    Ok(covering
        .into_iter()
        .map(|(_, output, geometry)| (output, geometry))
        .collect())
}

/// Returns an output's geometry as a rectangle in global coordinates.
fn output_rect((_, _, x, y, width, height): &OutputInfo) -> Rect {
    Rect::new(*x, *y, *width as i32, *height as i32)
}
