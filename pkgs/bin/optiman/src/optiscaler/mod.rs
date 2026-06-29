mod downloader;
mod manager;
pub mod github;

pub use github::{GitHubClient, Release};
pub use manager::Manager;
