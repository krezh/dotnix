//! The database key, kept in the Secret Service (gnome-keyring, KeePassXC and the like) so nobody
//! manages it: island makes a random key the first time and reads it back on every start. The
//! keyring is normally unlocked at login; a locked one is asked to unlock, which may show its
//! prompt.

use std::{collections::HashMap, time::Duration};

use tracing::info;
use futures::StreamExt;
use zbus::{Connection, MatchRule, MessageStream, OwnedObjectPath, OwnedVariant, Proxy, Variant, block_on, message::Type};

use crate::dbus::uncached;

const SECRETS: &str = "org.freedesktop.secrets";
const SERVICE_PATH: &str = "/org/freedesktop/secrets";
const SERVICE: &str = "org.freedesktop.Secret.Service";
const COLLECTION: &str = "org.freedesktop.Secret.Collection";
const ITEM: &str = "org.freedesktop.Secret.Item";
const PROMPT: &str = "org.freedesktop.Secret.Prompt";
/// How long an unlock prompt may wait for the user.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

/// A secret as the Secret Service sends it: session, parameters, value and content type.
type Secret = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

/// How island's key is found among the keyring's items.
fn attributes() -> HashMap<&'static str, &'static str> {
    HashMap::from([("application", "island"), ("purpose", "database-key")])
}

/// Whether `path` is the Secret Service's "nothing" path.
fn none(path: &OwnedObjectPath) -> bool {
    path.as_str() == "/"
}

fn random_key() -> Result<Vec<u8>, String> {
    let mut key = vec![0_u8; 32];
    let filled = unsafe { libc::getrandom(key.as_mut_ptr().cast(), key.len(), 0) };
    if usize::try_from(filled).ok() != Some(key.len()) {
        return Err(format!("cannot make a key: {}", std::io::Error::last_os_error()));
    }
    Ok(key)
}

/// Runs `prompt`, such as an unlock dialog, and waits for the user to finish with it; false when
/// they dismissed it.
fn run_prompt(connection: &Connection, prompt: &OwnedObjectPath) -> Result<bool, String> {
    if none(prompt) {
        return Ok(true);
    }
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(PROMPT)
        .member("Completed")
        .path(prompt.as_str())
        .build()
        .map_err(|error| error.to_string())?;
    let mut completed = block_on(MessageStream::for_match_rule(rule, connection, None)).map_err(|error| error.to_string())?;
    block_on(async {
        uncached(connection, SECRETS, prompt.as_str(), PROMPT)
            .await?
            .call::<_, _, ()>("Prompt", &("",))
            .await
    })
    .map_err(|error| format!("cannot show the keyring prompt: {error}"))?;
    let (sender, done) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        block_on(async {
            while let Some(message) = completed.next().await {
                let Ok(message) = message else {
                    continue;
                };
                let dismissed =
                    message.body().deserialize::<(bool, OwnedVariant)>().map_or(true, |(dismissed, _)| dismissed);
                let _ = sender.send(dismissed);
                break;
            }
        });
    });
    match done.recv_timeout(PROMPT_TIMEOUT) {
        Ok(dismissed) => Ok(!dismissed),
        Err(_) => Err("the keyring prompt was not answered".to_owned()),
    }
}

/// Unlocks `paths` if they are locked.
fn unlock(connection: &Connection, service: &Proxy<'_>, paths: Vec<OwnedObjectPath>) -> Result<(), String> {
    let (_, prompt): (Vec<OwnedObjectPath>, OwnedObjectPath) =
        block_on(service.call("Unlock", &(paths,))).map_err(|error| format!("cannot unlock the keyring: {error}"))?;
    if run_prompt(connection, &prompt)? {
        Ok(())
    } else {
        Err("the keyring stayed locked".to_owned())
    }
}

/// The database key, made and stored the first time; `fresh` says whether it was just made.
pub fn database_key() -> Result<(Vec<u8>, bool), String> {
    let connection = block_on(Connection::session()).map_err(|error| format!("no session bus: {error}"))?;
    let (service, session): (Proxy<'_>, OwnedObjectPath) = block_on(async {
        let service = uncached(&connection, SECRETS, SERVICE_PATH, SERVICE).await?;
        let (_, session): (OwnedVariant, OwnedObjectPath) =
            service.call("OpenSession", &("plain", Variant::from(""))).await?;
        Ok::<_, zbus::Error>((service, session))
    })
    .map_err(|error| format!("no keyring (Secret Service): {error}"))?;

    let (unlocked, locked): (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) = block_on(service.call("SearchItems", &(attributes(),)))
        .map_err(|error| format!("cannot search the keyring: {error}"))?;
    let item = match (unlocked.into_iter().next(), locked.into_iter().next()) {
        (Some(item), _) => Some(item),
        (None, Some(item)) => {
            unlock(&connection, &service, vec![item.clone()])?;
            Some(item)
        }
        (None, None) => None,
    };
    if let Some(item) = item {
        let (_, _, key, _): Secret =
            block_on(async { uncached(&connection, SECRETS, item.as_str(), ITEM).await?.call("GetSecret", &(&session,)).await })
                .map_err(|error| format!("cannot read the database key: {error}"))?;
        return Ok((key, false));
    }

    let collection: OwnedObjectPath = block_on(service.call("ReadAlias", &("default",)))
        .map_err(|error| format!("cannot find the keyring: {error}"))?;
    if none(&collection) {
        return Err("the keyring has no default collection".to_owned());
    }
    unlock(&connection, &service, vec![collection.clone()])?;
    let key = random_key()?;
    let properties: HashMap<&str, Variant> = HashMap::from([
        ("org.freedesktop.Secret.Item.Label", Variant::from("island database key")),
        ("org.freedesktop.Secret.Item.Attributes", Variant::from(attributes())),
    ]);
    let secret = (&session, Vec::<u8>::new(), key.clone(), "application/octet-stream");
    let (_, prompt): (OwnedObjectPath, OwnedObjectPath) = block_on(async {
        uncached(&connection, SECRETS, collection.as_str(), COLLECTION)
            .await?
            .call("CreateItem", &(properties, secret, true))
            .await
    })
    .map_err(|error| format!("cannot store the database key: {error}"))?;
    if !run_prompt(&connection, &prompt)? {
        return Err("storing the database key was refused".to_owned());
    }
    info!("made a new database key and stored it in the keyring");
    Ok((key, true))
}
