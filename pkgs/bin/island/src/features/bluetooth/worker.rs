//! The Bluetooth feature's worker. It reads BlueZ once, then follows its signals: objects added and
//! removed, properties changed, and BlueZ itself coming and going. Requests from the panel arrive
//! on the same loop; slow ones, such as connecting, run on their own thread and say when they end.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use futures::{
    StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    stream,
};
use tracing::{debug, info, warn};
use zbus::{
    Connection, MatchRule, Message, MessageStream, OwnedObjectPath, OwnedVariant, block_on,
};

use super::{
    agent::{self, Agent, Asked},
    bluez::{self, Adapter, Objects},
    runtime::Request,
    state::{Asking, Busy, State},
};
use crate::{features::Feed, supervisor::Ended};

/// How long a scan runs before it stops by itself.
const SCAN_FOR: Duration = Duration::from_secs(30);
/// How long thor stays visible, as BlueZ counts it.
const VISIBLE_FOR: u32 = 180;

enum Event {
    Bus(zbus::Result<Message>),
    Request(Request),
}

#[derive(Default)]
struct World {
    objects: Objects,
    adapter: Option<Adapter>,
    busy: HashMap<String, Busy>,
    since: HashMap<String, i64>,
    connected: HashSet<String>,
    /// The first read has happened, so a device connected after it is one island saw connect.
    loaded: bool,
    scan: u64,
    visible_until: Option<i64>,
    state: State,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl World {
    async fn reload(&mut self, connection: &Connection) {
        match bluez::objects(connection).await {
            Ok(objects) => self.objects = objects,
            Err(error) => {
                debug!("cannot read BlueZ: {error}");
                self.objects.clear();
            }
        }
    }

    /// Folds one of BlueZ's signals into what is known.
    fn apply(&mut self, message: &Message) -> bool {
        let header = message.header();
        let Some(member) = header.member().map(|member| member.as_str().to_owned()) else {
            return false;
        };
        let body = message.body();
        match member.as_str() {
            "InterfacesAdded" => {
                let Ok((path, interfaces)) = body.deserialize::<(
                    OwnedObjectPath,
                    HashMap<String, HashMap<String, OwnedVariant>>,
                )>() else {
                    return false;
                };
                self.objects.entry(path).or_default().extend(interfaces);
                true
            }
            "InterfacesRemoved" => {
                let Ok((path, gone)) = body.deserialize::<(OwnedObjectPath, Vec<String>)>() else {
                    return false;
                };
                if let Some(interfaces) = self.objects.get_mut(&path) {
                    for interface in gone {
                        interfaces.remove(&interface);
                    }
                    if interfaces.is_empty() {
                        self.objects.remove(&path);
                    }
                }
                true
            }
            "PropertiesChanged" => {
                let Some(path) = header
                    .path()
                    .map(|path| OwnedObjectPath::from(path.to_owned()))
                else {
                    return false;
                };
                let Ok((interface, changed, invalidated)) =
                    body.deserialize::<(String, HashMap<String, OwnedVariant>, Vec<String>)>()
                else {
                    return false;
                };
                let props = self
                    .objects
                    .entry(path)
                    .or_default()
                    .entry(interface)
                    .or_default();
                props.extend(changed);
                for name in invalidated {
                    props.remove(&name);
                }
                true
            }
            _ => false,
        }
    }

    fn build(&mut self, asked: &Mutex<Asked>, visible: &Mutex<bool>) {
        let parsed = bluez::parse(&self.objects);
        let (adapter, mut devices) = parsed.clone().unwrap_or_default();
        let at = now();
        let connected: HashSet<String> = devices
            .iter()
            .filter(|device| device.connected)
            .map(|device| device.path.clone())
            .collect();
        for path in connected.difference(&self.connected) {
            if self.loaded {
                self.since.insert(path.clone(), at);
            }
        }
        self.since.retain(|path, _| connected.contains(path));
        self.connected = connected;
        self.loaded = true;
        for device in &mut devices {
            device.busy = self.busy.get(&device.path).copied();
            device.since = self.since.get(&device.path).copied();
        }
        if !adapter.discoverable {
            self.visible_until = None;
        }
        *visible
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = adapter.discoverable;
        let asking = asked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .current
            .clone()
            .map(|ask| Asking {
                id: ask.id,
                device: devices
                    .iter()
                    .find(|device| device.path == ask.device)
                    .map_or_else(|| "a device".to_owned(), |device| device.name.clone()),
                code: ask.code,
                confirm: ask.confirm,
            });
        self.state = State {
            available: parsed.is_some(),
            powered: adapter.powered,
            discovering: adapter.discovering,
            discoverable: adapter.discoverable,
            visible_until: self.visible_until,
            name: adapter.name.clone(),
            address: adapter.address.clone(),
            devices,
            asking,
        };
        self.adapter = parsed.map(|(adapter, _)| adapter);
    }

    fn adapter_path(&self) -> Option<String> {
        self.adapter.as_ref().map(|adapter| adapter.path.clone())
    }

    /// Runs a slow call to BlueZ on its own thread, and says when it is done.
    fn slow(
        &mut self,
        connection: &Connection,
        sender: &UnboundedSender<Request>,
        path: String,
        busy: Busy,
        call: impl FnOnce(&Connection, &str) -> zbus::Result<()> + Send + 'static,
    ) {
        self.busy.insert(path.clone(), busy);
        let (connection, sender) = (connection.clone(), sender.clone());
        let spawned = thread::Builder::new()
            .name("island-bluetooth-call".into())
            .spawn(move || {
                if let Err(error) = call(&connection, &path) {
                    warn!(device = path, "BlueZ refused: {error}");
                }
                let _ = sender.unbounded_send(Request::Done(path));
            });
        if let Err(error) = spawned {
            warn!("cannot start a Bluetooth call: {error}");
        }
    }

    async fn handle(
        &mut self,
        connection: &Connection,
        sender: &UnboundedSender<Request>,
        asked: &Mutex<Asked>,
        request: Request,
    ) {
        debug!(?request, "bluetooth request");
        let adapter = self.adapter_path();
        let result = match request {
            Request::Power(on) => match &adapter {
                Some(adapter) => {
                    let on = on.unwrap_or(!self.state.powered);
                    bluez::set_adapter(connection, adapter, "Powered", on).await
                }
                None => Ok(()),
            },
            Request::Scan => match &adapter {
                Some(adapter) if self.state.discovering => {
                    bluez::adapter_call(connection, adapter, "StopDiscovery").await
                }
                Some(adapter) => {
                    self.scan += 1;
                    let (generation, sender) = (self.scan, sender.clone());
                    let _ = thread::Builder::new()
                        .name("island-bluetooth-scan".into())
                        .spawn(move || {
                            thread::sleep(SCAN_FOR);
                            let _ = sender.unbounded_send(Request::StopScan(generation));
                        });
                    bluez::adapter_call(connection, adapter, "StartDiscovery").await
                }
                None => Ok(()),
            },
            Request::StopScan(generation) => match &adapter {
                Some(adapter) if generation == self.scan && self.state.discovering => {
                    bluez::adapter_call(connection, adapter, "StopDiscovery").await
                }
                _ => Ok(()),
            },
            Request::Visible => match &adapter {
                Some(adapter) if self.state.discoverable => {
                    bluez::set_adapter(connection, adapter, "Discoverable", false).await
                }
                Some(adapter) => {
                    self.visible_until = Some(now() + i64::from(VISIBLE_FOR));
                    let timeout =
                        bluez::set_adapter(connection, adapter, "DiscoverableTimeout", VISIBLE_FOR)
                            .await;
                    let pairable = bluez::set_adapter(connection, adapter, "Pairable", true).await;
                    timeout
                        .and(pairable)
                        .and(bluez::set_adapter(connection, adapter, "Discoverable", true).await)
                }
                None => Ok(()),
            },
            Request::Connect(path) => {
                self.slow(
                    connection,
                    sender,
                    path,
                    Busy::Connecting,
                    |connection, path| block_on(bluez::device_call(connection, path, "Connect")),
                );
                Ok(())
            }
            Request::Disconnect(path) => {
                self.slow(
                    connection,
                    sender,
                    path,
                    Busy::Disconnecting,
                    |connection, path| block_on(bluez::device_call(connection, path, "Disconnect")),
                );
                Ok(())
            }
            Request::Pair(path) => {
                self.slow(
                    connection,
                    sender,
                    path,
                    Busy::Pairing,
                    |connection, path| {
                        block_on(async {
                            bluez::device_call(connection, path, "Pair").await?;
                            bluez::set_trusted(connection, path, true).await?;
                            bluez::device_call(connection, path, "Connect").await
                        })
                    },
                );
                Ok(())
            }
            Request::Trust(path, trusted) => bluez::set_trusted(connection, &path, trusted).await,
            Request::Forget(path) => match &adapter {
                Some(adapter) => bluez::remove(connection, adapter, &path).await,
                None => Ok(()),
            },
            Request::Answer(id, yes) => {
                asked
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .answer(id, yes);
                Ok(())
            }
            Request::Done(path) => {
                self.busy.remove(&path);
                // A pairing that ended takes its code card with it.
                let mut asked = asked
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if asked.current.as_ref().is_some_and(|ask| ask.device == path) {
                    asked.clear();
                }
                Ok(())
            }
            Request::Refresh => Ok(()),
        };
        if let Err(error) = result {
            warn!("BlueZ refused: {error}");
        }
    }
}

async fn listen(connection: &Connection, rule: &str) -> zbus::Result<MessageStream> {
    MessageStream::for_match_rule(MatchRule::try_from(rule)?, connection, None).await
}

pub(super) fn run(
    feed: &Feed<State>,
    requests: &Mutex<UnboundedReceiver<Request>>,
    sender: &UnboundedSender<Request>,
) -> Ended {
    let mut requests = requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    block_on(async {
        let connection = match Connection::system().await {
            Ok(connection) => connection,
            Err(error) => {
                return Ended::Failed(format!("cannot connect to the system bus: {error}"));
            }
        };
        let asked = Arc::new(Mutex::new(Asked::default()));
        let visible = Arc::new(Mutex::new(false));
        let agent = Agent {
            asked: asked.clone(),
            poke: Mutex::new(sender.clone()),
            visible: visible.clone(),
        };
        if let Err(error) = connection.object_server().at(agent::PATH, agent).await {
            return Ended::Failed(format!("cannot serve the pairing agent: {error}"));
        }
        let signals = match listen(&connection, "type='signal',sender='org.bluez'").await {
            Ok(signals) => signals,
            Err(error) => return Ended::Failed(format!("cannot follow BlueZ: {error}")),
        };
        let owner = match listen(
            &connection,
            "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.bluez'",
        )
        .await
        {
            Ok(owner) => owner,
            Err(error) => return Ended::Failed(format!("cannot follow BlueZ: {error}")),
        };
        let mut world = World::default();
        world.reload(&connection).await;
        match bluez::register_agent(&connection, agent::PATH).await {
            Ok(()) => info!("pairing through island"),
            Err(error) => warn!("cannot become BlueZ's pairing agent: {error}"),
        }
        world.build(&asked, &visible);
        if !feed.publish(world.state.clone()) {
            return Ended::Unwanted;
        }
        let bus = stream::select(signals, owner).map(Event::Bus);
        let mut events = stream::select(bus, (&mut *requests).map(Event::Request));
        while let Some(event) = events.next().await {
            let changed = match event {
                Event::Bus(Ok(message)) => {
                    let restarted = message
                        .header()
                        .member()
                        .is_some_and(|member| member.as_str() == "NameOwnerChanged");
                    if restarted {
                        // BlueZ came back or went away: read it afresh and be its agent again.
                        world.reload(&connection).await;
                        if !world.objects.is_empty() {
                            let _ = bluez::register_agent(&connection, agent::PATH).await;
                        }
                        true
                    } else {
                        world.apply(&message)
                    }
                }
                Event::Bus(Err(error)) => {
                    debug!("a BlueZ signal could not be read: {error}");
                    false
                }
                Event::Request(request) => {
                    world.handle(&connection, sender, &asked, request).await;
                    true
                }
            };
            if !changed {
                continue;
            }
            let before = world.state.clone();
            world.build(&asked, &visible);
            if world.state != before && !feed.publish(world.state.clone()) {
                return Ended::Unwanted;
            }
        }
        Ended::Unwanted
    })
}
