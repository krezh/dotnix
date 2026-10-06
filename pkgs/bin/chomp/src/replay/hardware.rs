use anyhow::{Context, Result};
use ffmpeg::{Dictionary, Packet, Rational, codec, encoder, filter, format::Pixel, frame};
use ffmpeg_next as ffmpeg;
use ffmpeg_sys_next::{
    AV_HWFRAME_MAP_WRITE, AVBufferRef, AVDRMFrameDescriptor, AVHWDeviceType, AVHWFramesContext,
    AVPixelFormat, av_buffer_ref, av_buffer_unref, av_buffersrc_parameters_alloc,
    av_buffersrc_parameters_set, av_free, av_hwdevice_ctx_create, av_hwframe_ctx_alloc,
    av_hwframe_ctx_init, av_hwframe_get_buffer, av_hwframe_map,
};
use std::ffi::CString;
use std::path::Path;
use std::ptr::null_mut;

pub struct HardwareFrame {
    surface: frame::Video,
    mapping: frame::Video,
    width: u32,
    height: u32,
}

impl HardwareFrame {
    pub fn descriptor(&self) -> &AVDRMFrameDescriptor {
        unsafe { &*((*self.mapping.as_ptr()).data[0] as *const AVDRMFrameDescriptor) }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

struct HardwareDevice {
    pointer: *mut AVBufferRef,
}

impl HardwareDevice {
    fn open(path: &Path) -> Result<Self> {
        let path = CString::new(path.to_string_lossy().as_bytes())?;
        let mut pointer = null_mut();
        let result = unsafe {
            av_hwdevice_ctx_create(
                &mut pointer,
                AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
                path.as_ptr(),
                null_mut(),
                0,
            )
        };
        anyhow::ensure!(
            result == 0,
            "Failed to open VA-API device: {}",
            ffmpeg::Error::from(result)
        );
        Ok(Self { pointer })
    }

    fn frames(&mut self, format: Pixel, width: u32, height: u32) -> Result<HardwareFrames> {
        let pointer = unsafe { av_hwframe_ctx_alloc(self.pointer) };
        anyhow::ensure!(
            !pointer.is_null(),
            "Failed to allocate VA-API frame context"
        );
        let context = unsafe { &mut *((*pointer).data as *mut AVHWFramesContext) };
        context.format = AVPixelFormat::AV_PIX_FMT_VAAPI;
        context.sw_format = format.into();
        context.width = width as i32;
        context.height = height as i32;
        context.initial_pool_size = 5;
        let result = unsafe { av_hwframe_ctx_init(pointer) };
        if result != 0 {
            let mut pointer = pointer;
            unsafe { av_buffer_unref(&mut pointer) };
            anyhow::bail!(
                "Failed to initialize VA-API frames: {}",
                ffmpeg::Error::from(result)
            );
        }
        Ok(HardwareFrames { pointer })
    }
}

impl Drop for HardwareDevice {
    fn drop(&mut self) {
        unsafe { av_buffer_unref(&mut self.pointer) };
    }
}

struct HardwareFrames {
    pointer: *mut AVBufferRef,
}

impl HardwareFrames {
    fn allocate(&mut self) -> Result<frame::Video> {
        let mut frame = frame::Video::empty();
        let result = unsafe { av_hwframe_get_buffer(self.pointer, frame.as_mut_ptr(), 0) };
        anyhow::ensure!(
            result == 0,
            "Failed to allocate VA-API frame: {}",
            ffmpeg::Error::from(result)
        );
        Ok(frame)
    }
}

impl Drop for HardwareFrames {
    fn drop(&mut self) {
        unsafe { av_buffer_unref(&mut self.pointer) };
    }
}

pub struct HardwareEncoder {
    _device: HardwareDevice,
    capture_frames: HardwareFrames,
    _encode_frames: HardwareFrames,
    normal_filter: filter::Graph,
    inverted_filter: filter::Graph,
    encoder: ffmpeg::encoder::video::Encoder,
    parameters: codec::Parameters,
    time_base: Rational,
    width: u32,
    height: u32,
}

impl HardwareEncoder {
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        bitrate_bits: usize,
        capture_format: Pixel,
        device_path: &Path,
    ) -> Result<Self> {
        ffmpeg::init().context("Failed to initialize FFmpeg")?;
        let mut device = HardwareDevice::open(device_path)?;
        let mut capture_frames = device.frames(capture_format, width, height)?;
        let encode_frames = device.frames(Pixel::NV12, width, height)?;
        let normal_filter = create_filter(&mut capture_frames, width, height, false)?;
        let inverted_filter = create_filter(&mut capture_frames, width, height, true)?;
        let codec = encoder::find_by_name("h264_vaapi").context("h264_vaapi is unavailable")?;
        let time_base = Rational(1, 1_000_000);
        let mut video = codec::context::Context::new_with_codec(codec)
            .encoder()
            .video()
            .context("Failed to create VA-API encoder")?;
        video.set_width(width);
        video.set_height(height);
        video.set_format(Pixel::VAAPI);
        video.set_time_base(time_base);
        video.set_frame_rate(Some(Rational(fps as i32, 1)));
        video.set_bit_rate(bitrate_bits);
        video.set_gop(fps * 2);
        video.set_max_b_frames(0);
        video.set_flags(codec::Flags::GLOBAL_HEADER);
        unsafe {
            (*video.as_mut_ptr()).hw_device_ctx = av_buffer_ref(device.pointer);
            (*video.as_mut_ptr()).hw_frames_ctx = av_buffer_ref(encode_frames.pointer);
            (*video.as_mut_ptr()).sw_pix_fmt = AVPixelFormat::AV_PIX_FMT_NV12;
        }
        let mut options = Dictionary::new();
        options.set("low_power", "0");
        let encoder = video
            .open_with(options)
            .context("Failed to open h264_vaapi")?;
        let parameters = codec::Parameters::from(&encoder);

        Ok(Self {
            _device: device,
            capture_frames,
            _encode_frames: encode_frames,
            normal_filter,
            inverted_filter,
            encoder,
            parameters,
            time_base,
            width,
            height,
        })
    }

    pub fn allocate_capture_frame(&mut self) -> Result<HardwareFrame> {
        let surface = self.capture_frames.allocate()?;
        let mut mapping = frame::Video::empty();
        mapping.set_format(Pixel::DRM_PRIME);
        let result = unsafe {
            av_hwframe_map(
                mapping.as_mut_ptr(),
                surface.as_ptr(),
                AV_HWFRAME_MAP_WRITE as i32,
            )
        };
        anyhow::ensure!(
            result == 0,
            "Failed to export VA-API frame: {}",
            ffmpeg::Error::from(result)
        );
        Ok(HardwareFrame {
            surface,
            mapping,
            width: self.width,
            height: self.height,
        })
    }

    pub fn encode(
        &mut self,
        mut frame: HardwareFrame,
        timestamp_micros: i64,
        y_inverted: bool,
    ) -> Result<Vec<Packet>> {
        drop(frame.mapping);
        frame.surface.set_pts(Some(timestamp_micros));
        let filter = if y_inverted {
            &mut self.inverted_filter
        } else {
            &mut self.normal_filter
        };
        filter
            .get("in")
            .context("VA-API filter source disappeared")?
            .source()
            .add(&frame.surface)
            .context("Failed to submit captured VA-API frame")?;

        let mut filtered = frame::Video::empty();
        loop {
            match filter
                .get("out")
                .context("VA-API filter sink disappeared")?
                .sink()
                .frame(&mut filtered)
            {
                Ok(()) => self
                    .encoder
                    .send_frame(&filtered)
                    .context("Failed to submit converted VA-API frame")?,
                Err(ffmpeg::Error::Other { errno }) if errno == libc::EAGAIN => break,
                Err(error) => return Err(error).context("Failed to receive filtered VA-API frame"),
            }
        }

        let mut packets = Vec::new();
        loop {
            let mut packet = Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    packet.set_time_base(self.time_base);
                    packets.push(packet);
                }
                Err(ffmpeg::Error::Other { errno }) if errno == libc::EAGAIN => break,
                Err(error) => return Err(error).context("Failed to receive encoded packet"),
            }
        }
        Ok(packets)
    }

    pub fn parameters(&self) -> codec::Parameters {
        self.parameters.clone()
    }

    pub fn time_base(&self) -> Rational {
        self.time_base
    }
}

