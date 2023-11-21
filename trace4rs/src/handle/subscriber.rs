use std::fmt;

use tracing::{span, Event, Level, Subscriber};
use tracing_span_tree::SpanTree;
use tracing_subscriber::filter::{Filtered, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use tracing_subscriber::{layer::Layered, registry::LookupSpan, reload, Layer};

use crate::handle::T4Layer;

type InnerLayered<Reg> = Layered<
    Filtered<SpanTree, Targets, Layered<reload::Layer<T4Layer<Reg>, Reg>, Reg>>,
    Layered<tracing_subscriber::reload::Layer<T4Layer<Reg>, Reg>, Reg>,
>;

/// The `tracing::Subscriber` that this crate implements.
pub struct T4Subscriber<Reg = Registry> {
    inner: InnerLayered<Reg>,
}

impl T4Subscriber {
    pub(crate) fn new_extra<Reg>(
        broker: Reg,
        layers: reload::Layer<T4Layer<Reg>, Reg>,
    ) -> T4Subscriber<Reg>
    where
        Reg: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + fmt::Debug,
    {
        let inner = broker.with(layers).with(Self::mk_extra());
        T4Subscriber { inner }
    }
    pub fn mk_extra<Reg>() -> Filtered<SpanTree, Targets, Reg>
    where
        Reg: Subscriber + for<'s> LookupSpan<'s> + fmt::Debug,
    {
d
        let layer = tracing_span_tree::span_tree();

        let filter = tracing_subscriber::filter::targets::Targets::new()
            .with_target("rasp_ffi", Level::TRACE);

        let filtered = layer.with_filter(filter);

        filtered
    }
}

// ########## DELEGATION BELOW ###########

impl<'a, Reg> LookupSpan<'a> for T4Subscriber<Reg>
where
    Reg: Subscriber + LookupSpan<'a>,
    Layered<tracing_subscriber::reload::Layer<T4Layer<Reg>, Reg>, Reg>: Subscriber,
{
    type Data = <Reg as LookupSpan<'a>>::Data;
    fn register_filter(&mut self) -> tracing_subscriber::filter::FilterId {
        self.inner.register_filter()
    }
    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}
impl<Reg> Subscriber for T4Subscriber<Reg>
where
    Reg: Subscriber + for<'a> LookupSpan<'a> + fmt::Debug,
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

    fn try_close(&self, id: span::Id) -> bool {
        Subscriber::try_close(&self.inner, id)
    }

    fn exit(&self, span: &span::Id) {
        Subscriber::exit(&self.inner, span);
    }
}
