use std::time::Duration;

use super::*;
use crate::features::{Feature, Osd};

#[derive(Default)]
struct Part {
    osds: usize,
    events: usize,
}

impl Local for Part {
    fn osd(&mut self, _osd: Osd) {
        self.osds += 1;
    }
}

struct Probe;

impl Feature for Probe {
    const NAME: &'static str = "probe";
    type State = u32;
    type Event = ();

    fn start(_feed: Feed<u32>) -> Self {
        Self
    }
}

impl Surface for Probe {
    type Local = Part;

    fn event(&self, _event: (), islands: &mut Islands<'_, Part>) {
        if let Some(island) = islands.find(|part| part.events == 0) {
            islands.update(island, |part| part.events += 1);
        }
    }

    fn osd(before: &u32, after: &u32) -> Option<Osd> {
        (after > before).then(|| Osd {
            glyph: "",
            color: 0,
            caption: "",
            label: String::new(),
            level: None,
            lasts: Duration::ZERO,
        })
    }
}

#[derive(Default)]
struct Fake {
    parts: Vec<Box<dyn AnyLocal>>,
    told: Vec<&'static str>,
}

impl Fake {
    fn with_islands(count: usize) -> Self {
        Self {
            parts: (0..count)
                .map(|_| Box::new(Part::default()) as Box<dyn AnyLocal>)
                .collect(),
            told: Vec::new(),
        }
    }

    fn part(&self, island: usize) -> &Part {
        self.parts[island].as_any().downcast_ref().unwrap()
    }
}

impl Frontend for Fake {
    fn islands(&self) -> usize {
        self.parts.len()
    }

    fn local(&self, island: usize, _name: &str) -> Option<&dyn AnyLocal> {
        self.parts.get(island).map(|part| &**part)
    }

    fn update_local(
        &mut self,
        island: usize,
        _name: &'static str,
        action: &mut dyn FnMut(&mut dyn AnyLocal),
    ) {
        action(&mut *self.parts[island]);
    }

    fn changed(&mut self, _name: &'static str) {
        self.told.push("changed");
    }

    fn live(&mut self, _name: &'static str) {
        self.told.push("live");
    }
}

#[test]
fn only_a_new_state_reaches_the_frontend() {
    let instance = Instance::new(Probe);
    let mut frontend = Fake::with_islands(1);
    instance.set_state(0, &mut frontend);
    assert!(frontend.told.is_empty());
    instance.set_state(1, &mut frontend);
    instance.set_state(1, &mut frontend);
    assert_eq!(frontend.told, ["changed"]);
    assert_eq!(*instance.state.borrow(), 1);
}

#[test]
fn an_osd_reaches_every_island() {
    let instance = Instance::new(Probe);
    let mut frontend = Fake::with_islands(2);
    instance.set_state(1, &mut frontend);
    instance.set_state(0, &mut frontend);
    assert_eq!(frontend.part(0).osds, 1);
    assert_eq!(frontend.part(1).osds, 1);
}

#[test]
fn an_event_reaches_the_island_the_feature_finds() {
    let instance = Instance::new(Probe);
    let mut frontend = Fake::with_islands(2);
    instance.event((), &mut frontend);
    instance.event((), &mut frontend);
    instance.event((), &mut frontend);
    assert_eq!(frontend.part(0).events, 1);
    assert_eq!(frontend.part(1).events, 1);
}

#[test]
fn live_levels_are_kept_and_told() {
    let instance = Rc::new(Instance::new(Probe));
    let mut frontend = Fake::with_islands(1);
    instance.set_live(vec![0.5], &mut frontend);
    assert_eq!(*instance.live.borrow(), [0.5]);
    assert_eq!(frontend.told, ["live"]);
}

#[test]
fn what_arrives_is_applied_in_order() {
    let instance = Rc::new(Instance::new(Probe));
    let (sender, states) = unbounded();
    let mut arrivals = Arrivals::new();
    arrivals.push(arriving(&instance, states, Instance::set_state));
    sender.unbounded_send(1).unwrap();
    sender.unbounded_send(2).unwrap();
    drop(sender);
    let mut frontend = Fake::with_islands(1);
    zbus::block_on(async {
        while let Some(arrival) = arrivals.next().await {
            arrival(&mut frontend);
        }
    });
    assert_eq!(*instance.state.borrow(), 2);
    assert_eq!(frontend.told, ["changed", "changed"]);
}
