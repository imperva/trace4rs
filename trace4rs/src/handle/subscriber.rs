use std::{fmt, io};

use tracing::{span, Event, Subscriber};
use tracing_span_tree::SpanTree;
use tracing_subscriber::filter::{Filtered, Targets};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::Registry;
use tracing_subscriber::{layer::Layered, registry::LookupSpan, reload, Layer};

use crate::handle::T4Layer;

pub type T4Reload<Reg> = reload::Layer<T4Layer<Reg>, Reg>;
pub type LayeredT4Reload<Reg> = Layered<T4Reload<Reg>, Reg>;
pub type FilteredST<Reg, Wrt> = Filtered<SpanTree<Wrt>, Targets, LayeredT4Reload<Reg>>;

/// The `tracing::Subscriber` that this crate implements.
pub struct T4Subscriber<Reg = Registry, ExtLyr = FilteredST<Reg, fn() -> io::Stderr>> {
    inner: Layered<ExtLyr, LayeredT4Reload<Reg>>,
}

impl T4Subscriber {
    pub(crate) fn new_with<Reg, ExtLyr>(
        broker: Reg,
        t4_layer: T4Reload<Reg>,
        extra: ExtLyr,
    ) -> T4Subscriber<Reg, ExtLyr>
    where
        ExtLyr: Layer<LayeredT4Reload<Reg>>,
        Reg: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + fmt::Debug,
    {
        // let inner = t4_layer.with_subscriber(broker).with(extra);
        let inner = broker.with(t4_layer).with(extra);
        T4Subscriber { inner }
    }
}

// ########## DELEGATION BELOW ###########

impl<'a, Reg, ExtLyr> LookupSpan<'a> for T4Subscriber<Reg, ExtLyr>
where
    LayeredT4Reload<Reg>: Subscriber + LookupSpan<'a>,
    ExtLyr: Layer<LayeredT4Reload<Reg>> + Layer<LayeredT4Reload<Reg>>,
{
    type Data = <LayeredT4Reload<Reg> as LookupSpan<'a>>::Data;
    fn register_filter(&mut self) -> tracing_subscriber::filter::FilterId {
        self.inner.register_filter()
    }
    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}
impl<Reg, ExtLyr> Subscriber for T4Subscriber<Reg, ExtLyr>
where
    ExtLyr: Layer<LayeredT4Reload<Reg>> + 'static,
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
