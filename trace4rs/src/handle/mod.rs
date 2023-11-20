use std::sync::Arc;

use derive_where::derive_where;
use tracing::Subscriber;
use tracing_subscriber::{layer::Layer, registry::LookupSpan, reload};

use crate::{config::Config, error::Result};

mod inner;
mod registry;
mod subscriber;

use inner::layer::T4Layer;
use registry::T4Registry;

pub use inner::logger::Logger;
pub use subscriber::T4Subscriber;

/// The reloadable handle for a `ExtraTraceLogger`, with this we can modify the
/// logging configuration at runtime.
#[derive_where(Clone)]
pub struct Handle<Reg = T4Registry> {
    reload_handle: Arc<reload::Handle<T4Layer<Reg>, Reg>>,
}

/// Initializes the default `trace4rs` handle as the `tracing` global default.
///
/// # Errors
/// We could fail to set the global default subscriber for `tracing`.
pub fn init_console_logger() -> Result<Handle> {
    let (h, t): (Handle, T4Subscriber) = Handle::new();
    tracing::subscriber::set_global_default(t)?;
    Ok(h)
}

impl<Reg> Handle<Reg>
where
    Reg: Layer<Reg> + Subscriber + for<'s> LookupSpan<'s> + Send + Sync + Default,
    Logger<Reg>: Layer<Reg>,
{
    #[must_use]
    pub fn unit() -> Self {
        let (handle, _layer) = Handle::from_layers(T4Layer::default());
        handle
    }

    #[must_use]
    pub fn new() -> (Handle<Reg>, T4Subscriber<Reg>) {
        let layers = T4Layer::default();

        Handle::from_layers(layers)
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

    /// Using the given `T4Registry` we configure and initialize our `Self`.
    ///
    /// # Errors
    /// This could fail building the appenders in the config, for example
    /// opening a file for write.
    pub fn from_config(config: &Config) -> Result<(Handle<Reg>, T4Subscriber<Reg>)>
    where
        Reg: Subscriber + Send + Sync + for<'s> LookupSpan<'s>,
    {
        let layers: T4Layer<Reg> = T4Layer::from_config(config)?;
        Ok(Self::from_layers(layers))
    }

    /// Builds `Self` from `Layers` and the backing `Reg`.
    fn from_layers(layers: T4Layer<Reg>) -> (Handle<Reg>, T4Subscriber<Reg>)
    where
        Reg: Subscriber + Send + Sync,
    {
        let (reloadable, reload_handle) = reload::Layer::new(layers);
        let trace_logger = T4Subscriber::new_extra(Reg::default(), reloadable);

        (
            Handle {
                reload_handle: Arc::new(reload_handle),
            },
            trace_logger,
        )
    }
}
