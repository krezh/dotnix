use std::{fmt, path::PathBuf, sync::Arc};

use image::RgbaImage;

#[derive(Clone)]
pub enum Picture {
    File(PathBuf),
    /// Blue first.
    Pixels(Arc<RgbaImage>),
}

impl Picture {
    pub fn pixels(buffer: RgbaImage) -> Self {
        Self::Pixels(Arc::new(buffer))
    }
}

/// Pixel images compare by identity: decoding them again makes a new image, even when the pixels
/// are the same.
impl PartialEq for Picture {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Picture::File(left), Picture::File(right)) => left == right,
            (Picture::Pixels(left), Picture::Pixels(right)) => Arc::ptr_eq(left, right),
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
        }
    }
}
