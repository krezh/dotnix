use super::{Event, Notifications};
use crate::control::{self, Switch};

impl Notifications {
    pub(super) fn set_dnd(&self, switch: Switch) {
        self.change(|store| store.dnd = switch.apply(store.dnd));
    }

    pub(super) fn set_private(&self, switch: Switch) {
        self.change(|store| store.private = switch.apply(store.private));
    }

    pub(super) fn command(&self, args: &[&str]) {
        if args == ["clear"] {
            self.feed.send(Event::Clear);
            return;
        }
        let Ok(switch) = control::switch(&args[1..]) else {
            return;
        };
        match args.first() {
            Some(&"dnd") => self.set_dnd(switch),
            Some(&"private") => self.set_private(switch),
            _ => {}
        }
    }
}

pub(super) fn check(args: &[&str]) -> Result<(), String> {
    match args {
        ["clear"] => Ok(()),
        ["clear", ..] => Err("clear takes no arguments".to_owned()),
        _ => control::switch(&args[1..]).map(drop),
    }
}
