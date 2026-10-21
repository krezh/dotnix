//! Puts text on the clipboard, for a Copy button: offered through ext-data-control on a Wayland
//! connection of its own, and served until something else takes the clipboard.

use std::{fs::File, io::Write, os::fd::OwnedFd, sync::Arc, thread};

use tracing::{debug, warn};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry::WlRegistry, wl_seat::WlSeat},
};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};

/// Text types, preferred first, as apps ask for them.
const TEXT: [&str; 5] = [
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
    "TEXT",
];

struct Offering {
    text: Arc<Vec<u8>>,
    /// Once another selection replaces it.
    done: bool,
}

impl Dispatch<WlRegistry, GlobalListContents> for Offering {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(Offering: ignore WlSeat);
delegate_noop!(Offering: ExtDataControlManagerV1);
delegate_noop!(Offering: ignore ExtDataControlOfferV1);

impl Dispatch<ExtDataControlDeviceV1, ()> for Offering {
    fn event(
        offering: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::Selection { id: Some(offer) }
            | ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                offer.destroy();
            }
            ext_data_control_device_v1::Event::Finished => offering.done = true,
            _ => {}
        }
    }

    event_created_child!(Offering, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ExtDataControlSourceV1, ()> for Offering {
    fn event(
        offering: &mut Self,
        source: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_source_v1::Event::Send { fd, .. } => send(fd, offering.text.clone()),
            ext_data_control_source_v1::Event::Cancelled => {
                source.destroy();
                offering.done = true;
            }
            _ => {}
        }
    }
}

/// Off the serving thread, so a slow reader cannot hold up the next request.
fn send(fd: OwnedFd, text: Arc<Vec<u8>>) {
    thread::spawn(move || {
        if let Err(error) = File::from(fd).write_all(&text) {
            debug!("copied text not fully read: {error}");
        }
    });
}

fn serve(text: String) -> Result<(), String> {
    let connection = Connection::connect_to_env()
        .map_err(|error| format!("cannot connect to Wayland: {error}"))?;
    let (globals, mut queue) =
        registry_queue_init::<Offering>(&connection).map_err(|error| error.to_string())?;
    let handle = queue.handle();
    let manager: ExtDataControlManagerV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|_| "the compositor does not offer ext-data-control".to_owned())?;
    let seat: WlSeat = globals
        .bind(&handle, 1..=7, ())
        .map_err(|_| "the compositor has no seat".to_owned())?;
    let device = manager.get_data_device(&seat, &handle, ());
    let source = manager.create_data_source(&handle, ());
    for mime in TEXT {
        source.offer(mime.to_owned());
    }
    device.set_selection(Some(&source));
    let mut offering = Offering {
        text: Arc::new(text.into_bytes()),
        done: false,
    };
    while !offering.done {
        queue
            .blocking_dispatch(&mut offering)
            .map_err(|error| format!("lost the Wayland connection: {error}"))?;
    }
    device.destroy();
    Ok(())
}

pub fn text(text: String) {
    let spawned = thread::Builder::new()
        .name("copy".to_owned())
        .spawn(move || {
            if let Err(error) = serve(text) {
                warn!("cannot copy: {error}");
            }
        });
    if let Err(error) = spawned {
        warn!("cannot copy: {error}");
    }
}
