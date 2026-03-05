//! # Application logger module
//! **Purpose**: This module configures and initialises the global `tracing` subscriber for the
//! application. It routes all log output to a rotating file via a non-blocking writer.
//! Terminal helpers (`log_success`, `log_error`) are provided as lightweight wrappers for
//! early-startup or pre-subscriber output where `tracing` is not yet active.
//!
//! ## Exported functions
//! * [`configure_logger`] — Full logger setup: renames any existing log file, opens a fresh
//!   truncated log file, attaches a non-blocking `tracing_appender` writer, builds a
//!   pretty-formatted `tracing_subscriber` layer filtered to `llava_core=trace`, and registers
//!   it as the global default subscriber. Returns a [`WorkerGuard`] that **must be kept alive**
//!   for the lifetime of the application — dropping it flushes and closes the background writer
//! * [`log_success`] — debug functions just printing to the console just prettier
//! * [`log_error`] — = debug functions just printing to the console just prettier
//!
//! ## Key design decisions
//! Each application run starts with a fresh log file: `rename_log_file` renames any existing
//! `app.log` to `log_{timestamp}` before a new file is opened, giving per-run log history
//! without an external log rotation daemon. The `EnvFilter` is hardcoded to
//! `off,llava_core=trace` so that noisy dependency crates are silenced while all internal spans
//! and events are captured at full verbosity. ANSI colours are enabled (`with_ansi(true)`) —
//! set to `false` if the log files are consumed by tooling that does not handle escape codes.
//! The non-blocking writer offloads I/O to a background thread to avoid blocking hot paths.
//!
//! ## Dependencies
//! - `tracing` — Span and event macros used across the whole codebase
//! - `tracing_subscriber` — `Registry`, `fmt` layer, and `EnvFilter` for subscriber composition
//! - `tracing_appender` — Non-blocking file writer and [`WorkerGuard`]
//! - `anyhow` — `.context()` propagation for subscriber registration and path errors
use anyhow::Context;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::{fmt::Debug, fs::OpenOptions};
use tracing_subscriber::Layer;
use tracing_subscriber::{Registry, fmt, layer::SubscriberExt};

pub fn log_success(log_content: &str) {
    println!("✅ {}", log_content)
}

pub fn log_error<T>(log_content: &str, error: T)
where
    T: Debug,
{
    eprintln!("❌ {:?}, {}", error, log_content)
}

pub fn configure_logger(
    path_to_log_file: &PathBuf,
) -> Result<tracing_appender::non_blocking::WorkerGuard, crate::errors::Error> {
    rename_log_file(&path_to_log_file)?;
    let filter = tracing_subscriber::filter::EnvFilter::new("off,llava_core=trace");
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path_to_log_file)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(log_file);
    let subscriber = Registry::default().with(
        fmt::layer()
            .with_writer(non_blocking)
            .pretty()
            .with_ansi(true) //set as false if colors are useless
            .with_line_number(true)
            .with_file(true)
            .with_level(true)
            .with_filter(filter),
    );
    tracing::subscriber::set_global_default(subscriber)
        .context("couldnt set subscriber to global default")?;

    Ok(guard)
}

fn rename_log_file(logger_file_path: &PathBuf) -> Result<(), crate::errors::Error> {
    let parent_path = logger_file_path
        .parent()
        .context("There always should be parrent path for log file")?;

    create_dir_all(&parent_path)?;
    if logger_file_path.exists() {
        let time_for_name = crate::utils::get_time();

        let new_file_name = "log_".to_string() + &time_for_name.to_string();
        std::fs::rename(logger_file_path, parent_path.join(new_file_name))?;
    }
    Ok(())
}

#[test]
fn log_test() {
    let paths = crate::config::ProgramFiles::init_in_base().unwrap();
    let _guard = configure_logger(&paths.logs_path).unwrap();
    tracing::info!("event bez spanu");
    let span = tracing::info_span!("login", user = "test_user");
    let _e = span.enter();
    tracing::info!("log w spanie");
    tracing::error!("blad w spanie");
}
