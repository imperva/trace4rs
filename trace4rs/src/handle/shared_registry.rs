#![allow(clippy::single_char_lifetime_names)]

use std::sync::Arc;

use derive_where::derive_where;
use tracing::{span, Event, Subscriber};
use tracing_subscriber::{
    registry::{self, LookupSpan},
    Registry,
};

type DynRegistry = SharedRegistry<Box<dyn Subscriber>>;

/// SharedRegistry exists because we need to be able to override the layer functionality.
/// Also we would otherwise need to wrap a registry in an arc to share it as much as we do.
///
#[derive(Debug)]
#[derive_where(Clone)]
#[derive_where(Default; Reg: Default)]
pub struct SharedRegistry<Reg = Registry> {
    inner: Arc<Reg>,
}

impl SharedRegistry<Registry> {
    pub fn new() -> Self {
        SharedRegistry {
            inner: Arc::new(tracing_subscriber::registry()),
        }
    }

    // pub fn new_hierarchical(n: usize) -> HierarchicalBroker {
    //     use tracing_subscriber::layer::SubscriberExt;

    //     let hier = HierarchicalLayer::new(n).with_writer(Self::open_metrics());
    //     let reg = tracing_subscriber::registry().with(hier);
    //     SpanBroker {
    //         inner: Arc::new(reg),
    //     }
    // }
}
// pub type HierarchicalBroker =
//     SharedRegistry<tracing_subscriber::layer::Layered<HierarchicalLayer<Mutex<File>>, Registry>>;
// impl Default for HierarchicalBroker {
//     fn default() -> Self {
//         SharedRegistry::new_hierarchical(2)
//     }
// }

impl tracing_subscriber::Layer<SharedRegistry> for SharedRegistry {}

// ########## DELEGATION BELOW ###########

impl<'a, R> LookupSpan<'a> for SharedRegistry<R>
where
    R: LookupSpan<'a, Data = registry::Data<'a>>,
{
    type Data = registry::Data<'a>;

    fn span_data(&'a self, id: &tracing::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}

impl<R> Subscriber for SharedRegistry<R>
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
