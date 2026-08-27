use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use tracing_subscriber::Registry;

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
    pub otlp_exporter_endpoint: String,
    pub otlp_exporter_protocol: String,
}

pub fn init_telemetry(
    config: TelemetryConfig,
) -> (SdkTracerProvider, OpenTelemetryLayer<Registry, Tracer>) {
    // Create an OTLP span exporter using the tonic (gRPC) implementation.
    // This exporter will send collected spans to an OTLP collector/endpoint.
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(config.otlp_exporter_endpoint.to_string())
        .build()
        .expect("failed to create OTLP exporter");

    // Build a tracer provider with a Resource that identifies this service.
    // The provider is configured with a batch exporter to buffer and export spans
    // asynchronously in the background.
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name("forward-proxy")
                .build(),
        )
        .with_batch_exporter(exporter)
        .build();

    // Obtain a tracer from the provider. This tracer is used to create spans.
    let tracer = provider.tracer("forward-proxy");

    // Create a tracing layer that forwards spans from `tracing` to OpenTelemetry.
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Initialize a tracing subscriber registry with:
    //  - a pretty-printing / formatting layer for local log/span output
    //  - the OpenTelemetry layer so spans are exported
    // Calling `init` sets this subscriber as the global default for `tracing`.
    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::fmt::layer())
    //     .with(telemetry)
    //     .init();

    // Set the OpenTelemetry global tracer provider so other libraries can
    // retrieve tracers from the global API.
    global::set_tracer_provider(provider.clone());

    // Return the configured provider so the caller can keep ownership if needed.
    (provider, telemetry_layer)
}

use opentelemetry::propagation::Injector;
use opentelemetry_sdk::trace::Tracer;
use pingora::prelude::RequestHeader;
use serde::Deserialize;
use tracing_opentelemetry::OpenTelemetryLayer;

pub struct PingoraHeaderInjector<'a> {
    pub request: &'a mut RequestHeader,
}

impl<'a> Injector for PingoraHeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        let _ = self.request.insert_header(key.to_owned(), value);
    }
}
