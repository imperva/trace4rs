use std::{
    borrow::Borrow,
    convert::TryFrom,
};

use tracing_subscriber::reload;

use crate::{
    config::Config,
    error::{
        Error,
        Result,
    },
};

mod layers;
mod loggers;
mod span_broker;
mod trace_logger;

use layers::Layers;
pub use layers::PolyLayer;
use span_broker::SpanBroker;
pub use trace_logger::TraceLogger;

/// The reloadable handle for a `TraceLogger`, with this we can modify the
/// logging configuration at runtime.
#[derive(Clone)]
pub struct Handle {
    reload_handle: reload::Handle<Layers<SpanBroker>, SpanBroker>,
    trace_logger:  TraceLogger,
    broker:        SpanBroker,
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

impl Handle {
    /// Get the subscriber that backs this handle.
    #[must_use]
    pub fn subscriber(&self) -> TraceLogger {
        self.trace_logger.clone()
    }

    /// Disable the subscriber.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn disable(&self) -> Result<()> {
        self.reload_handle
            .modify(Layers::disable)
            .map_err(Into::into)
    }

    /// Enable the subscriber.
    ///
    /// # Errors
    /// - An io error occurred in flushing output.
    /// - We were unable to update the subscriber.
    pub fn enable(&self) -> Result<()> {
        self.reload_handle
            .modify(Layers::enable)
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
            .with_current(Layers::correct_appender_paths)??;
        Ok(())
    }

    /// Update with the given config.
    ///
    /// # Errors
    /// - We were unable to update the subscriber.
    /// - Building the appenders in the config, for example
    /// opening a file for write.
    pub fn update(&mut self, config: &Config) -> Result<()> {
        let ls = Layers::from_config(self.broker.clone(), config)?;
        Ok(self.reload_handle.reload(ls)?)
    }

    /// Using the given `SpanBroker` we configure and initialize our `Self`.
    ///
    /// # Errors
    /// This could fail building the appenders in the config, for example
    /// opening a file for write.
    pub fn from_config(broker: SpanBroker, config: &Config) -> Result<Handle> {
        let layers = Layers::from_config(broker.clone(), config)?;
        Ok(Self::from_layers(broker, layers))
    }

    /// Builds `Self` from `Layers` and the backing `SpanBroker`.
    fn from_layers(broker: SpanBroker, layers: Layers) -> Self {
        let (reloadable, reload_handle) = reload::Layer::new(layers);
        let trace_logger = TraceLogger::new(broker.clone(), reloadable);

        Self {
            reload_handle,
            trace_logger,
            broker,
        }
    }
}

impl Default for Handle {
    fn default() -> Self {
        let broker = SpanBroker::new();
        let layers = Layers::default(broker.clone());

        Self::from_layers(broker, layers)
    }
}

impl TryFrom<Config> for Handle {
    type Error = Error;

    fn try_from(c: Config) -> Result<Handle> {
        Self::try_from(&c)
    }
}

impl TryFrom<&Config> for Handle {
    type Error = Error;

    fn try_from(c: &Config) -> Result<Handle> {
        let broker = SpanBroker::new();
        Self::from_config(broker, c)
    }
}
