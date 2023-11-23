use std::{fmt, io, sync::Arc};

use derive_where::derive_where;
use tracing::{Level, Subscriber};
use tracing_span_tree::SpanTree;
use tracing_subscriber::{
    filter::{Filtered, Targets},
    fmt::MakeWriter,
    layer::Layer,
    registry::LookupSpan,
    reload, Registry,
};

use crate::{config::Config, error::Result};

mod inner;
mod subscriber;

use inner::layer::T4Layer;

pub use inner::logger::Logger;
pub use subscriber::T4Subscriber;

use self::subscriber::{FilteredST, LayeredT4Reload};

/// The reloadable handle for a `ExtraTraceLogger`, with this we can modify the
/// logging configuration at runtime.
#[derive_where(Clone)]
pub struct Handle<Reg = Registry> {
    reload_handle: Arc<reload::Handle<T4Layer<Reg>, Reg>>,
}

impl<Reg> Handle<Reg>
where
    Reg: Subscriber + for<'s> LookupSpan<'s> + Send + Sync + Default + fmt::Debug,
    Logger<Reg>: Layer<Reg>,
{
    /// Used for when you need a handle, but you don't need a logger.
    #[must_use]
    pub fn unit() -> Self {
        let (handle, _layer) = Handle::from_layers_mw(T4Layer::default(), io::empty);
        handle
    }

    #[must_use]
    pub fn new_with_mw<W>(w: W) -> (Handle<Reg>, T4Subscriber<Reg, FilteredST<Reg, W>>)
    where
        W: for<'a> MakeWriter<'a> + 'static,
    {
        let layers = T4Layer::default();

        Handle::from_layers_mw(layers, w)
    }

    #[must_use]
    pub fn new() -> (Handle<Reg>, T4Subscriber<Reg>) {
        let layers = T4Layer::default();

        Handle::from_layers_mw(layers, io::stderr)
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

    /// Using the given `Registry` we configure and initialize our `Self`.
    ///
    /// # Errors
    /// This could fail building the appenders in the config, for example
    /// opening a file for write.
    pub fn from_config(config: &Config) -> Result<(Handle<Reg>, T4Subscriber<Reg>)>
    where
        Reg: Subscriber + Send + Sync + for<'s> LookupSpan<'s> + fmt::Debug,
    {
        let layers: T4Layer<Reg> = T4Layer::from_config(config)?;
        Ok(Self::from_layers_mw(layers, io::stderr))
    }

    /// Builds `Self` from `Layers` and the backing `Reg`.
    fn from_layers_mw<Wrt>(
        layers: T4Layer<Reg>,
        w: Wrt,
    ) -> (Handle<Reg>, T4Subscriber<Reg, FilteredST<Reg, Wrt>>)
    where
        Reg: Subscriber + Send + Sync + fmt::Debug,
        Wrt: for<'a> MakeWriter<'a> + 'static,
    {
        let extra = Self::mk_extra_mw(w);
        Handle::from_layers_with_extra(layers, extra)
    }
    /// Builds `Self` from `Layers` and the backing `Reg`.
    fn from_layers_with_extra<ExtLyr>(
        layers: T4Layer<Reg>,
        extra: ExtLyr,
    ) -> (Handle<Reg>, T4Subscriber<Reg, ExtLyr>)
    where
        Reg: Subscriber + Send + Sync + fmt::Debug,
        LayeredT4Reload<Reg>: Subscriber,
        ExtLyr: Layer<LayeredT4Reload<Reg>>,
    {
        let (reloadable, reload_handle) = reload::Layer::new(layers);
        let trace_logger = T4Subscriber::new_with(Reg::default(), reloadable, extra);

        (
            Handle {
                reload_handle: Arc::new(reload_handle),
            },
            trace_logger,
        )
    }
    fn mk_extra_mw<Wrt, R>(w: Wrt) -> Filtered<SpanTree<Wrt>, Targets, R>
    where
        R: Subscriber + for<'a> LookupSpan<'a> + fmt::Debug,
        Wrt: for<'a> MakeWriter<'a> + 'static,
    {
        let layer = tracing_span_tree::span_tree_with(w);

        let filter = tracing_subscriber::filter::targets::Targets::new()
            .with_target("rasp_ffi", Level::TRACE);

        layer.with_filter(filter)
    }
}
