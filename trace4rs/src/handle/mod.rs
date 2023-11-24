use std::sync::Arc;

use derive_where::derive_where;
use tracing::{Level, Subscriber};
use tracing_span_tree::SpanTree;
use tracing_subscriber::{
    filter::{Filtered, Targets},
    fmt::MakeWriter,
    layer::{self, Layer, Layered, SubscriberExt as _},
    registry::LookupSpan,
    reload, Registry,
};

use crate::{config::Config, error::Result};

mod inner;

use inner::layer::T4Layer;
pub use inner::logger::Logger;

pub type T4<Reg> = reload::Layer<T4Layer<Reg>, Reg>;
pub type T4H<Reg> = reload::Handle<T4Layer<Reg>, Reg>;
pub type LayeredT4<Reg> = Layered<T4<Reg>, Reg>;
pub type ExtendedT4<Reg, ExtLyr> = Layered<ExtLyr, LayeredT4<Reg>>;

pub type FilteredST<Reg, Wrt> = Filtered<SpanTree<Wrt>, Targets, LayeredT4<Reg>>;

pub fn init_with_metrics<Reg, Wrt>(
    target: impl Into<String>,
    w: Wrt,
) -> (Handle<Reg>, ExtendedT4<Reg, FilteredST<Reg, Wrt>>)
where
    Wrt: for<'a> MakeWriter<'a> + 'static,
    Reg: Subscriber + for<'a> LookupSpan<'a> + Default + Send + Sync,
{
    let layer = tracing_span_tree::span_tree_with(w);
    let filter = Targets::new().with_target(target, Level::TRACE);
    let extra = layer.with_filter(filter);

    Handle::new_with(extra)
}

/// The reloadable handle for a `ExtraTraceLogger`, with this we can modify the
/// logging configuration at runtime.
#[derive_where(Clone)]
pub struct Handle<Reg = Registry> {
    reload_handle: Arc<T4H<Reg>>,
}

impl<Reg> Handle<Reg>
where
    Reg: Subscriber + for<'s> LookupSpan<'s> + Send + Sync + Default,
{
    /// Used for when you need a handle, but you don't need a logger. Should only ever really be useful to satisfy the compiler.
    #[must_use]
    pub fn unit() -> Handle<Reg> {
        let (handle, _layer) = Handle::from_layers_with(T4Layer::default(), layer::Identity::new());
        handle
    }

    /// Initialize trace4rs without an additional layer
    #[must_use]
    pub fn new() -> (Handle<Reg>, ExtendedT4<Reg, layer::Identity>) {
        Handle::new_with(layer::Identity::new())
    }

    /// Initialize trace4rs with an additional layer
    pub fn new_with<ExtLyr>(extra: ExtLyr) -> (Handle<Reg>, ExtendedT4<Reg, ExtLyr>)
    where
        ExtLyr: Layer<LayeredT4<Reg>>,
    {
        let layers = T4Layer::default();

        Handle::from_layers_with(layers, extra)
    }

    /// Initialize trace4rs from a `Config`
    ///
    /// # Errors
    /// This could fail building the appenders in the config, for example
    /// opening a file for write.
    pub fn from_config(config: &Config) -> Result<(Handle<Reg>, ExtendedT4<Reg, layer::Identity>)> {
        let layers: T4Layer<Reg> = T4Layer::from_config(config)?;
        Ok(Handle::from_layers_with(layers, layer::Identity::new()))
    }

    /// Initialize trace4rs from a `Config` with an additional layer
    pub fn from_config_with<ExtLyr>(
        config: &Config,
        extra: ExtLyr,
    ) -> Result<(Handle<Reg>, ExtendedT4<Reg, ExtLyr>)>
    where
        ExtLyr: Layer<LayeredT4<Reg>>,
    {
        let layers: T4Layer<Reg> = T4Layer::from_config(config)?;
        Ok(Handle::from_layers_with(layers, extra))
    }

    /// Builds `Self` from `Layers` and the backing `Reg`.
    fn from_layers_with<ExtLyr>(
        layers: T4Layer<Reg>,
        extra: ExtLyr,
    ) -> (Handle<Reg>, ExtendedT4<Reg, ExtLyr>)
    where
        ExtLyr: Layer<LayeredT4<Reg>>,
    {
        let (reloadable, reload_handle) = reload::Layer::new(layers);
        let trace_logger = Reg::default().with(reloadable).with(extra);

        (
            Handle {
                reload_handle: Arc::new(reload_handle),
            },
            trace_logger,
        )
    }

    /// Disable the subscriber.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn disable(&self) -> Result<()> {
        self.reload_handle
            .modify(T4Layer::disable)
            .map_err(Into::into)
    }

    /// Enable the subscriber.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn enable(&self) -> Result<()> {
        self.reload_handle
            .modify(T4Layer::enable)
            .map_err(Into::into)
    }

    /// Flush buffered output for all appenders.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn flush(&self) -> Result<()> {
        self.reload_handle
            .with_current(|ls| ls.appenders().flush())??;
        Ok(())
    }

    /// Correct the output path of log files if they have been moved.
    ///
    /// # Errors
    /// - We were unable to update the subscriber.
    /// - Re-mounting a file has failed.
    pub fn correct_appender_paths(&self) -> Result<()> {
        self.reload_handle
            .with_current(T4Layer::correct_appender_paths)??;
        Ok(())
    }

    /// Update with the given config.
    ///
    /// # Errors
    /// - We were unable to update the subscriber.
    /// - Building the appenders in the config, for example
    /// opening a file for write.
    pub fn update(&mut self, config: &Config) -> Result<()> {
        let ls = T4Layer::from_config(config)?;
        Ok(self.reload_handle.reload(ls)?)
    }
}
