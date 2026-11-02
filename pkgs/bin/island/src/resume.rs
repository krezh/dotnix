//! Hears the machine wake from sleep, which timers that sleep through it do not notice.

use std::thread;

use futures::StreamExt;
use tracing::{debug, warn};
use zbus::{Connection, MatchRule, MessageStream, block_on, message::Type};

async fn follow(resumed: &mut impl FnMut()) -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.login1")
        .interface("org.freedesktop.login1.Manager")
        .member("PrepareForSleep")
        .path("/org/freedesktop/login1")
        .build()?;
    let mut stream = MessageStream::for_match_rule(rule, &connection, None).await?;
    while let Some(message) = stream.next().await {
        if message.is_ok_and(|message| matches!(message.body().deserialize(), Ok(false))) {
            debug!("the machine woke");
            resumed();
        }
    }
    Ok(())
}

pub fn listen(mut resumed: impl FnMut() + Send + 'static) {
    let spawned = thread::Builder::new()
        .name("island-resume".into())
        .spawn(move || {
            if let Err(error) = block_on(follow(&mut resumed)) {
                warn!("cannot hear the machine wake: {error}");
            }
        });
    if let Err(error) = spawned {
        warn!("cannot listen for the machine waking: {error}");
    }
}
