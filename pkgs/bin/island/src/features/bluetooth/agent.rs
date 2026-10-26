//! island as BlueZ's pairing agent: a code to confirm waits for an answer from the panel, a code to
//! type on the device is shown until pairing ends, and anything island cannot answer is refused.

use std::sync::{Arc, Mutex};

use futures::channel::{mpsc::UnboundedSender, oneshot};
use tracing::info;
use zbus::{OwnedObjectPath, fdo, interface};

use super::runtime::Request;

pub const PATH: &str = "/io/github/krezh/Island/Bluetooth/Agent";

/// What the agent is asking now, and where the answer goes.
#[derive(Default)]
pub struct Asked {
    pub next: u64,
    pub current: Option<Ask>,
    reply: Option<oneshot::Sender<bool>>,
}

#[derive(Clone, Debug)]
pub struct Ask {
    pub id: u64,
    pub device: String,
    pub code: String,
    pub confirm: bool,
}

impl Asked {
    fn ask(&mut self, device: &OwnedObjectPath, code: String, confirm: bool) -> u64 {
        self.next += 1;
        self.current = Some(Ask {
            id: self.next,
            device: device.as_str().to_owned(),
            code,
            confirm,
        });
        self.reply = None;
        self.next
    }

    /// Answers the confirmation `id`, if it is still the one asked.
    pub fn answer(&mut self, id: u64, yes: bool) {
        if self.current.as_ref().is_some_and(|ask| ask.id == id) {
            self.current = None;
            if let Some(reply) = self.reply.take() {
                let _ = reply.send(yes);
            }
        }
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.reply = None;
    }
}

pub struct Agent {
    pub asked: Arc<Mutex<Asked>>,
    /// Wakes the worker, so the panel shows what is asked at once.
    pub poke: Mutex<UnboundedSender<Request>>,
    /// Whether thor is visible, which is when it lets devices pair with it unasked.
    pub visible: Arc<Mutex<bool>>,
}

impl Agent {
    fn lock(&self) -> std::sync::MutexGuard<'_, Asked> {
        self.asked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn poke(&self) {
        if let Ok(poke) = self.poke.lock() {
            let _ = poke.unbounded_send(Request::Refresh);
        }
    }
}

fn refused(what: &str) -> fdo::Error {
    fdo::Error::AccessDenied(format!("island {what}"))
}

/// Six digits, split for reading off a device.
pub fn passkey(code: u32) -> String {
    let digits = format!("{code:06}");
    format!("{} {}", &digits[..3], &digits[3..])
}

#[interface(name = "org.bluez.Agent1")]
impl Agent {
    async fn release(&self) {
        self.lock().clear();
    }

    async fn request_pin_code(&self, _device: OwnedObjectPath) -> fdo::Result<String> {
        Err(refused("cannot type a PIN for a device"))
    }

    async fn request_passkey(&self, _device: OwnedObjectPath) -> fdo::Result<u32> {
        Err(refused("cannot type a passkey for a device"))
    }

    async fn display_pin_code(&self, device: OwnedObjectPath, pincode: String) {
        self.lock().ask(&device, pincode, false);
        self.poke();
    }

    async fn display_passkey(&self, device: OwnedObjectPath, passkey_value: u32, _entered: u16) {
        self.lock().ask(&device, passkey(passkey_value), false);
        self.poke();
    }

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey_value: u32,
    ) -> fdo::Result<()> {
        let (reply, answer) = oneshot::channel();
        {
            let mut asked = self.lock();
            asked.ask(&device, passkey(passkey_value), true);
            asked.reply = Some(reply);
        }
        self.poke();
        let yes = answer.await.unwrap_or(false);
        self.poke();
        if yes {
            Ok(())
        } else {
            Err(refused("was told not to pair"))
        }
    }

    async fn request_authorization(&self, device: OwnedObjectPath) -> fdo::Result<()> {
        let visible = *self
            .visible
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if visible {
            info!(device = device.as_str(), "let a device pair while visible");
            Ok(())
        } else {
            Err(refused("is not visible to pair with"))
        }
    }

    async fn authorize_service(&self, _device: OwnedObjectPath, _uuid: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn cancel(&self) {
        self.lock().clear();
        self.poke();
    }
}

#[cfg(test)]
#[path = "agent_test.rs"]
mod tests;
