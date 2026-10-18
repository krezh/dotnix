//! island serves the StatusNotifierWatcher when no other app does.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use futures::{StreamExt, channel::oneshot};
use gpui::RenderImage;
use serde::de::DeserializeOwned;
use zbus::{
    Connection, MatchRule, MessageStream, OwnedObjectPath, OwnedVariant, Proxy, Variant, block_on,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    message::{Header, Type},
    object_server::SignalEmitter,
};

use tracing::{debug, info, warn};

use crate::{dbus::uncached, features::Feed, icons, menu::MenuNode, supervisor::Ended};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
const MENU: &str = "com.canonical.dbusmenu";
const DEFAULT_PATH: &str = "/StatusNotifierItem";

pub use crate::components::icon::Picture as TrayIcon;

#[derive(Clone, PartialEq)]
pub struct TrayItem {
    pub key: String,
    pub title: String,
    pub icon: Option<TrayIcon>,
    pub has_menu: bool,
    pub item_is_menu: bool,
    pub attention: bool,
}

pub enum Command {
    Activate(String),
    Secondary(String),
    /// Asks an item without a menu of its own to show one itself, at a position on screen.
    ContextMenu(String, i32, i32),
    Menu(String, oneshot::Sender<Option<MenuNode>>),
    Click(String, i32),
}

enum Msg {
    Registered(String, Option<String>),
    Changed(String, String),
    Gone(String),
    Command(Command),
}

#[derive(Clone)]
pub struct Client {
    sender: Sender<Msg>,
}

impl Client {
    pub fn send(&self, command: Command) {
        let _ = self.sender.send(Msg::Command(command));
    }
}

struct Watcher {
    items: Arc<Mutex<Vec<String>>>,
    sender: Mutex<Sender<Msg>>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let sender = header.sender().map(ToString::to_string);
        let key = item_key(&service, sender.as_deref());
        {
            let mut items = lock(&self.items);
            if !items.contains(&key) {
                items.push(key.clone());
            }
        }
        let _ = lock(&self.sender).send(Msg::Registered(service, sender));
        let _ = Self::status_notifier_item_registered(&emitter, &key).await;
    }

    fn register_status_notifier_host(&self, _service: String) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        lock(&self.items).clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

fn split(service: &str, sender: Option<&str>) -> (String, String) {
    if service.starts_with('/') {
        return (sender.unwrap_or_default().to_owned(), service.to_owned());
    }
    match service.find('/') {
        Some(index) => (service[..index].to_owned(), service[index..].to_owned()),
        None => (service.to_owned(), DEFAULT_PATH.to_owned()),
    }
}

fn item_key(service: &str, sender: Option<&str>) -> String {
    let (bus, path) = split(service, sender);
    format!("{bus}{path}")
}

struct Entry {
    bus: String,
    owner: String,
    path: String,
    menu: Option<OwnedObjectPath>,
    item: TrayItem,
}

async fn property<T: DeserializeOwned + zbus::Type>(proxy: &Proxy<'_>, name: &str) -> Option<T> {
    proxy.get_property::<T>(name).await.ok()
}

