//! Wayland output enumeration and geometry queries

use anyhow::{Context, Result};
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use wayland_client::{Connection, globals::registry_queue_init, protocol::wl_output};

use crate::render::Rect;

#[derive(Clone)]
pub struct OutputInfo {
    pub output: wl_output::WlOutput,
    pub name: String,
    pub logical: Rect,
    pub transform: wl_output::Transform,
}

pub fn get_outputs(conn: &Connection) -> Result<Vec<OutputInfo>> {
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

    event_queue
        .roundtrip(&mut state)
        .context("Failed to enumerate outputs")?;

    let outputs = state
        .output_state
        .outputs()
        .filter_map(|output| {
            let info = state.output_state.info(&output)?;
            let (x, y) = info.logical_position?;
            let (width, height) = info.logical_size?;
            (width > 0 && height > 0).then(|| OutputInfo {
                output: output.clone(),
                name: info.name.clone().unwrap_or_default(),
                logical: Rect::new(x, y, width, height),
                transform: info.transform,
            })
        })
        .collect::<Vec<_>>();

    anyhow::ensure!(
        !outputs.is_empty(),
        "No outputs with logical geometry found"
    );
    Ok(outputs)
}

pub fn find_outputs_for_rect(
    outputs: &[OutputInfo],
    rect: Rect,
) -> Result<Vec<(&OutputInfo, Rect)>> {
    let mut covering = outputs
        .iter()
        .filter_map(|output| {
            let overlap = rect.intersection(&output.logical)?;
            Some((overlap.area(), output, overlap))
        })
        .collect::<Vec<_>>();

    anyhow::ensure!(
        !covering.is_empty(),
        "Selection {} is not on any output",
        rect.describe()
    );
    covering.sort_by_key(|(area, ..)| std::cmp::Reverse(*area));
    Ok(covering
        .into_iter()
        .map(|(_, output, overlap)| (output, overlap))
        .collect())
}
