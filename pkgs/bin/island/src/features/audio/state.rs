use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Output,
    Input,
}

/// What a device looks like, for its icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Form {
    Headset,
    Speakers,
    Microphone,
    #[default]
    Card,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Device {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub form: Form,
    /// Such as "48 kHz · stereo".
    pub format: String,
    /// The source that hears a sink; empty for a source.
    pub monitor: String,
    pub volume: f32,
    pub muted: bool,
    pub channels: u8,
}

/// An app playing to a sink or recording from a source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct App {
    pub index: u32,
    pub name: String,
    /// The stream's own name, such as "Playback", telling one app's streams apart.
    pub stream: String,
    pub icon: Option<PathBuf>,
    /// The index of the sink or source it is on.
    pub device: u32,
    pub volume: f32,
    pub muted: bool,
    pub channels: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Channel {
    pub volume: f32,
    pub muted: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioState {
    /// While not, there are no devices.
    pub connected: bool,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,
    pub default_sink: String,
    pub default_source: String,
    pub playing: Vec<App>,
    pub recording: Vec<App>,
}

impl AudioState {
    pub fn devices(&self, target: Target) -> &[Device] {
        match target {
            Target::Output => &self.sinks,
            Target::Input => &self.sources,
        }
    }

    pub fn default_device(&self, target: Target) -> Option<&Device> {
        let name = match target {
            Target::Output => &self.default_sink,
            Target::Input => &self.default_source,
        };
        self.devices(target).iter().find(|device| &device.name == name)
    }

    pub fn default_device_mut(&mut self, target: Target) -> Option<&mut Device> {
        let (devices, name) = match target {
            Target::Output => (&mut self.sinks, &self.default_sink),
            Target::Input => (&mut self.sources, &self.default_source),
        };
        devices.iter_mut().find(|device| &device.name == name)
    }

    pub fn channel(&self, target: Target) -> Channel {
        self.default_device(target).map_or_else(Channel::default, |device| Channel {
            volume: device.volume,
            muted: device.muted,
        })
    }

    pub fn sink(&self, index: u32) -> Option<&Device> {
        self.sinks.iter().find(|sink| sink.index == index)
    }
}