fn pixmap(pixmaps: Vec<(i32, i32, Vec<u8>)>) -> Option<Arc<RenderImage>> {
    let (width, height, data) = pixmaps
        .into_iter()
        .filter(|(width, height, data)| {
            *width > 0 && *height > 0 && data.len() == (*width as usize) * (*height as usize) * 4
        })
        .min_by_key(|(width, _, _)| width.abs_diff(32))?;
    let mut bgra = Vec::with_capacity(data.len());
    for [a, r, g, b] in data.as_chunks::<4>().0 {
        bgra.extend_from_slice(&[*b, *g, *r, *a]);
    }
    let buffer = image::RgbaImage::from_raw(width as u32, height as u32, bgra)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

async fn first<T: DeserializeOwned + zbus::Type>(proxy: &Proxy<'_>, names: &[&str]) -> Option<T> {
    for name in names {
        if let Some(value) = property(proxy, name).await {
            return Some(value);
        }
    }
    None
}

async fn load(connection: &Connection, bus: &str, path: &str, key: String) -> Option<Entry> {
    let proxy = uncached(connection, bus.to_owned(), path.to_owned(), ITEM)
        .await
        .ok()?;
    let status: String = property(&proxy, "Status").await.unwrap_or_default();
    if status == "Passive" {
        return None;
    }
    let title: String = match property::<String>(&proxy, "Title")
        .await
        .filter(|title| !title.is_empty())
    {
        Some(title) => title,
        None => property(&proxy, "Id").await.unwrap_or_default(),
    };
    let theme_path: String = property(&proxy, "IconThemePath").await.unwrap_or_default();
    let name_key = if status == "NeedsAttention" {
        "AttentionIconName"
    } else {
        "IconName"
    };
    let pixmap_key = if status == "NeedsAttention" {
        "AttentionIconPixmap"
    } else {
        "IconPixmap"
    };
    let icon = match first::<String>(&proxy, &[name_key, "IconName"])
        .await
        .and_then(|name| icons::lookup_in(&theme_path, &name))
    {
        Some(file) => Some(TrayIcon::File(file)),
        None => first::<Vec<(i32, i32, Vec<u8>)>>(&proxy, &[pixmap_key, "IconPixmap"])
            .await
            .and_then(pixmap)
            .map(TrayIcon::Pixels),
    };
    let menu: Option<OwnedObjectPath> = property(&proxy, "Menu")
        .await
        .filter(|menu: &OwnedObjectPath| menu.as_str() != "/");
    let owner = async {
        DBusProxy::new(connection)
            .await
            .ok()?
            .get_name_owner(bus.try_into().ok()?)
            .await
            .ok()
    }
    .await
    .map_or_else(|| bus.to_owned(), |owner| owner.to_string());
    let item_is_menu = property(&proxy, "ItemIsMenu").await.unwrap_or(false);
    Some(Entry {
        bus: bus.to_owned(),
        owner,
        path: path.to_owned(),
        item: TrayItem {
            key,
            title,
            icon,
            has_menu: menu.is_some(),
            item_is_menu,
            attention: status == "NeedsAttention",
        },
        menu,
    })
}

/// One dbusmenu entry, `(ia{sv}av)`, as its children arrive: a structure inside a variant.
fn parse_node(value: &Variant<'_>) -> Option<MenuNode> {
    let Variant::Structure(structure) = value else {
        return None;
    };
    let fields = structure.fields();
    let id = i32::try_from(fields.first()?).ok()?;
    let mut props: HashMap<String, OwnedVariant> = HashMap::new();
    if let Some(Variant::Dict(dict)) = fields.get(1) {
        props = HashMap::<String, OwnedVariant>::try_from(dict.try_clone().ok()?).ok()?;
    }
    let children: Vec<&Variant<'_>> = match fields.get(2) {
        Some(Variant::Array(array)) => array.iter().collect(),
        _ => Vec::new(),
    };
    menu_node(id, &props, &children)
}

