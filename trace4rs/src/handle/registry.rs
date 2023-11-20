#![allow(clippy::single_char_lifetime_names)]

use derive_where::derive_where;
use tracing::{span, Event, Subscriber};
use tracing_subscriber::{
    registry::{self, LookupSpan},
    Registry,
};

// type DynRegistry = T4Registry<Box<dyn Subscriber>>;

/// T4Registry exists because we need to be able to override the layer functionality.
/// Also we would otherwise need to wrap a registry in an arc to share it as much as we do.
///
#[derive(Debug)]
#[derive_where(Default; Reg: Default)]
pub struct T4Registry<Reg = Registry> {
    inner: Reg,
}

impl T4Registry<Registry> {
    pub fn new() -> Self {
        T4Registry {
            inner: tracing_subscriber::registry(),
        }
    }
}

// eas: verify necessary, if not use a naked registry
impl tracing_subscriber::Layer<T4Registry> for T4Registry {}

// ########## DELEGATION BELOW ###########

impl<'a, R> LookupSpan<'a> for T4Registry<R>
where
    R: LookupSpan<'a, Data = registry::Data<'a>>,
{
    type Data = registry::Data<'a>;

    fn span_data(&'a self, id: &tracing::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
    fn register_filter(&mut self) -> tracing_subscriber::filter::FilterId {
        self.inner.register_filter()
    }
}

impl<R> Subscriber for T4Registry<R>
where
    R: Subscriber,
{
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