fn create_filter(
    frames: &mut HardwareFrames,
    width: u32,
    height: u32,
    y_inverted: bool,
) -> Result<filter::Graph> {
    let mut graph = filter::Graph::new();
    unsafe {
        let source = ffmpeg_sys_next::avfilter_graph_alloc_filter(
            graph.as_mut_ptr(),
            filter::find("buffer")
                .context("FFmpeg buffer filter unavailable")?
                .as_mut_ptr(),
            c"in".as_ptr(),
        );
        anyhow::ensure!(!source.is_null(), "Failed to allocate VA-API filter source");
        let parameters = av_buffersrc_parameters_alloc();
        anyhow::ensure!(
            !parameters.is_null(),
            "Failed to allocate filter parameters"
        );
        (*parameters).width = width as i32;
        (*parameters).height = height as i32;
        (*parameters).format = AVPixelFormat::AV_PIX_FMT_VAAPI as i32;
        (*parameters).time_base.num = 1;
        (*parameters).time_base.den = 1_000_000;
        (*parameters).hw_frames_ctx = frames.pointer;
        let result = av_buffersrc_parameters_set(source, parameters);
        av_free(parameters.cast());
        anyhow::ensure!(result == 0, "Failed to configure VA-API filter source");
        let result = ffmpeg_sys_next::avfilter_init_dict(source, null_mut());
        anyhow::ensure!(result == 0, "Failed to initialize VA-API filter source");
    }
    graph
        .add(
            &filter::find("buffersink").context("FFmpeg buffer sink unavailable")?,
            "out",
            "pixel_formats=vaapi",
        )
        .context("Failed to create VA-API filter sink")?;
    let chain = if y_inverted {
        "scale_vaapi=format=nv12,transpose_vaapi=dir=vflip"
    } else {
        "scale_vaapi=format=nv12"
    };
    graph.output("in", 0)?.input("out", 0)?.parse(chain)?;
    graph.validate().context("Invalid VA-API filter graph")?;
    Ok(graph)
}
