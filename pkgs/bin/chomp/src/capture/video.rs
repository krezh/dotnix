//! Native screen recording management.

use anyhow::{Context, Result};
use ffmpeg_next as ffmpeg;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use wayland_client::Connection;

use super::video_ipc::{self, RecordingCommand, RecordingListener, RecordingReply};
use crate::cli::{RecordingOptions, Settings};
use crate::compositor::{
    Screencopy, get_outputs,
    protocol::{VideoCapture, outputs::find_outputs_for_rect},
};
use crate::render::Rect;
use crate::replay::hardware::HardwareEncoder;

const TARGET_BITS_PER_PIXEL: f64 = 0.35;
const MIN_BITRATE_MB: u64 = 5;
const MAX_BITRATE_MB: u64 = 25;

pub fn recording(_settings: &Settings) -> Result<Option<PathBuf>> {
    if !video_ipc::socket_exists()? {
        return Ok(None);
    }
    match video_ipc::request(RecordingCommand::Status)? {
        RecordingReply::Status { output } => Ok(Some(output)),
        RecordingReply::Stopped { .. } => {
            anyhow::bail!("Recorder returned an invalid status reply")
        }
    }
}

pub fn stop_recording(_settings: &Settings) -> Result<PathBuf> {
    anyhow::ensure!(video_ipc::socket_exists()?, "No recording active");
    match video_ipc::request(RecordingCommand::Stop)? {
        RecordingReply::Stopped { output } => Ok(output),
        RecordingReply::Status { .. } => anyhow::bail!("Recorder returned an invalid stop reply"),
    }
}

