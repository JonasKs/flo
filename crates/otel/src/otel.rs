use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use std::collections::HashMap;
use tracing_core::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) headers: HashMap<String, String>,
    pub(crate) attributes: Vec<KeyValue>,
    pub(crate) project_name: String,
}

// Create a Resource that captures information about the entity for which telemetry is recorded.
fn resource(config: Config) -> Resource {
    Resource::builder()
        .with_service_name(config.project_name)
        .with_attributes(config.attributes)
        .build()
}

fn init_tracer_provider(config: Config) -> SdkTracerProvider {
    // Build the OTLP span exporter using the tonic transport.
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        // TODO: Use tonic
        .with_http()
        .with_endpoint(
            "<Grafana Cloud endpoint here>",
        )
        .with_headers(config.headers.clone())
        .build()
        .unwrap();

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource(config))
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    tracer_provider
}

// Construct MeterProvider for MetricsLayer
fn init_meter_provider(config: Config) -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::Delta)
        .with_endpoint(
            "<Grafana Cloud endpoint here>",
        )
        .with_headers(config.headers.clone())
        .build()
        .unwrap();

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource(config))
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    meter_provider
}

/// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
/// It's important to keep the guard in scope, even though it's not used for anything.
/// Usage: `let _guard = init_tracing_subscriber(&config);`
pub(crate) fn init_tracing_subscriber(config: Config) -> OtelGuard {
    let tracer_provider = init_tracer_provider(config.clone());
    let meter_provider = init_meter_provider(config);

    let tracer = tracer_provider.tracer("net-rust");

    tracing_subscriber::registry()
        // The global level filter prevents the exporter network stack
        // from reentering the globally installed OpenTelemetryLayer with
        // its own spans while exporting, as the libraries should not use
        // tracing levels below DEBUG. If the OpenTelemetry layer needs to
        // trace spans and events with higher verbosity levels, consider using
        // per-layer filtering to target the telemetry layer specifically,
        // e.g. by target matching.
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    OtelGuard {
        tracer_provider,
        meter_provider,
    }
}

pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    /// Call `shutdown` on the `TracerProvider` and `MeterProvider` when the guard is dropped.
    /// Ensures that all spans and metrics are flushed before the process exits.
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("{err:?}");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }
    }
}