fn menu_node(
    id: i32,
    props: &HashMap<String, OwnedVariant>,
    children: &[&Variant<'_>],
) -> Option<MenuNode> {
    let text = |key: &str| {
        props
            .get(key)
            .and_then(|value| String::try_from(value.try_clone().ok()?).ok())
    };
    let flag = |key: &str| {
        props
            .get(key)
            .and_then(|value| bool::try_from(&**value).ok())
    };
    if flag("visible") == Some(false) {
        return None;
    }
    let toggle = text("toggle-type")
        .filter(|kind| !kind.is_empty())
        .map(|_| {
            props
                .get("toggle-state")
                .and_then(|value| i32::try_from(&**value).ok())
                == Some(1)
        });
    let children = children
        .iter()
        .filter_map(|child| match child {
            Variant::Variant(inner) => parse_node(inner),
            other => parse_node(other),
        })
        .collect();
    Some(MenuNode {
        id,
        label: text("label")
            .unwrap_or_default()
            .replace("__", "\u{0}")
            .replace('_', "")
            .replace('\u{0}', "_"),
        enabled: flag("enabled").unwrap_or(true),
        separator: text("type").as_deref() == Some("separator"),
        toggle,
        children,
    })
}

fn fetch_menu(connection: &Connection, entry: &Entry) -> Result<MenuNode, String> {
    let path = entry.menu.as_ref().ok_or("the item has no menu")?;
    let proxy = block_on(uncached(
        connection,
        entry.bus.clone(),
        path.to_string(),
        MENU,
    ))
    .map_err(|error| error.to_string())?;
    if let Err(error) = block_on(proxy.call_method("AboutToShow", &(0i32,))) {
        debug!(
            item = entry.item.key,
            "AboutToShow failed, asking for the layout anyway: {error}"
        );
    }
    let reply = block_on(proxy.call_method("GetLayout", &(0i32, -1i32, Vec::<&str>::new())))
        .map_err(|error| format!("GetLayout failed: {error}"))?;
    let body = reply.body();
    let (revision, (id, props, children)): (
        u32,
        (i32, HashMap<String, OwnedVariant>, Vec<OwnedVariant>),
    ) = body
        .deserialize()
        .map_err(|error| format!("cannot read the layout: {error}"))?;
    let children: Vec<&Variant<'_>> = children.iter().map(|child| &**child).collect();
    let root = menu_node(id, &props, &children).ok_or("the menu's root is hidden")?;
    debug!(
        item = entry.item.key,
        revision,
        entries = root.children.len(),
        "tray menu fetched"
    );
    Ok(root)
}

fn click(connection: &Connection, entry: &Entry, id: i32) {
    let Some(path) = entry.menu.as_ref() else {
        return;
    };
    debug!(item = entry.item.key, id, "tray menu entry clicked");
    let sent = block_on(async {
        uncached(connection, entry.bus.clone(), path.to_string(), MENU)
            .await?
            .call_method("Event", &(id, "clicked", Variant::from(0i32), 0u32))
            .await
    });
    if let Err(error) = sent {
        warn!(item = entry.item.key, id, "tray menu click failed: {error}");
    }
}

/// The item may place a window of its own at `x`, `y`.
fn activate(connection: &Connection, entry: &Entry, method: &str, x: i32, y: i32) {
    debug!(item = entry.item.key, method, x, y, "tray item activated");
    let sent = block_on(async {
        uncached(connection, entry.bus.clone(), entry.path.clone(), ITEM)
            .await?
            .call_method(method, &(x, y))
            .await
    });
    if let Err(error) = sent {
        warn!(
            item = entry.item.key,
            method, "tray activation failed: {error}"
        );
    }
}

fn listen(
    connection: &Connection,
    rule: MatchRule<'static>,
    sender: Sender<Msg>,
    map: fn(&zbus::Message) -> Option<Msg>,
) {
    let Ok(mut stream) = block_on(MessageStream::for_match_rule(rule, connection, None)) else {
        return;
    };
    let _ = thread::Builder::new()
        .name("island-tray-signals".into())
        .spawn(move || {
            block_on(async {
                while let Some(message) = stream.next().await {
                    if let Ok(message) = message
                        && let Some(msg) = map(&message)
                        && sender.send(msg).is_err()
                    {
                        return;
                    }
                }
            });
        });
}

fn signal_rule(interface: &'static str) -> Option<MatchRule<'static>> {
    MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(interface)
        .build()
        .ok()
}

/// Recovers a lock a panicking worker held, so the watcher keeps serving its item list.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The items registered so far, for a worker that restarts with none loaded.
fn registered(
    connection: &Connection,
    items: &Mutex<Vec<String>>,
    own_watcher: bool,
) -> Vec<String> {
    if own_watcher {
        return lock(items).clone();
    }
    block_on(async {
        uncached(connection, WATCHER, WATCHER_PATH, WATCHER)
            .await?
            .get_property("RegisteredStatusNotifierItems")
            .await
    })
    .unwrap_or_default()
}

fn worker(
    connection: &Connection,
    receiver: &Receiver<Msg>,
    items: &Mutex<Vec<String>>,
    feed: &Feed<Vec<TrayItem>>,
    own_watcher: bool,
    restarted: bool,
) -> Ended {
    let mut entries: Vec<Entry> = Vec::new();
    if restarted {
        for service in registered(connection, items, own_watcher) {
            let (bus, path) = split(&service, None);
            if let Some(entry) = block_on(load(connection, &bus, &path, format!("{bus}{path}"))) {
                entries.push(entry);
            }
        }
        if !feed.publish(entries.iter().map(|entry| entry.item.clone()).collect()) {
            return Ended::Unwanted;
        }
    }
    let emit_unregistered = |key: &str| {
        if !own_watcher {
            return;
        }
        lock(items).retain(|item| item != key);
        let _ = block_on(connection.emit_signal(
            None::<&str>,
            WATCHER_PATH,
            WATCHER,
            "StatusNotifierItemUnregistered",
            &(key,),
        ));
    };
    for msg in receiver {
        let changed = match msg {
            Msg::Registered(service, sender) => {
                let (bus, path) = split(&service, sender.as_deref());
                let key = format!("{bus}{path}");
                entries.retain(|entry| entry.item.key != key);
                match block_on(load(connection, &bus, &path, key.clone())) {
                    Some(entry) => {
                        debug!(item = key, title = entry.item.title, "tray item added");
                        entries.push(entry);
                    }
                    None => warn!(item = key, "cannot read a tray item"),
                }
                true
            }
            Msg::Changed(owner, path) => {
                let Some(index) = entries
                    .iter()
                    .position(|entry| entry.owner == owner && entry.path == path)
                else {
                    continue;
                };
                let old = entries.remove(index);
                if let Some(entry) =
                    block_on(load(connection, &old.bus, &old.path, old.item.key.clone()))
                {
                    entries.insert(index, entry);
                }
                true
            }
            Msg::Gone(name) => {
                let before = entries.len();
                entries.retain(|entry| {
                    let gone = entry.owner == name || entry.bus == name;
                    if gone {
                        debug!(
                            item = entry.item.key,
                            title = entry.item.title,
                            "tray item removed"
                        );
                        emit_unregistered(&entry.item.key);
                    }
                    !gone
                });
                entries.len() != before
            }
            Msg::Command(command) => {
                let find = |key: &str| entries.iter().find(|entry| entry.item.key == key);
                match command {
                    Command::Activate(key) => {
                        if let Some(entry) = find(&key) {
                            activate(&connection, entry, "Activate", 0, 0);
                        }
                    }
                    Command::Secondary(key) => {
                        if let Some(entry) = find(&key) {
                            activate(&connection, entry, "SecondaryActivate", 0, 0);
                        }
                    }
                    Command::ContextMenu(key, x, y) => {
                        if let Some(entry) = find(&key) {
                            activate(&connection, entry, "ContextMenu", x, y);
                        }
                    }
                    Command::Menu(key, reply) => {
                        let menu = match find(&key) {
                            Some(entry) => fetch_menu(&connection, entry)
                                .inspect_err(|error| {
                                    warn!(item = key, "cannot open the tray menu: {error}")
                                })
                                .ok(),
                            None => {
                                warn!(item = key, "cannot open the tray menu: the item is gone");
                                None
                            }
                        };
                        let _ = reply.send(menu);
                    }
                    Command::Click(key, id) => {
                        if let Some(entry) = find(&key) {
                            click(&connection, entry, id);
                        }
                    }
                }
                false
            }
        };
        if changed && !feed.publish(entries.iter().map(|entry| entry.item.clone()).collect()) {
            return Ended::Unwanted;
        }
    }
    Ended::Unwanted
}

pub fn start(feed: Feed<Vec<TrayItem>>) -> Result<Client, String> {
    let connection = block_on(Connection::session())
        .map_err(|error| format!("tray: no session bus: {error}"))?;
    let (sender, receiver) = mpsc::channel();
    let items = Arc::new(Mutex::new(Vec::new()));

    let watcher = Watcher {
        items: items.clone(),
        sender: Mutex::new(sender.clone()),
    };
    block_on(connection.object_server().at(WATCHER_PATH, watcher))
        .map_err(|error| format!("tray: {error}"))?;
    let own_watcher =
        block_on(connection.request_name_with_flags(WATCHER, RequestNameFlags::DO_NOT_QUEUE))
            .is_ok_and(|reply| reply == RequestNameReply::PrimaryOwner);
    if own_watcher {
        info!("serving the tray watcher");
    } else {
        info!("using another app's tray watcher");
    }
    let host = format!("org.kde.StatusNotifierHost-{}", std::process::id());
    let _ = block_on(connection.request_name(host.as_str()));

    if let Some(rule) = signal_rule(ITEM) {
        listen(&connection, rule, sender.clone(), |message| {
            let header = message.header();
            Some(Msg::Changed(
                header.sender()?.to_string(),
                header.path()?.to_string(),
            ))
        });
    }
    if let Some(rule) = signal_rule("org.freedesktop.DBus") {
        listen(&connection, rule, sender.clone(), |message| {
            if message.header().member()?.as_str() != "NameOwnerChanged" {
                return None;
            }
            let (name, _, new): (String, String, String) = message.body().deserialize().ok()?;
            new.is_empty().then_some(Msg::Gone(name))
        });
    }
    if own_watcher {
        let _ = block_on(connection.emit_signal(
            None::<&str>,
            WATCHER_PATH,
            WATCHER,
            "StatusNotifierHostRegistered",
            &(),
        ));
    } else {
        if let Some(rule) = signal_rule(WATCHER) {
            listen(&connection, rule, sender.clone(), |message| {
                if message.header().member()?.as_str() != "StatusNotifierItemRegistered" {
                    return None;
                }
                let service: String = message.body().deserialize().ok()?;
                Some(Msg::Registered(service, None))
            });
        }
        if let Ok(proxy) = block_on(uncached(&connection, WATCHER, WATCHER_PATH, WATCHER)) {
            let _ = block_on(proxy.call_method("RegisterStatusNotifierHost", &(host.as_str(),)));
            let existing: Vec<String> =
                block_on(proxy.get_property("RegisteredStatusNotifierItems")).unwrap_or_default();
            for service in existing {
                let _ = sender.send(Msg::Registered(service, None));
            }
        }
    }

    let mut restarted = false;
    feed.supervise(move |feed| {
        let ended = worker(&connection, &receiver, &items, feed, own_watcher, restarted);
        restarted = true;
        ended
    });
    Ok(Client { sender })
}
