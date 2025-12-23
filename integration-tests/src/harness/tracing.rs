use std::sync::{Arc, Mutex, Once};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::{EnvFilter, Layer, fmt};

use tracing_subscriber::prelude::*;

#[derive(Debug, Clone)]
pub struct CapturedEvent {
    pub target: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct TestEventLayer {
    pub events: Arc<Mutex<Vec<CapturedEvent>>>,
}

static INIT_TRACING: Once = Once::new();

pub fn init_test_tracing(events: Arc<Mutex<Vec<CapturedEvent>>>) {
    INIT_TRACING.call_once(|| {
        let capture_layer = TestEventLayer { events };

        tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")))
            .with(capture_layer)
            .with(fmt::layer().with_test_writer().with_ansi(false))
            .init();

        tracing::info!("test tracing initialized");
    });
}

impl<S> Layer<S> for TestEventLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = Vec::new();
        let mut visitor = FieldVisitor {
            fields: &mut fields,
        };
        event.record(&mut visitor);

        let meta = event.metadata();

        self.events.lock().unwrap().push(CapturedEvent {
            target: meta.target().to_string(),
            fields,
        });
    }
}

struct FieldVisitor<'a> {
    fields: &'a mut Vec<(String, String)>,
}

impl<'a> Visit for FieldVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .push((field.name().to_string(), format!("{value:?}")));
    }
}
