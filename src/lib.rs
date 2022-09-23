#![warn(clippy::all, rust_2018_idioms)]

mod app;

pub use app::AdbApp;
mod commands;

pub use commands::*;
