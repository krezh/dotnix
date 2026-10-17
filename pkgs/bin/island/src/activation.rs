//! xdg-activation tokens, so an app island hands focus to, such as one whose notification action
//! was clicked, may raise its window. The compositor revokes a token once its object is
//! destroyed, so the last few are kept alive until their apps have had time to use them.

use std::collections::VecDeque;

use tracing::debug;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry::WlRegistry,
};
use wayland_protocols::xdg::activation::v1::client::{
    xdg_activation_token_v1::{self, XdgActivationTokenV1},
    xdg_activation_v1::XdgActivationV1,
};

const KEEP: usize = 8;

#[derive(Default)]
struct Received {
    token: Option<String>,
}

impl Dispatch<WlRegistry, GlobalListContents> for Received {
    fn event(_: &mut Self, _: &WlRegistry, _: <WlRegistry as wayland_client::Proxy>::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

delegate_noop!(Received: XdgActivationV1);

impl Dispatch<XdgActivationTokenV1, ()> for Received {
    fn event(
        received: &mut Self,
        _: &XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            received.token = Some(token);
        }
    }
}

pub struct Activation {
    queue: EventQueue<Received>,
    activation: XdgActivationV1,
    kept: VecDeque<XdgActivationTokenV1>,
    _connection: Connection,
}

impl Activation {
    /// `None` when the compositor offers no xdg-activation.
    pub fn connect() -> Option<Self> {
        let connection = Connection::connect_to_env().ok()?;
        let (globals, queue) = registry_queue_init::<Received>(&connection).ok()?;
        let activation = globals.bind(&queue.handle(), 1..=1, ()).ok()?;
        Some(Self {
            queue,
            activation,
            kept: VecDeque::new(),
            _connection: connection,
        })
    }

    /// `None` when the connection broke, after which it should be made again.
    pub fn token(&mut self) -> Option<String> {
        let request = self.activation.get_activation_token(&self.queue.handle(), ());
        request.commit();
        let mut received = Received::default();
        self.queue.roundtrip(&mut received).ok()?;
        self.kept.push_back(request);
        if self.kept.len() > KEEP
            && let Some(oldest) = self.kept.pop_front()
        {
            oldest.destroy();
        }
        debug!(token = received.token.is_some(), "asked for an activation token");
        received.token
    }
}
