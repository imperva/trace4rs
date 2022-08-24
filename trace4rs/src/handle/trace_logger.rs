use std::sync::Arc;

use tracing::{
    span,
    Event,
    Subscriber,
};
use tracing_subscriber::{
    layer::Layered,
    prelude::*,
    reload,
};

use super::span_broker::SpanBroker;
use crate::handle::Layers;

/// The `tracing::Subscriber` that this crate implements.
#[derive(Clone)]
pub struct TraceLogger {
    inner: Arc<Layered<reload::Layer<Layers<SpanBroker>, SpanBroker>, SpanBroker>>,
}

impl TraceLogger {
    pub(crate) fn new(broker: SpanBroker, layers: reload::Layer<Layers, SpanBroker>) -> Self {
        let inner = Arc::new(broker.with(layers));
        Self { inner }
    }
}

impl Subscriber for TraceLogger {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        Subscriber::enabled(self.inner.as_ref(), metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        Subscriber::new_span(self.inner.as_ref(), span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        Subscriber::record(self.inner.as_ref(), span, values);
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        Subscriber::record_follows_from(self.inner.as_ref(), span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        Subscriber::event(self.inner.as_ref(), event);
    }

    fn enter(&self, span: &span::Id) {
        Subscriber::enter(self.inner.as_ref(), span);
    }

    fn exit(&self, span: &span::Id) {
        Subscriber::exit(self.inner.as_ref(), span);
    }
}
