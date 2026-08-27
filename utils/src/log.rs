use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, prelude::*};

use opentelemetry_sdk::trace::SdkTracerProvider;

use crate::telemetry::{init_telemetry, TelemetryConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    pub log_level: String,
    /// default to "json" if not "plain"
    pub log_format: String,
    /// "console" or folder path
    pub log_path: String,
    /// required if log_path is not "console"
    pub log_filename: String,
}

pub struct LogAndTraceGuard {
    _file_guard: Option<WorkerGuard>,
    _trace_provider: SdkTracerProvider,
}

pub fn init_logger(
    log_config: LogConfig,
    telemetry_config: TelemetryConfig,
) -> LogAndTraceGuard {
    // 1) Map the configured string level to the tracing filter.
    //    This is the gatekeeper for events emitted by the app.
    let level_filter = to_level_filter(log_config.log_level);

    // 2) Select the destination writer.
    //    - "console" => stdout
    //    - otherwise => daily rotating file under the configured directory/name
    //    The non-blocking wrapper prevents logging from blocking the app thread.
    let (writer, file_guard) = if log_config.log_path == "console" {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
        (fmt::writer::BoxMakeWriter::new(non_blocking), Some(guard))
    } else {
        let file_appender = tracing_appender::rolling::daily(log_config.log_path, log_config.log_filename);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        (fmt::writer::BoxMakeWriter::new(non_blocking), Some(guard))
    };

    // 3) Build the OpenTelemetry provider and attach its tracing layer.
    //    This allows tracing events to be exported through the configured SDK.
    let (telemetry_provider, telemetry_layer) = init_telemetry(telemetry_config);

    // 4) Determine output format.
    //    Anything other than "plain" is treated as structured JSON output.
    let is_json = log_config.log_format.to_lowercase() != "plain";

    // 5) Create the base registry.
    //    The registry combines the exporter layer with the log-level filter.
    let registry = tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(level_filter);

    // 6) Create the final dispatch object.
    //    The selected formatter controls how events are rendered.
    //    `writer` is consumed in the chosen branch and is therefore not reused.
    let dispatch = if is_json {
        let json_layer = fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .json()
            .with_current_span(true)
            .flatten_event(true);

        tracing::Dispatch::new(registry.with(Some(json_layer)))
    } else {
        let plain_layer = fmt::layer()
            .with_writer(writer)
            .with_ansi(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .compact();

        tracing::Dispatch::new(registry.with(Some(plain_layer)))
    };

    // 7) Install this subscriber as the process-wide default.
    //    After this call, all `tracing::info!`, `warn!`, and `error!`
    //    calls in the application use this configuration by default.
    tracing::dispatcher::set_global_default(dispatch)
        .expect("Failed to set global tracing default dispatcher");

    tracing::info!("Logger and OpenTelemetry Tracing initialized successfully.");

    LogAndTraceGuard {
        _file_guard: file_guard,
        _trace_provider: telemetry_provider,
    }
}

fn to_level_filter(level: String) -> LevelFilter {
    match level.to_uppercase().as_str() {
        "INFO" => LevelFilter::INFO,
        "DEBUG" => LevelFilter::DEBUG,
        "WARNING" => LevelFilter::WARN,
        "ERROR" => LevelFilter::ERROR,
        "TRACE" => LevelFilter::TRACE,
        "OFF" => LevelFilter::OFF,
        _ => LevelFilter::INFO,
    }
    // EnvFilter::try_from_default_env()
    //     .unwrap_or_else(|_| EnvFilter::new(&level))
}
