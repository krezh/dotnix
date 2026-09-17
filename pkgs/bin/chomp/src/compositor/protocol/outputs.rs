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

/// Finds the output holding a given rectangle and returns local coordinates.
///
/// A capture comes from a single output, so a rectangle spanning two monitors is
/// resolved to the one holding most of it; picking the largest overlap rather than
/// the first intersection at least keeps the result the part the user aimed at.
/// The returned rectangle is relative to that output's top-left corner and may
/// extend past its edges — cropping clips it.
pub fn find_output_for_rect(
    outputs: &[OutputInfo],
    rect: Rect,
) -> Result<(&wl_output::WlOutput, Rect)> {
    let best = outputs
        .iter()
        .map(|info| (overlap_area(rect, output_rect(info)), info))
        .filter(|(overlap, _)| *overlap > 0)
        .max_by_key(|(overlap, _)| *overlap);

    let (overlap, (output, _, offset_x, offset_y, _, _)) =
        best.context("Selection is not on any output")?;

    if overlap < rect.width as i64 * rect.height as i64 {
        log::warn!(
            "Selection {} spans more than one output; capturing only the part on the output holding most of it",
            rect.describe()
        );
    }

    let local_rect = Rect::new(
        rect.x - offset_x,
        rect.y - offset_y,
        rect.width,
        rect.height,
    );

    Ok((output, local_rect))
}

/// Returns an output's geometry as a rectangle in global coordinates.
fn output_rect((_, _, x, y, width, height): &OutputInfo) -> Rect {
    Rect::new(*x, *y, *width as i32, *height as i32)
}

/// Returns the area shared by two rectangles, in pixels.
fn overlap_area(a: Rect, b: Rect) -> i64 {
    let width = i64::from((a.x + a.width).min(b.x + b.width)) - i64::from(a.x.max(b.x));
    let height = i64::from((a.y + a.height).min(b.y + b.height)) - i64::from(a.y.max(b.y));

    width.max(0) * height.max(0)
}
