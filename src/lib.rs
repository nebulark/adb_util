#![warn(clippy::all, rust_2018_idioms)]

mod app;

pub use app::AirApp;
mod commands;

pub use commands::*;
