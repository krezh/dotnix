//! The worker: keeps a connection to the sound server, follows its changes and carries out
//! requests, reconnecting when it goes away.

use std::{
    cell::Cell,
    os::fd::AsRawFd,
    rc::Rc,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    time::Instant,
};

use libpulse_binding::{
    context::{State, subscribe::InterestMaskSet},
    mainloop::{
        api::Mainloop as _,
        events::io::FlagSet as IoFlags,
        standard::{IterateResult, Mainloop},
    },
};
use tracing::{debug, error, info, trace, warn};

use super::{
    meters::{Levels, Meters},
    query::Query,
    request::{self, Intents, Mixer, Request},
    state::AudioState,
};
use crate::{
    features::Feed,
    pulse,
    supervisor::{Ended, Reconnect},
    wake::{self, Woken},
};

fn log_change(before: &AudioState, after: &AudioState) {
    if after.default_sink != before.default_sink {
        info!(device = after.default_sink, "default output changed");
    }
    if after.default_source != before.default_source {
        info!(device = after.default_source, "default input changed");
    }
    trace!(
        outputs = after.sinks.len(),
        inputs = after.sources.len(),
        playing = after.playing.len(),
        recording = after.recording.len(),
        "audio state changed"
    );
}

fn run(
    requests: &Receiver<Request>,
    wake: &Woken,
    feed: &Feed<AudioState>,
    levels: &Mutex<Levels>,
) -> Option<()> {
    let mut mainloop = Mainloop::new()?;
    let woken = wake.try_clone().ok()?;
    let _wake_event = mainloop.new_io_event(
        wake.as_raw_fd(),
        IoFlags::INPUT,
        Box::new(move |_, _, _| woken.drain()),
    )?;
    let mut context = pulse::connect(&mut mainloop, "island")
        .map_err(|error| {
            if let Some(error) = error {
                debug!(?error, "cannot connect to the sound server");
            }
        })
        .ok()?;
    info!(
        server = context.get_server().as_deref().unwrap_or("unknown"),
        "connected to the sound server"
    );

    let dirty = Rc::new(Cell::new(true));
    let subscribed = dirty.clone();
    context.set_subscribe_callback(Some(Box::new(move |_, _, _| subscribed.set(true))));
    let interest = InterestMaskSet::SINK
        | InterestMaskSet::SOURCE
        | InterestMaskSet::SERVER
        | InterestMaskSet::SINK_INPUT
        | InterestMaskSet::SOURCE_OUTPUT;
    let _ = context.subscribe(interest, |_| {});

    let mut state = AudioState::default();
    let mut intents = Intents::default();
    let mut pending: Option<Query> = None;
    let mut meters = Meters::default();
    loop {
        if !matches!(context.get_state(), State::Ready) {
            warn!(error = ?context.errno(), "lost the sound server, reconnecting");
            return None;
        }
        let mut changed = false;
        while let Ok(asked) = requests.try_recv() {
            request::apply(&mut context, &mut state, &mut intents, asked);
            changed = true;
        }
        if changed && !feed.publish(state.clone()) {
            return Some(());
        }
        if pending.is_none() && dirty.replace(false) {
            pending = Some(Query::start(&context));
        }
        if let Some(mut next) = pending.as_ref().and_then(Query::finish) {
            pending = None;
            intents.honour(&mut next);
            if next != state {
                log_change(&state, &next);
                state = next;
                if !feed.publish(state.clone()) {
                    return Some(());
                }
            }
        }
        meters.update(&mut context, &state, levels);
        if let IterateResult::Quit(_) | IterateResult::Err(_) = mainloop.iterate(true) {
            return None;
        }
    }
}

pub fn start(feed: Feed<AudioState>) -> Mixer {
    let (sender, requests) = mpsc::channel();
    let levels = Arc::new(Mutex::new(Levels::default()));
    let wake = match wake::pair() {
        Ok((waker, woken)) => {
            let mut pace = Reconnect::default();
            let levels = levels.clone();
            feed.supervise(move |feed| {
                loop {
                    let started = Instant::now();
                    if run(&requests, &woken, feed, &levels).is_some()
                        || !feed.publish(AudioState::default())
                    {
                        return Ended::Unwanted;
                    }
                    pace.wait(started);
                }
            });
            Some(waker)
        }
        Err(error) => {
            error!("audio failed to start: {error}");
            None
        }
    };
    Mixer {
        sender,
        wake,
        levels,
    }
}
