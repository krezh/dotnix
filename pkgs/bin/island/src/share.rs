//! Images on the GPU that workers fill and the frontend draws as they are, scaled as they are
//! drawn: shared with the compositor through dma-buf, so a window's capture goes from one GPU
//! buffer to another and never passes through memory island reads, or uploaded from pixels a
//! worker has. The frontend makes them, [`offer`]ing how.

use std::{
    any::Any,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{Arc, LazyLock, Mutex, OnceLock},
};

use crate::sync::lock;

/// `DRM_FORMAT_ARGB8888` and `DRM_FORMAT_XRGB8888`: four bytes a pixel, blue first in memory.
pub const ARGB8888: u32 = 0x3432_5241;
pub const XRGB8888: u32 = 0x3432_5258;

/// One shared image: what the compositor is told of it, and the frontend's own hold on it.
pub struct Shared {
    pub fd: OwnedFd,
    pub stride: u32,
    pub offset: u32,
    pub modifier: u64,
    pub size: (u32, u32),
    pub image: Image,
}

/// What the frontend draws an image from.
pub type Image = Arc<dyn Any + Send + Sync>;

/// How the frontend makes images.
pub trait Maker: Send + Sync {
    /// An image of `size` in the DRM format `fourcc`, laid out in one of `modifiers`, shared with
    /// the compositor; none when the device cannot share.
    fn make(&self, size: (u32, u32), fourcc: u32, modifiers: &[u64]) -> Option<Shared>;

    /// An image of `size` holding `pixels`, four bytes each, blue first.
    fn upload(&self, size: (u32, u32), pixels: &[u8]) -> Option<Image>;
}

static MAKER: OnceLock<Box<dyn Maker>> = OnceLock::new();
static WAITING: LazyLock<Mutex<Vec<Box<dyn FnOnce() + Send>>>> = LazyLock::new(Mutex::default);

/// The frontend says how it makes images, once its device is open.
pub fn offer(maker: Box<dyn Maker>) {
    let _ = MAKER.set(maker);
    for offered in lock(&WAITING).drain(..) {
        offered();
    }
}

/// Calls `offered` once the frontend has offered, at once if it has.
pub fn on_offer(offered: impl FnOnce() + Send + 'static) {
    let mut waiting = lock(&WAITING);
    if MAKER.get().is_some() {
        drop(waiting);
        offered();
    } else {
        waiting.push(Box::new(offered));
    }
}

/// Whether the frontend has offered yet; until it has, no image can be made.
pub fn can_make() -> bool {
    MAKER.get().is_some()
}

pub fn make(size: (u32, u32), fourcc: u32, modifiers: &[u64]) -> Option<Shared> {
    MAKER.get()?.make(size, fourcc, modifiers)
}

pub fn upload(size: (u32, u32), pixels: &[u8]) -> Option<Image> {
    MAKER.get()?.upload(size, pixels)
}

/// `struct dma_buf_export_sync_file`.
#[repr(C)]
struct ExportSyncFile {
    flags: u32,
    fd: i32,
}

/// `DMA_BUF_IOCTL_EXPORT_SYNC_FILE`: `_IOWR('b', 2, struct dma_buf_export_sync_file)`.
const EXPORT_SYNC_FILE: u64 = 0xC008_6202;
/// The fence waits for whatever writes, before a read.
const SYNC_READ: u32 = 1;

/// Waits until whatever writes into the image the dma-buf `fd` holds is done, so reading it sees
/// all of what was written.
pub fn wait_written(fd: &OwnedFd) {
    let mut export = ExportSyncFile {
        flags: SYNC_READ,
        fd: -1,
    };
    // Fills `fd` with a new sync file on success.
    let asked = unsafe { libc::ioctl(fd.as_raw_fd(), EXPORT_SYNC_FILE as _, &mut export) };
    if asked != 0 || export.fd < 0 {
        return;
    }
    // A new descriptor, which `OwnedFd` then owns.
    let fence = unsafe { OwnedFd::from_raw_fd(export.fd) };
    let mut waiting = libc::pollfd {
        fd: fence.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // The fence lives past the call; it is readable once signalled.
    unsafe { libc::poll(&mut waiting, 1, -1) };
}
