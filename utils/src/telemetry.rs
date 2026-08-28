use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use tracing_subscriber::Registry;
use crate::deserializer;

#[derive(Debug, Clone)]
pub enum OTLPProtocol {
    Grpc,
    Http,
}

impl<'de> serde::Deserialize<'de> for OTLPProtocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ProtocolVisitor;

        impl<'de> serde::de::Visitor<'de> for ProtocolVisitor {
            type Value = OTLPProtocol;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string 'grpc' or 'http' (case-insensitive)")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v.to_ascii_lowercase().as_str() {
                    "grpc" => Ok(OTLPProtocol::Grpc),
                    "http" => Ok(OTLPProtocol::Http),
                    other => Err(E::unknown_variant(other, &["grpc", "http"])),
                }
            }
        }

        deserializer.deserialize_str(ProtocolVisitor)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
    #[serde(deserialize_with = "deserializer::string_to_bool")]
    pub otlp_enable: bool,
    pub otlp_endpoint: String,
    pub otlp_protocol: OTLPProtocol,
    pub otlp_service_name: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            otlp_enable: false,
            otlp_endpoint: "".to_string(),
            otlp_protocol: OTLPProtocol::Grpc,
            otlp_service_name: "app".to_string(),
        }
    }
}

pub fn init_telemetry(
    config: TelemetryConfig,
) -> Option<(SdkTracerProvider, OpenTelemetryLayer<Registry, Tracer>)> {
    if !config.otlp_enable {
        return None;
    }
    // Create an OTLP span exporter using the tonic (gRPC) implementation.
    // This exporter will send collected spans to an OTLP collector/endpoint.
    let exporter = match config.otlp_protocol {
        OTLPProtocol::Grpc => {
            SpanExporter::builder()
                .with_tonic()
                .with_endpoint(config.otlp_endpoint)
                .build()
                .expect("Failed to create OTLP gRPC exporter")
        }

        OTLPProtocol::Http => {
            SpanExporter::builder()
                .with_http()
                .with_endpoint(config.otlp_endpoint)
                .build()
                .expect("Failed to create OTLP HTTP exporter")
        }
    };

    // Build a tracer provider with a Resource that identifies this service.
    // The provider is configured with a batch exporter to buffer and export spans
    // asynchronously in the background.
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(config.otlp_service_name.clone())
                .build(),
        )
        .with_batch_exporter(exporter)
        .build();

    // Obtain a tracer from the provider. This tracer is used to create spans.
    let tracer = provider.tracer(config.otlp_service_name.clone());

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

    // registers a propagator in OpenTelemetry's global context
    global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    // Set the OpenTelemetry global tracer provider so other libraries can
    // retrieve tracers from the global API.
    global::set_tracer_provider(provider.clone());

    // Return the configured provider so the caller can keep ownership if needed.
    Some((provider, telemetry_layer))
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
        self.request
            .insert_header(key.to_owned(), value)
            .expect("failed to insert OTel header");
    }
}

pub struct PingoraHeaderExtractor<'a> {
    pub request: &'a pingora::http::RequestHeader,
}

impl opentelemetry::propagation::Extractor for PingoraHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.request
            .headers
            .get(key)
            .and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.request
            .headers
            .keys()
            .map(|name| name.as_str())
            .collect()
    }
}
