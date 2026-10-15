use std::time::Instant;

use gpui::SpringConfig;

use crate::spring::Spring;

pub struct Slot<T> {
    pub key: String,
    pub item: T,
    pub presence: Spring,
}

pub struct Presence<T> {
    slots: Vec<Slot<T>>,
    motion: SpringConfig,
}

impl<T: Clone> Presence<T> {
    pub fn new(motion: SpringConfig) -> Self {
        Self {
            slots: Vec::new(),
            motion,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Slot<T>> {
        self.slots.iter()
    }

    pub fn sync(&mut self, items: &[(String, T)], animate: bool) {
        let motion = self.motion;
        let order: Vec<String> = self.slots.iter().map(|slot| slot.key.clone()).collect();
        let mut old = std::mem::take(&mut self.slots);
        let mut next = Vec::with_capacity(items.len() + old.len());
        for (key, item) in items {
            let slot = match old.iter().position(|slot| &slot.key == key) {
                Some(index) => {
                    let mut slot = old.remove(index);
                    slot.item = item.clone();
                    slot.presence.retarget(1.0, motion);
                    slot
                }
                None => {
                    let mut presence = Spring::new(if animate { 0.0 } else { 1.0 }, motion, 0.002);
                    presence.retarget(1.0, motion);
                    Slot {
                        key: key.clone(),
                        item: item.clone(),
                        presence,
                    }
                }
            };
            next.push(slot);
        }
        for mut gone in old {
            if !animate {
                continue;
            }
            gone.presence.retarget(0.0, motion);
            let at = order
                .iter()
                .position(|key| *key == gone.key)
                .and_then(|index| {
                    order[..index]
                        .iter()
                        .rev()
                        .find_map(|before| next.iter().position(|slot| slot.key == *before))
                })
                .map_or(0, |index| index + 1);
            next.insert(at, gone);
        }
        self.slots = next;
    }

    pub fn pending_width(&self, unit: f32) -> f32 {
        self.slots
            .iter()
            .map(|slot| unit * (slot.presence.target() - slot.presence.value()))
            .sum()
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for slot in &mut self.slots {
            slot.presence.advance(dt, now);
        }
        self.slots
            .retain(|slot| !(slot.presence.target() == 0.0 && slot.presence.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.slots.iter().all(|slot| slot.presence.is_settled())
    }
}
