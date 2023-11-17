use std::sync::Arc;

use derive_where::derive_where;
use tracing::Subscriber;
use tracing_subscriber::{
    layer::{Layer, Layered},
    registry::LookupSpan,
    reload,
};
use tracing_tree::HierarchicalLayer;

use crate::{config::Config, error::Result};

mod layers;
mod loggers;
mod shared_registry;
mod trace_logger;

use layers::Trace4Layers;
use shared_registry::SharedRegistry;
pub use trace_logger::TraceLogger;

use self::loggers::Logger;

pub type StandardHandle = Handle;

/// The reloadable handle for a `TraceLogger`, with this we can modify the
/// logging configuration at runtime.
#[derive_where(Clone)]
pub struct Handle<Reg = SharedRegistry> {
    reload_handle: Arc<reload::Handle<Trace4Layers<Reg>, Reg>>,
}

/// Initializes the default `trace4rs` handle as the `tracing` global default.
///
/// # Errors
/// We could fail to set the global default subscriber for `tracing`.
pub fn init_console_logger() -> Result<Handle> {
    let (h, t) = Handle::new();
    tracing::subscriber::set_global_default(t)?;
    Ok(h)
}
pub type HierarchicalHandle = Handle<Layered<HierarchicalLayer, SharedRegistry>>;

impl<Reg> Handle<Reg>
where
    Reg: Layer<Reg> + Subscriber + Send + Sync + Default + for<'s> LookupSpan<'s>,
    Logger<Reg>: Layer<Reg>,
{
    pub fn unit() -> Self {
        let (handle, layer) = Handle::from_layers(Trace4Layers::default());
        handle
    }

    pub fn new() -> (Handle<Reg>, TraceLogger<Reg>) {
        let layers = Trace4Layers::default();

        Handle::from_layers(layers)
    }

    /// Disable the subscriber.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn disable(&self) -> Result<()> {
        self.reload_handle
            .modify(Trace4Layers::disable)
            .map_err(Into::into)
    }

    /// Enable the subscriber.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn enable(&self) -> Result<()> {
        self.reload_handle
            .modify(Trace4Layers::enable)
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
            .with_current(Trace4Layers::correct_appender_paths)??;
        Ok(())
    }

    /// Update with the given config.
    ///
    /// # Errors
    /// - We were unable to update the subscriber.
    /// - Building the appenders in the config, for example
    /// opening a file for write.
    pub fn update(&mut self, config: &Config) -> Result<()> {
        let ls = Trace4Layers::from_config(config)?;
        Ok(self.reload_handle.reload(ls)?)
    }

    /// Using the given `SharedRegistry` we configure and initialize our `Self`.
    ///
    /// # Errors
    /// This could fail building the appenders in the config, for example
    /// opening a file for write.
    pub fn from_config(config: &Config) -> Result<(Handle<Reg>, TraceLogger<Reg>)>
    where
        Reg: Subscriber + Send + Sync + for<'s> LookupSpan<'s>,
    {
        let layers: Trace4Layers<Reg> = Trace4Layers::from_config(config)?;
        Ok(Self::from_layers(layers))
    }

    /// Builds `Self` from `Layers` and the backing `Reg`.
    fn from_layers(layers: Trace4Layers<Reg>) -> (Handle<Reg>, TraceLogger<Reg>)
    where
        Reg: Subscriber + Send + Sync,
    {
        let (reloadable, reload_handle) = reload::Layer::new(layers);
        let trace_logger = TraceLogger::new(Reg::default(), reloadable);

        (
            Handle {
                reload_handle: Arc::new(reload_handle),
            },
            trace_logger,
        )
    }
}
