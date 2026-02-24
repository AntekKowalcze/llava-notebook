//! Module for application logger, temporarry holds terminal logging
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
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);
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

    Ok(_guard)
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
    let paths = ProgramFiles::init_in_base().unwrap();
    let _guard = configure_logger(&paths.logs_path).unwrap();
    tracing::info!("event bez spanu");
    let span = tracing::info_span!("login", user = "test_user");
    let _e = span.enter();
    tracing::info!("log w spanie");
    tracing::error!("blad w spanie");
}
