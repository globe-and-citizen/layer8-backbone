use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, prelude::*};

use opentelemetry_sdk::trace::SdkTracerProvider;

use crate::telemetry::{init_telemetry, TelemetryConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub log_level: String,
    /// default to "json" if not "plain"
    pub log_format: String,
    /// "console" or folder path
    pub log_path: String,
    /// required if log_path is not "console"
    pub log_filename: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_level: "INFO".into(),
            log_format: "plain".into(),
            log_path: "console".into(),
            log_filename: "app.log".into(),
        }
    }
}

pub struct LogAndTraceGuard {
    _file_guard: Option<WorkerGuard>,
    _trace_provider: Option<SdkTracerProvider>,
}

pub fn init_logger(
    log_config: LogConfig,
    telemetry_config: TelemetryConfig,
) -> LogAndTraceGuard {
    // 1) Map the configured string level to the tracing filter.
    let level_filter = to_level_filter(log_config.log_level);

    // 2) Select the destination writer.
    let (writer, file_guard) = if log_config.log_path == "console" {
        let (non_blocking, guard) =
            tracing_appender::non_blocking(std::io::stdout());

        (
            fmt::writer::BoxMakeWriter::new(non_blocking),
            Some(guard),
        )
    } else {
        let file_appender = tracing_appender::rolling::daily(
            log_config.log_path,
            log_config.log_filename,
        );

        let (non_blocking, guard) =
            tracing_appender::non_blocking(file_appender);

        (
            fmt::writer::BoxMakeWriter::new(non_blocking),
            Some(guard),
        )
    };

    // 3) Initialize OpenTelemetry.
    //
    //    If telemetry is disabled:
    //        telemetry = None
    //
    //    If telemetry is enabled:
    //        telemetry = Some((provider, layer))
    let telemetry = init_telemetry(telemetry_config);

    // Keep the provider alive for as long as the logger is alive.
    let telemetry_provider = telemetry
        .as_ref()
        .map(|(provider, _)| provider.clone());

    // Extract the optional OpenTelemetry layer.
    let telemetry_layer = telemetry.map(|(_, layer)| layer);

    // 4) Determine output format.
    let is_json = log_config.log_format.to_lowercase() != "plain";

    // 5) Create the base registry.
    //
    // `telemetry_layer` is Option<OpenTelemetryLayer<...>>.
    //
    // When Some:
    //     Registry → OpenTelemetry layer → level filter
    //
    // When None:
    //     Registry → level filter
    let registry = tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(level_filter);

    // 6) Add the selected formatting layer.
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

        tracing::Dispatch::new(
            registry.with(Some(json_layer))
        )
    } else {
        let plain_layer = fmt::layer()
            .with_writer(writer)
            .with_ansi(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .compact();

        tracing::Dispatch::new(
            registry.with(Some(plain_layer))
        )
    };

    // 7) Install the single global tracing subscriber.
    tracing::dispatcher::set_global_default(dispatch)
        .expect("Failed to set global tracing default dispatcher");

    tracing::info!(
        "Logger initialized successfully. OpenTelemetry tracing: {}",
        if telemetry_provider.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    );

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
