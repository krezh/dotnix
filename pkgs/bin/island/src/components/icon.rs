use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::{AnyElement, ObjectFit, RenderImage, SharedString, img, prelude::*, px, rgb, svg};

use crate::icons;

#[derive(Clone)]
pub enum Picture {
    File(PathBuf),
    Pixels(Arc<RenderImage>),
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
            Picture::Pixels(image) => formatter.debug_tuple("Pixels").field(&image.id).finish(),
        }
    }
}

pub fn picture(picture: &Picture, size: f32, fit: ObjectFit) -> AnyElement {
    rounded(picture, size, fit, 0.0)
}

pub fn rounded(picture: &Picture, size: f32, fit: ObjectFit, radius: f32) -> AnyElement {
    match picture {
        Picture::File(path) => img(path.clone())
            .size(px(size))
            .object_fit(fit)
            .rounded(px(radius))
            .into_any_element(),
        Picture::Pixels(image) => img(image.clone())
            .size(px(size))
            .object_fit(fit)
            .rounded(px(radius))
            .into_any_element(),
    }
}

pub fn glyph(path: &Path, size: f32, color: u32) -> AnyElement {
    if icons::is_svg(path) {
        svg()
            .path(SharedString::from(path.to_string_lossy().into_owned()))
            .size(px(size))
            .text_color(rgb(color))
            .into_any_element()
    } else {
        img(path.to_path_buf()).size(px(size)).into_any_element()
    }
}

pub fn image(path: &Path, size: f32) -> AnyElement {
    img(path.to_path_buf())
        .size(px(size))
        .object_fit(ObjectFit::Contain)
        .into_any_element()
}

pub fn tray(icon: &Picture, size: f32) -> AnyElement {
    picture(icon, size, ObjectFit::Contain)
}
