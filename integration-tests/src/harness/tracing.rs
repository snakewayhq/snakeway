use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, layer::Context};

#[derive(Debug, Clone)]
pub struct CapturedEvent {
    pub target: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct TestEventLayer {
    pub events: Arc<Mutex<Vec<CapturedEvent>>>,
}

pub fn init_test_tracing(events: Arc<Mutex<Vec<CapturedEvent>>>) {
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        let layer = TestEventLayer { events };

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
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
