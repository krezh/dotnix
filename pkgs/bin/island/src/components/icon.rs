//! Drawing icons and pictures: tinted symbolic glyphs, full-color images, pixel buffers.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Img, ImgResourceLoader, ObjectFit, RenderImage, Resource, SharedString, img,
    prelude::*, px, rgb, svg,
};
use image::{Frame, RgbaImage};

use crate::icons;
pub use crate::picture::Picture;

const KEPT_FILES: usize = 24;
const UNUSED_FOR: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Drawn {
    pixels: HashMap<usize, (Arc<RgbaImage>, Arc<RenderImage>, Option<Instant>)>,
    files: VecDeque<PathBuf>,
}

impl Drawn {
    fn pixels(&mut self, image: &Arc<RgbaImage>) -> Arc<RenderImage> {
        self.pixels
            .entry(Arc::as_ptr(image) as usize)
            .or_insert_with(|| {
                let frame = Frame::new(image.as_ref().clone());
                (image.clone(), Arc::new(RenderImage::new(vec![frame])), None)
            })
            .1
            .clone()
    }

    fn file(&mut self, path: &Path) {
        if self.files.back().is_some_and(|last| last == path) {
            return;
        }
        self.files.retain(|file| file != path);
        self.files.push_back(path.to_path_buf());
    }

    fn sweep(&mut self, now: Instant) -> (Vec<Arc<RenderImage>>, Vec<PathBuf>) {
        let mut unused = Vec::new();
        self.pixels.retain(|_, (source, image, idle)| {
            if Arc::strong_count(source) > 1 {
                *idle = None;
                return true;
            }
            match *idle {
                Some(since) if now.duration_since(since) >= UNUSED_FOR => {
                    unused.push(image.clone());
                    false
                }
                Some(_) => true,
                None => {
                    *idle = Some(now);
                    true
                }
            }
        });
        let over = self.files.len().saturating_sub(KEPT_FILES);
        (unused, self.files.drain(..over).collect())
    }
}

thread_local! {
    static DRAWN: RefCell<Drawn> = RefCell::default();
}

pub fn release(cx: &mut App) {
    let (images, files) = DRAWN.with_borrow_mut(|drawn| drawn.sweep(Instant::now()));
    for image in images {
        cx.drop_image(image, None);
    }
    for path in files {
        let resource = Resource::Path(path.into());
        if let Some(Ok(image)) = cx.fetch_asset::<ImgResourceLoader>(&resource) {
            cx.drop_image(image, None);
        }
        cx.remove_asset::<ImgResourceLoader>(&resource);
    }
}

/// Panels only: its atlas tile can be freed, and a bar widget's cached drawing would still use it.
pub fn art(path: PathBuf) -> Img {
    DRAWN.with_borrow_mut(|drawn| drawn.file(&path));
    img(path)
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
        Picture::Pixels(image) => {
            let image = DRAWN.with_borrow_mut(|drawn| drawn.pixels(image));
            img(image)
                .size(px(size))
                .object_fit(fit)
                .rounded(px(radius))
                .into_any_element()
        }
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

#[cfg(test)]
#[path = "icon_test.rs"]
mod tests;
