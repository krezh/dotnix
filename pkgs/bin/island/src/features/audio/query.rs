//! Asking the server for everything at once, and turning its answers into an [`AudioState`].

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use libpulse_binding::{
    callbacks::ListResult,
    context::Context,
    proplist::Proplist,
    sample::Spec,
    volume::{ChannelVolumes, Volume},
};

use super::state::{App, AudioState, Device, Form};
use crate::icons;

fn level(volume: &ChannelVolumes) -> f32 {
    volume.avg().0 as f32 / Volume::NORMAL.0 as f32
}

fn form(properties: &Proplist) -> Form {
    let factor = properties.get_str("device.form_factor").unwrap_or_default();
    let icon = properties.get_str("device.icon_name").unwrap_or_default();
    match factor.as_str() {
        "headset" | "headphone" | "hands-free" | "handset" => Form::Headset,
        "speaker" | "hifi" | "tv" => Form::Speakers,
        "microphone" | "webcam" => Form::Microphone,
        _ if icon.contains("headset") || icon.contains("headphone") => Form::Headset,
        _ if icon.contains("speaker") => Form::Speakers,
        _ if icon.contains("microphone") || icon.contains("camera") => Form::Microphone,
        _ => Form::Card,
    }
}

fn format(spec: &Spec) -> String {
    let rate = if spec.rate % 1000 == 0 {
        format!("{} kHz", spec.rate / 1000)
    } else {
        format!("{:.1} kHz", spec.rate as f32 / 1000.0)
    };
    let channels = match spec.channels {
        1 => "mono".to_owned(),
        2 => "stereo".to_owned(),
        channels => format!("{channels}ch"),
    };
    format!("{rate} · {channels}")
}

/// Island's own meters show up as recording apps; they are left out.
fn ours(name: &str) -> bool {
    name == "island" || name.starts_with("island ")
}

fn app(index: u32, device: u32, stream: Option<&str>, properties: &Proplist, volume: &ChannelVolumes, muted: bool) -> App {
    let stream = stream.unwrap_or_default().to_owned();
    let name = properties.get_str("application.name").unwrap_or_else(|| stream.clone());
    let icon = ["application.icon_name", "application.process.binary"]
        .iter()
        .filter_map(|key| properties.get_str(key))
        .chain([name.to_lowercase().replace(' ', "-")])
        .find_map(|candidate| icons::lookup(&candidate));
    App {
        index,
        name,
        stream,
        icon,
        device,
        volume: level(volume),
        muted,
        channels: volume.len(),
    }
}

#[derive(Default)]
struct Answers {
    sinks: Vec<Device>,
    sources: Vec<Device>,
    defaults: Option<(String, String)>,
    playing: Vec<App>,
    recording: Vec<App>,
}

/// The five lists asked for together; done once every one has answered.
pub struct Query {
    answers: Rc<RefCell<Answers>>,
    remaining: Rc<Cell<u8>>,
}

impl Query {
    pub fn start(context: &Context) -> Self {
        let answers = Rc::new(RefCell::new(Answers::default()));
        let remaining = Rc::new(Cell::new(5u8));
        let introspect = context.introspect();
        {
            let (answers, remaining) = (answers.clone(), remaining.clone());
            introspect.get_sink_info_list(move |result| match result {
                ListResult::Item(info) => answers.borrow_mut().sinks.push(Device {
                    index: info.index,
                    name: info.name.as_deref().unwrap_or_default().to_owned(),
                    description: info.description.as_deref().unwrap_or_default().to_owned(),
                    form: form(&info.proplist),
                    format: format(&info.sample_spec),
                    monitor: info.monitor_source_name.as_deref().unwrap_or_default().to_owned(),
                    volume: level(&info.volume),
                    muted: info.mute,
                    channels: info.volume.len(),
                }),
                ListResult::End | ListResult::Error => remaining.set(remaining.get() - 1),
            });
        }
        {
            let (answers, remaining) = (answers.clone(), remaining.clone());
            introspect.get_source_info_list(move |result| match result {
                ListResult::Item(info) if info.monitor_of_sink.is_none() => answers.borrow_mut().sources.push(Device {
                    index: info.index,
                    name: info.name.as_deref().unwrap_or_default().to_owned(),
                    description: info.description.as_deref().unwrap_or_default().to_owned(),
                    form: form(&info.proplist),
                    format: format(&info.sample_spec),
                    monitor: String::new(),
                    volume: level(&info.volume),
                    muted: info.mute,
                    channels: info.volume.len(),
                }),
                ListResult::Item(_) => {}
                ListResult::End | ListResult::Error => remaining.set(remaining.get() - 1),
            });
        }
        {
            let (answers, remaining) = (answers.clone(), remaining.clone());
            introspect.get_server_info(move |info| {
                answers.borrow_mut().defaults = Some((
                    info.default_sink_name.as_deref().unwrap_or_default().to_owned(),
                    info.default_source_name.as_deref().unwrap_or_default().to_owned(),
                ));
                remaining.set(remaining.get() - 1);
            });
        }
        {
            let (answers, remaining) = (answers.clone(), remaining.clone());
            introspect.get_sink_input_info_list(move |result| match result {
                ListResult::Item(info) => {
                    let app = app(info.index, info.sink, info.name.as_deref(), &info.proplist, &info.volume, info.mute);
                    if !ours(&app.name) {
                        answers.borrow_mut().playing.push(app);
                    }
                }
                ListResult::End | ListResult::Error => remaining.set(remaining.get() - 1),
            });
        }
        {
            let (answers, remaining) = (answers.clone(), remaining.clone());
            introspect.get_source_output_info_list(move |result| match result {
                ListResult::Item(info) => {
                    let app = app(info.index, info.source, info.name.as_deref(), &info.proplist, &info.volume, info.mute);
                    if !ours(&app.name) {
                        answers.borrow_mut().recording.push(app);
                    }
                }
                ListResult::End | ListResult::Error => remaining.set(remaining.get() - 1),
            });
        }
        Self { answers, remaining }
    }

    /// The state, once every answer is in.
    pub fn finish(&self) -> Option<AudioState> {
        if self.remaining.get() > 0 {
            return None;
        }
        let mut answers = std::mem::take(&mut *self.answers.borrow_mut());
        let (default_sink, default_source) = answers.defaults.unwrap_or_default();
        // Recording a sink's monitor is listening to what plays, not using a microphone.
        answers.recording.retain(|app| answers.sources.iter().any(|source| source.index == app.device));
        Some(AudioState {
            connected: true,
            sinks: answers.sinks,
            sources: answers.sources,
            default_sink,
            default_source,
            playing: answers.playing,
            recording: answers.recording,
        })
    }
}

#[cfg(test)]
#[path = "query_test.rs"]
mod tests;
