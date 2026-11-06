#[cfg(feature = "desktop")]
mod commands;
pub mod domain;
pub mod downloads;
pub mod jobs;
pub mod media;
pub mod paths;
pub mod planner;
pub mod process;
pub mod segments;
pub mod storage;
pub mod tags;

#[cfg(feature = "desktop")]
pub fn run() {
    commands::run();
}
