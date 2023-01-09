#![feature(label_break_value, let_else)]
//! This crate allows users to configure output from
//! [`tracing`](docs.rs/tracing) in the same way as you would configure the
//! output of [`log4rs`](docs.rs/log4rs).
//!
//! # Overview
//!
//! For a usage example see the `examples` folder or `src/test.rs`.
//!
//! ## Benchmarks & Results
//!
//! The takeaway is that the actual appenders are roughly equivalent in
//! performance. However, when using the `tracing` macros vs the `log` macros
//! the appender performance is roughly 2 orders of magnitude larger.
//! See for yourself with `cargo bench --features tracing-macros`

mod appenders;
mod env;
mod handle;

pub mod error;
#[cfg(test)]
mod test;

pub use appenders::Appender;
pub use handle::{
    init_console_logger,
    Handle,
    TraceLogger,
};
pub use trace4rs_config::{
    config,
    config::Config,
    error::Error as ConfigError,
};
