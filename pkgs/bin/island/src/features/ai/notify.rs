use std::collections::HashMap;

use tracing::warn;
use zbus::{Connection, Variant, block_on};

use super::model::Notice;

pub(super) fn send(notices: Vec<Notice>) {
    let sent = block_on(async {
        let connection = Connection::session().await?;
        let proxy = crate::dbus::uncached(
            &connection,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )
        .await?;
        for notice in notices {
            let hints: HashMap<&str, Variant<'_>> = HashMap::from([
                (
                    "urgency",
                    Variant::from(if notice.critical { 2_u8 } else { 1_u8 }),
                ),
                ("desktop-entry", Variant::from("island")),
            ]);
            let _: u32 = proxy
                .call(
                    "Notify",
                    &(
                        "Island AI",
                        0_u32,
                        "",
                        notice.title,
                        notice.body,
                        Vec::<String>::new(),
                        hints,
                        -1_i32,
                    ),
                )
                .await?;
        }
        Ok::<(), zbus::Error>(())
    });
    if sent.is_err() {
        warn!("cannot deliver AI quota notifications");
    }
}
