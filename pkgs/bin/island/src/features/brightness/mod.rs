//! Neither logind nor DDC/CI tells of changes made elsewhere, so the OSD only shows changes made
//! through `island ctl brightness`.

mod device;
mod feature;
mod runtime;
mod state;

pub use runtime::Brightness;
