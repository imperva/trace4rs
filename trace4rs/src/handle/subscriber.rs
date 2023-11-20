use std::marker::PhantomData;

use tracing::{span, Event, Level, Subscriber};
use tracing_subscriber::filter::{Filtered, Targets};
use tracing_subscriber::layer::{Filter, SubscriberExt};
use tracing_subscriber::{layer::Layered, registry::LookupSpan, reload, Layer};
use tracing_tree::HierarchicalLayer;

use crate::handle::T4Layer;

use super::registry::T4Registry;

trait T4Sub<'a, S>: Layer<S> + Subscriber + Send + Sync + LookupSpan<'a>
where
    S: Subscriber,
{
}
impl<'a, L, I, S> T4Sub<'a, S> for Layered<L, I, S>
where
    L: Layer<S> + Send + Sync,
    I: Layer<S> + Send + Sync,
    S: Subscriber,
    Layered<L, I, S>: Subscriber + LookupSpan<'a>,
{
}
impl<'a, L, F, Reg> T4Sub<'a, Reg> for Filtered<L, F, Reg>
where
    L: Layer<Reg> + Send + Sync,
    F: Filter<Reg> + Send + Sync,
    for<'s> Reg: Subscriber + LookupSpan<'s>,
    for<'s> Filtered<L, F, Reg>: Subscriber + LookupSpan<'s>,
{
}

pub type DynSubscriber<S>
where
    S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync,
= Box<dyn for<'a> T4Sub<'a, S, Data = <S as LookupSpan<'a>>::Data>>;

/// The `tracing::Subscriber` that this crate implements.
pub struct T4Subscriber<
    Reg = T4Registry,
    L = Layered<
        Layered<Filtered<HierarchicalLayer, Targets, Reg>, Reg>,
        tracing_subscriber::reload::Layer<T4Layer<Reg>, Reg>,
        Reg,
    >,
> {
    inner: L,
    reg: PhantomData<Reg>,
}

impl T4Subscriber {
    pub(crate) fn new_extra<Reg>(
        broker: Reg,
        layers: reload::Layer<T4Layer<Reg>, Reg>,
    ) -> T4Subscriber<Reg>
    where
        Reg: Layer<Reg> + Subscriber + Send + Sync + for<'a> LookupSpan<'a>,
    {
        let extra: Layered<Filtered<HierarchicalLayer, Targets, Reg>, Reg> =
            broker.with(Self::mk_extra());
        let inner = layers.and_then(extra);
        T4Subscriber {
            inner,
            reg: PhantomData,
        }
    }
    pub fn mk_extra<Reg>() -> Filtered<HierarchicalLayer, Targets, Reg>
    where
        Reg: Subscriber + for<'s> LookupSpan<'s>,
    {
        let layer = tracing_tree::HierarchicalLayer::default()
            .with_indent_lines(true)
            .with_indent_amount(2)
            .with_thread_names(true)
            .with_thread_ids(true)
            .with_verbose_exit(true)
            .with_verbose_entry(true)
            .with_targets(true)
            .with_higher_precision(true);

        let filter = tracing_subscriber::filter::targets::Targets::new()
            .with_target("rasp_ffi", Level::TRACE);

        let filtered = layer.with_filter(filter);

        filtered
    }
}

// ########## DELEGATION BELOW ###########

impl<'a, Reg, Lay> LookupSpan<'a> for T4Subscriber<Reg, Lay>
where
    Reg: Subscriber,
    Lay: T4Sub<'a, Reg>,
{
    type Data = Lay::Data;
    fn register_filter(&mut self) -> tracing_subscriber::filter::FilterId {
        self.inner.register_filter()
    }
    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}
impl<Reg, Lay> Subscriber for T4Subscriber<Reg, Lay>
where
    Reg: Subscriber,
    Lay: Subscriber + Layer<Reg>,
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
