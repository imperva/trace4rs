#![allow(clippy::single_char_lifetime_names)]
use std::sync::Arc;

use tracing::{
    span,
    Event,
    Subscriber,
};
use tracing_subscriber::{
    registry::{
        self,
        LookupSpan,
    },
    Registry,
};

#[derive(Debug, Clone)]
pub struct SpanBroker {
    inner: Arc<Registry>,
}

impl SpanBroker {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for SpanBroker {
    fn default() -> Self {
        SpanBroker {
            inner: Arc::new(tracing_subscriber::registry()),
        }
    }
}
impl<'a> LookupSpan<'a> for SpanBroker {
    type Data = registry::Data<'a>;

    fn span_data(&'a self, id: &tracing::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}

impl Subscriber for SpanBroker {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        self.inner.new_span(span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.inner.record(span, values);
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        self.inner.event(event);
    }

    fn enter(&self, span: &span::Id) {
        self.inner.enter(span);
    }

    fn exit(&self, span: &span::Id) {
        self.inner.exit(span);
    }
}

impl tracing_subscriber::Layer<SpanBroker> for SpanBroker {}
