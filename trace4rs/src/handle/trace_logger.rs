use std::sync::Arc;

use tracing::{span, Event, Subscriber};
use tracing_subscriber::{layer::Layered, prelude::*, reload, Registry};

use crate::handle::Layers;

/// The `tracing::Subscriber` that this crate implements.
#[derive(Clone)]
pub struct TraceLogger<Reg = Registry> {
    inner: Arc<Layered<reload::Layer<Layers<Reg>, Reg>, Reg>>,
}

impl TraceLogger {
    pub(crate) fn new<Reg>(broker: Reg, layers: reload::Layer<Layers<Reg>, Reg>) -> TraceLogger<Reg>
    where
        Reg: Subscriber + Clone,
        // Arc<reload::Layer<Layers<Reg>, Reg>>: tracing_subscriber::layer::Layer<Reg>,
    {
        let inner = Arc::new(broker.with(layers));
        TraceLogger { inner }
    }
}

// ########## DELEGATION BELOW ###########

impl<Reg> Subscriber for TraceLogger<Reg>
where
    Reg: Subscriber + Clone,
    // reload::Layer<Layers<Reg>, Reg>: tracing_subscriber::Layer<Reg>,
{
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        Subscriber::enabled(&self.inner, metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        Subscriber::new_span(&self.inner, span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        Subscriber::record(&self.inner, span, values);
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        Subscriber::record_follows_from(&self.inner, span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        Subscriber::event(&self.inner, event);
    }

    fn enter(&self, span: &span::Id) {
        Subscriber::enter(&self.inner, span);
    }

    fn exit(&self, span: &span::Id) {
        Subscriber::exit(&self.inner, span);
    }
}