pub fn start_recording(
    _settings: &Settings,
    geometry: Option<&str>,
    monitor: Option<&str>,
    output_file: &Path,
) -> Result<()> {
    if video_ipc::socket_exists()? {
        if let Ok(RecordingReply::Status { output }) = video_ipc::request(RecordingCommand::Status)
        {
            anyhow::bail!("A recording is already active at {}", output.display());
        }
    }

    let executable = std::env::current_exe().context("Failed to locate the chomp executable")?;
    let output_file = absolute_path(output_file)?;
    let mut command = Command::new(&executable);
    command
        .arg("--recording-service")
        .arg("--recording-output")
        .arg(&output_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    if let Some(geometry) = geometry {
        command.arg("--recording-geometry").arg(geometry);
    }
    if let Some(monitor) = monitor {
        command.arg("--recording-monitor").arg(monitor);
    }
    let mut child = command
        .spawn()
        .context("Failed to start the native recorder")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child
            .try_wait()
            .context("Failed to inspect recorder startup")?
        {
            anyhow::bail!("Native recorder exited during startup with {status}");
        }
        if let Ok(RecordingReply::Status { output }) = video_ipc::request(RecordingCommand::Status)
        {
            if output == output_file {
                return Ok(());
            }
            let _ = child.kill();
            anyhow::bail!("A recording is already active at {}", output.display());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            anyhow::bail!("Native recorder did not become ready within 10s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn run_recording_service(
    settings: Settings,
    geometry: Option<String>,
    monitor: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let connection = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let outputs = get_outputs(&connection)?;
    let target = match (geometry, monitor) {
        (Some(geometry), None) => Rect::from_geometry_string(&geometry)?,
        (None, Some(monitor)) => {
            outputs
                .iter()
                .find(|output| output.name == monitor)
                .with_context(|| format!("Output {monitor} was not found"))?
                .logical
        }
        _ => anyhow::bail!("The native recorder requires one geometry or output"),
    };
    let covering = find_outputs_for_rect(&outputs, target)?;
    if covering.len() == 1
        && covering[0].0.transform == wayland_client::protocol::wl_output::Transform::Normal
    {
        let (output_info, overlap) = covering[0];
        let local = Rect::new(
            overlap.x - output_info.logical.x,
            overlap.y - output_info.logical.y,
            overlap.width,
            overlap.height,
        );
        match record_dmabuf(&connection, output_info, local, &settings, &output) {
            Ok(()) => Ok(()),
            Err(error) => {
                log::warn!("DMA-BUF recording unavailable, using SHM capture: {error:#}");
                record_composited(&connection, &outputs, target, &settings, &output)
            }
        }
    } else {
        record_composited(&connection, &outputs, target, &settings, &output)
    }
}

fn record_dmabuf(
    connection: &Connection,
    output_info: &crate::compositor::protocol::outputs::OutputInfo,
    local: Rect,
    settings: &Settings,
    output: &Path,
) -> Result<()> {
    let mut capture = VideoCapture::new(connection)?;
    let (width, height, fourcc) = capture.probe_region(&output_info.output, local)?;
    let capture_format = drm_pixel_format(fourcc)?;
    let (output_width, output_height) = output_dimensions(&settings.recording, width, height)?;
    let bitrate = resolve_bitrate_bytes(&settings.recording, output_width, output_height)?;
    let mut encoder = HardwareEncoder::new_with_options(
        width,
        height,
        output_width,
        output_height,
        settings.recording.max_fps,
        bitrate * 8,
        capture_format,
        codec_name(&settings.recording.codec)?,
        Path::new(&settings.replay.dri_device),
    )?;
    let mut writer = VideoWriter::new(output, encoder.parameters(), encoder.time_base())?;
    let control = RecordingListener::bind()?;
    let frame_interval = Duration::from_secs_f64(1.0 / settings.recording.max_fps as f64);
    let mut stop_client = loop {
        if let Some(client) = control.poll(output)? {
            break client;
        }
        let started = Instant::now();
        let frame = encoder.allocate_capture_frame()?;
        let (timestamp, y_inverted) = capture.capture_region(&output_info.output, local, &frame)?;
        writer.write(encoder.encode(frame, timestamp, y_inverted)?)?;
        if let Some(remaining) = frame_interval.checked_sub(started.elapsed()) {
            std::thread::sleep(remaining);
        }
    };
    finish_recording(&mut encoder, writer, &mut stop_client, output)
}

fn record_composited(
    connection: &Connection,
    outputs: &[crate::compositor::protocol::outputs::OutputInfo],
    target: Rect,
    settings: &Settings,
    output: &Path,
) -> Result<()> {
    let mut capture = Screencopy::new(connection)?;
    let first = super::capture_region(&mut capture, outputs, target)?;
    let (output_width, output_height) =
        output_dimensions(&settings.recording, first.width, first.height)?;
    let bitrate = resolve_bitrate_bytes(&settings.recording, output_width, output_height)?;
    let mut encoder = HardwareEncoder::new_with_options(
        first.width,
        first.height,
        output_width,
        output_height,
        settings.recording.max_fps,
        bitrate * 8,
        ffmpeg::format::Pixel::BGRA,
        codec_name(&settings.recording.codec)?,
        Path::new(&settings.replay.dri_device),
    )?;
    let mut writer = VideoWriter::new(output, encoder.parameters(), encoder.time_base())?;
    let control = RecordingListener::bind()?;
    let frame_interval = Duration::from_secs_f64(1.0 / settings.recording.max_fps as f64);
    let epoch = Instant::now();
    writer.write(encoder.encode_image(&first, 0)?)?;
    let mut stop_client = loop {
        if let Some(client) = control.poll(output)? {
            break client;
        }
        let started = Instant::now();
        let image = super::capture_region(&mut capture, outputs, target)?;
        writer.write(encoder.encode_image(&image, epoch.elapsed().as_micros() as i64)?)?;
        if let Some(remaining) = frame_interval.checked_sub(started.elapsed()) {
            std::thread::sleep(remaining);
        }
    };
    finish_recording(&mut encoder, writer, &mut stop_client, output)
}

fn finish_recording(
    encoder: &mut HardwareEncoder,
    mut writer: VideoWriter,
    stop_client: &mut std::os::unix::net::UnixStream,
    output: &Path,
) -> Result<()> {
    let result = (|| {
        writer.write(encoder.finish()?)?;
        writer.finish()
    })();
    let reply = video_ipc::reply_stopped(stop_client, &result, output);
    result?;
    reply
}

struct VideoWriter {
    context: ffmpeg::format::context::Output,
    stream_index: usize,
    input_time_base: ffmpeg::Rational,
    first_timestamp: Option<i64>,
    output: PathBuf,
    temporary: PathBuf,
    finished: bool,
}

impl VideoWriter {
    fn new(
        output: &Path,
        parameters: ffmpeg::codec::Parameters,
        input_time_base: ffmpeg::Rational,
    ) -> Result<Self> {
        anyhow::ensure!(
            output
                .extension()
                .is_some_and(|extension| extension == "mp4"),
            "Recording output must use the .mp4 extension"
        );
        if let Some(parent) = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        let temporary = output.with_extension(format!("mp4.part-{}", std::process::id()));
        let mut context = ffmpeg::format::output_as(&temporary, "mp4")
            .context("Failed to create recording MP4")?;
        let mut stream = context
            .add_stream(ffmpeg::encoder::find(parameters.id()))
            .context("Failed to add recording video stream")?;
        stream.set_parameters(parameters);
        stream.set_time_base(input_time_base);
        let stream_index = stream.index();
        context
            .write_header()
            .context("Failed to write MP4 header")?;
        Ok(Self {
            context,
            stream_index,
            input_time_base,
            first_timestamp: None,
            output: output.to_path_buf(),
            temporary,
            finished: false,
        })
    }

    fn write(&mut self, packets: Vec<ffmpeg::Packet>) -> Result<()> {
        let output_time_base = self
            .context
            .stream(self.stream_index)
            .context("Recording stream disappeared")?
            .time_base();
        for mut packet in packets {
            let timestamp = packet.dts().or_else(|| packet.pts()).unwrap_or(0);
            let first = *self.first_timestamp.get_or_insert(timestamp);
            packet.set_pts(packet.pts().map(|value| value - first));
            packet.set_dts(packet.dts().map(|value| value - first));
            packet.set_stream(self.stream_index);
            packet.rescale_ts(self.input_time_base, output_time_base);
            packet.set_position(-1);
            packet
                .write_interleaved(&mut self.context)
                .context("Failed to write recording packet")?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<()> {
        self.context
            .write_trailer()
            .context("Failed to finish recording MP4")?;
        OpenOptions::new()
            .read(true)
            .open(&self.temporary)?
            .sync_all()?;
        fs::rename(&self.temporary, &self.output)
            .with_context(|| format!("Failed to publish {}", self.output.display()))?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for VideoWriter {
    fn drop(&mut self) {
        if !self.finished {
            let _ = fs::remove_file(&self.temporary);
        }
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir()
        .context("Failed to resolve the current directory")?
        .join(path))
}

fn codec_name(codec: &str) -> Result<&'static str> {
    match codec {
        "" | "auto" | "avc" => Ok("h264_vaapi"),
        "hevc" => Ok("hevc_vaapi"),
        "vp8" => Ok("vp8_vaapi"),
        "vp9" => Ok("vp9_vaapi"),
        "av1" => Ok("av1_vaapi"),
        other => anyhow::bail!("Unsupported video codec {other}"),
    }
}

fn output_dimensions(options: &RecordingOptions, width: u32, height: u32) -> Result<(u32, u32)> {
    if options.encode_resolution.trim().is_empty() {
        return Ok((width, height));
    }
    parse_resolution(&options.encode_resolution)
        .with_context(|| format!("Invalid encoder resolution {}", options.encode_resolution))
}

fn drm_pixel_format(fourcc: u32) -> Result<ffmpeg::format::Pixel> {
    use drm::buffer::DrmFourcc;
    match DrmFourcc::try_from(fourcc)
        .map_err(|_| anyhow::anyhow!("Unknown DRM format {fourcc:#x}"))?
    {
        DrmFourcc::Xrgb8888 => Ok(ffmpeg::format::Pixel::BGRZ),
        DrmFourcc::Argb8888 => Ok(ffmpeg::format::Pixel::BGRA),
        format => anyhow::bail!("Unsupported DMA-BUF capture format {format:?}"),
    }
}

fn resolve_bitrate(options: &RecordingOptions, width: u32, height: u32) -> String {
    if !options.bitrate.is_empty() {
        return options.bitrate.clone();
    }
    let pixels_per_second = width as f64 * height as f64 * options.max_fps as f64;
    let bytes_per_second = pixels_per_second * TARGET_BITS_PER_PIXEL / 8.0;
    let megabytes = (bytes_per_second / 1_000_000.0).round() as u64;
    format!("{} MB", megabytes.clamp(MIN_BITRATE_MB, MAX_BITRATE_MB))
}

fn resolve_bitrate_bytes(options: &RecordingOptions, width: u32, height: u32) -> Result<usize> {
    let value = resolve_bitrate(options, width, height);
    let mut parts = value.split_whitespace();
    let amount: f64 = parts
        .next()
        .context("Missing bitrate value")?
        .parse()
        .context("Invalid bitrate value")?;
    let multiplier = match parts.next().unwrap_or("B").to_ascii_lowercase().as_str() {
        "b" => 1.0,
        "kb" => 1_000.0,
        "mb" => 1_000_000.0,
        other => anyhow::bail!("Unsupported bitrate unit {other}"),
    };
    Ok((amount * multiplier) as usize)
}

fn parse_resolution(resolution: &str) -> Option<(u32, u32)> {
    let (width, height) = resolution.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

#[cfg(test)]
#[path = "video_test.rs"]
mod tests;
