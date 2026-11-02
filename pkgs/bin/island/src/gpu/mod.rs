//! Draws the islands with wgpu instead of GPUI, chosen with `frontend` in the config or the flag.

mod app;
mod blocks;
mod chrome;
mod cover;
mod island;
mod menu;
mod node;
mod paint;
mod panel;
mod records;
mod renderer;
mod turn;
mod widget;
mod wrap;

pub use app::run;
