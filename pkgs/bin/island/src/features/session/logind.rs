use zbus::{Connection, block_on};

use super::{
    Action,
    action::{LOGIN, MANAGER},
};
use crate::dbus::uncached;

/// Asks logind to do `action`, as this session or for the machine.
pub fn logind(action: Action) -> zbus::Result<()> {
    block_on(async {
        let connection = Connection::system().await?;
        let (path, interface, method) = action.logind();
        let proxy = uncached(&connection, LOGIN, path, interface).await?;
        if interface == MANAGER {
            proxy.call::<_, _, ()>(method, &(true,)).await
        } else {
            proxy.call::<_, _, ()>(method, &()).await
        }
    })
}
