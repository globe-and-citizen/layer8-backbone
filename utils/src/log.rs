use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;

pub fn init_logger(service_name: &str, level: String, _log_path: String) {
    let level_filter = to_level_filter(level);

    // let (nb, guard) = match log_path.as_str() {
    //     "console" => {
    //         non_blocking(std::io::stdout())
    //     }
    //     _ => {
    //         let file_appender = tracing_appender::rolling::daily(log_path, "forwardproxy.log");
    //         non_blocking(file_appender)
    //     }
    // };

    // Structured JSON logger
    fmt::Subscriber::builder()
        .json()
        .with_max_level(level_filter)
        .with_current_span(true)
        .with_target(true)
        .with_file(true)     // ✅ include file path
        .with_line_number(true) // ✅ include line number
        .flatten_event(true)
        .with_ansi(true)
        // .with_writer(nb)
        .init();

    tracing::info!(service = service_name, "Logger initialized");
}


fn to_level_filter(level: String) -> LevelFilter {
    match level.to_uppercase().as_str() {
        "INFO" => LevelFilter::INFO,
        "DEBUG" => LevelFilter::DEBUG,
        "WARNING" => LevelFilter::WARN,
        "ERROR" => LevelFilter::ERROR,
        "TRACE" => LevelFilter::TRACE,
        "OFF" => LevelFilter::OFF,
        _ => LevelFilter::INFO
    }
}


