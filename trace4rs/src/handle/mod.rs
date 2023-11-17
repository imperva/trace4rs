use std::convert::TryFrom;

use tracing::Subscriber;
use tracing_subscriber::{
    layer::{Layer, Layered},
    registry::LookupSpan,
    reload,
};
use tracing_tree::HierarchicalLayer;

use crate::{
    config::Config,
    error::{Error, Result},
};

mod layers;
mod loggers;
mod shared_registry;
mod trace_logger;

use layers::Trace4Layers;
use shared_registry::SharedRegistry;
pub use trace_logger::TraceLogger;

use self::loggers::Logger;

type DynHandle = Handle<Box<dyn Subscriber + Send + Sync>>;
pub type StandardHandle = Handle;

/// The reloadable handle for a `TraceLogger`, with this we can modify the
/// logging configuration at runtime.
#[derive(Clone)]
pub struct Handle<Reg = SharedRegistry> {
    reload_handle: reload::Handle<Trace4Layers<Reg>, Reg>,
    trace_logger: TraceLogger<Reg>,
    root_subscriber: Reg,
}

/// Initializes the default `trace4rs` handle as the `tracing` global default.
///
/// # Errors
/// We could fail to set the global default subscriber for `tracing`.
pub fn init_console_logger() -> Result<Handle> {
    let h = Handle::default();
    tracing::subscriber::set_global_default(h.subscriber())?;
    Ok(h)
}
pub type HierarchicalHandle = Handle<Layered<HierarchicalLayer, SharedRegistry>>;

impl<Reg> Handle<Reg>
where
    Reg: Default + Subscriber + Send + Sync + Clone + for<'b> LookupSpan<'b>,
    Logger<Reg>: Layer<Reg>,
{
    pub fn new() -> Handle<Reg> {
        let registry = Reg::default();
        let layers = Trace4Layers::default(registry.clone());

        Handle::<Reg>::from_layers(registry, layers)
    }

    // pub fn new_hierarchical(n: usize) -> HierarchicalHandle {
    //     let registry = Spanregistry::new_hierarchical(n);
    //     let layers = Layers::<Hierarchicalregistry>::default(registry.clone());

    //     Handle::<Hierarchicalregistry>::from_layers(registry, layers)
    // }

    /// Get the subscriber that backs this handle.
    #[must_use]
    pub fn subscriber(&self) -> TraceLogger<Reg> {
        self.trace_logger.clone()
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
        let broke_clone = self.root_subscriber.clone();
        let ls = Trace4Layers::from_config(broke_clone, config)?;
        Ok(self.reload_handle.reload(ls)?)
    }

    /// Using the given `SharedRegistry` we configure and initialize our `Self`.
    ///
    /// # Errors
    /// This could fail building the appenders in the config, for example
    /// opening a file for write.
    pub fn from_config(reg: Reg, config: &Config) -> Result<Handle<Reg>>
    where
        Reg: Subscriber + Send + Sync + Clone,
    {
        let layers: Trace4Layers<Reg> = Trace4Layers::from_config(reg.clone(), config)?;
        Ok(Self::from_layers(reg, layers))
    }

    /// Builds `Self` from `Layers` and the backing `Reg`.
    fn from_layers(registry: Reg, layers: Trace4Layers<Reg>) -> Handle<Reg>
    where
        Reg: Subscriber + Send + Sync,
    {
        let (reloadable, reload_handle) = reload::Layer::new(layers);
        let root_subscriber = registry.clone();
        let trace_logger = TraceLogger::new(root_subscriber.clone(), reloadable);

        Handle {
            reload_handle,
            trace_logger,
            root_subscriber,
        }
    }
}

impl Default for Handle {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<Config> for Handle {
    type Error = Error;

    fn try_from(c: Config) -> Result<Self> {
        Self::try_from(&c)
    }
}

impl TryFrom<&Config> for Handle {
    type Error = Error;

    fn try_from(c: &Config) -> Result<Handle> {
        let reg = SharedRegistry::default();
        Handle::from_config(reg, c)
    }
}
