use tracing_appender::rolling;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

pub fn init_logger(
    level: String,
    log_format: String,
    log_folder: String,
    log_file: String,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let level_filter = to_level_filter(level);

    // Structured JSON logger
    let builder = fmt::Subscriber::builder()
        .with_max_level(level_filter)
        .with_target(true)
        .with_file(true) // ✅ include file path
        .with_line_number(true) // ✅ include line number
        .with_ansi(true);

    // Dynamic writer
    let (writer, guard): (BoxMakeWriter, Option<tracing_appender::non_blocking::WorkerGuard>) =
        if log_folder == "console" {
            let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
            (BoxMakeWriter::new(non_blocking), Some(guard))
        } else {
            let file_appender = rolling::daily(log_folder, log_file);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            (BoxMakeWriter::new(non_blocking), Some(guard))
        };

    // Build with chosen writer
    let builder = builder.with_writer(writer);

    if log_format.to_lowercase() != "plain" {
        builder
            .json()
            .with_current_span(true)
            .with_current_span(true)
            .flatten_event(true)
            .init();
    } else {
        builder.init();
    }

    tracing::info!("Logger initialized");
    guard
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
}
