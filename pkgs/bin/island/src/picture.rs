use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use image::RgbaImage;
use resvg::{tiny_skia, usvg};

use crate::{icons, share::Image, sync::lock};

#[derive(Clone)]
pub enum Picture {
    File(PathBuf),
    /// Blue first.
    Pixels(Arc<RgbaImage>),
    /// Pixels that keep changing, such as a window shown live.
    Live(Arc<Live>),
}

/// What a live picture shows now: an image on the GPU, made by whatever draws it through
/// [`share`](crate::share) and scaled as it is drawn.
#[derive(Clone)]
pub struct Frame {
    pub image: Image,
    pub size: (u32, u32),
    /// Its alpha is to be ignored.
    pub opaque: bool,
}

/// A picture whose pixels its owner replaces as they change: whatever shows it keeps one place
/// for it and redraws only that place, without anything else being laid out again.
pub struct Live {
    frame: Mutex<Frame>,
    version: AtomicU64,
}

impl Live {
    pub fn new(frame: Frame) -> Arc<Self> {
        Arc::new(Self {
            frame: Mutex::new(frame),
            version: AtomicU64::new(0),
        })
    }

    pub fn show(&self, frame: Frame) {
        *lock(&self.frame) = frame;
        self.version.fetch_add(1, Ordering::Release);
    }

    pub fn frame(&self) -> Frame {
        lock(&self.frame).clone()
    }

    /// Counts the images shown.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }
}

impl Picture {
    pub fn pixels(buffer: RgbaImage) -> Self {
        Self::Pixels(Arc::new(buffer))
    }

    /// The picture in the file at `path`, read now rather than as it is first drawn, so a worker
    /// can take that off whatever draws it.
    pub fn read(path: &Path) -> Option<Self> {
        let mut image = decode(path)?;
        swap_red_blue(&mut image);
        Some(Self::pixels(image))
    }
}

/// Turns pixels blue first from red first, or back.
pub fn swap_red_blue(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

/// SVGs are drawn at twice their own size, so they stay sharp shown larger.
const SVG_DETAIL: f32 = 2.0;

/// The picture in the file at `path`, red first.
pub fn decode(path: &Path) -> Option<RgbaImage> {
    if !icons::is_svg(path) {
        return image::open(path).ok().map(|image| image.to_rgba8());
    }
    let tree = usvg::Tree::from_data(&std::fs::read(path).ok()?, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let (width, height) = (size.width() * SVG_DETAIL, size.height() * SVG_DETAIL);
    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(SVG_DETAIL, SVG_DETAIL),
        &mut pixmap.as_mut(),
    );
    let (width, height) = (pixmap.width(), pixmap.height());
    let mut rgba = pixmap.take();
    for pixel in rgba.chunks_exact_mut(4) {
        if pixel[3] > 0 {
            let alpha = f32::from(pixel[3]) / 255.0;
            for channel in &mut pixel[..3] {
                *channel = (f32::from(*channel) / alpha) as u8;
            }
        }
    }
    RgbaImage::from_raw(width, height, rgba)
}

/// Pixel images compare by identity: decoding them again makes a new image, even when the pixels
/// are the same.
impl PartialEq for Picture {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Picture::File(left), Picture::File(right)) => left == right,
            (Picture::Pixels(left), Picture::Pixels(right)) => Arc::ptr_eq(left, right),
            (Picture::Live(left), Picture::Live(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl fmt::Debug for Picture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Picture::File(path) => formatter.debug_tuple("File").field(path).finish(),
            Picture::Pixels(image) => formatter
                .debug_tuple("Pixels")
                .field(&Arc::as_ptr(image))
                .finish(),
            Picture::Live(live) => formatter
                .debug_tuple("Live")
                .field(&Arc::as_ptr(live))
                .finish(),
        }
    }
}
