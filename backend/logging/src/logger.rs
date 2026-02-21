//! Structured Logger
//!
//! Wraps `tracing` to provide JSON-formatted output, file rotation (NDJSON),
//! and environment-based level control.

use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the global structured logger.
/// Creates a console logger and a rolling file logger.
pub fn init_logger<P: AsRef<Path>>(log_dir: P, level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    // Rolling file appender: writes NDJSON to `logs/clawforge.log.YYYY-MM-DD`
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "clawforge.log");
    
    // JSON layer for file
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_ansi(false);

    // Standard console layer
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_ansi(true);

    // Filter, console, and file
    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .try_init();
}
