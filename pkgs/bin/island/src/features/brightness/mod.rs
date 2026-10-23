//! Neither logind nor DDC/CI tells of changes made elsewhere, so a screen's level is as island last
//! read or set it.

mod device;
mod feature;
mod runtime;
mod state;

pub use runtime::Brightness;
