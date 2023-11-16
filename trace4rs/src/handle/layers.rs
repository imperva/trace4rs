use std::sync::Arc;

use tracing::{metadata::LevelFilter, Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{fmt::format::DefaultFields, layer::Context, registry::LookupSpan, Layer};

use super::loggers::{EventFormatter, Logger};
use crate::{
    appenders::{Appender, Appenders},
    config::{AppenderId, Config},
    error::Result,
};

pub type PolyLayer<S> = Box<dyn Layer<S> + Send + Sync + 'static>;

pub struct Layers<S> {
    enabled: bool,
    default: PolyLayer<S>,
    loggers: Vec<PolyLayer<S>>,
    appenders: Appenders,
}
impl<S: Clone> Clone for Layers<S>
where
    PolyLayer<S>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            default: self.default.clone(),
            loggers: self.loggers.clone(),
            appenders: self.appenders.clone(),
        }
    }
}
impl<S: Clone> Layers<S> {
    /// If the files which are the target of appenders have been moved we
    /// abandon the moved files and remount at the correct path.
    pub fn correct_appender_paths(&self) -> Result<()> {
        self.appenders.correct_paths()
    }

    pub fn appenders(&self) -> &Appenders {
        &self.appenders
    }
}

impl<S: Clone> Layers<S> {
    /// The default `Layers` backed by `broker` (`INFO` and above goes to
    /// stdout).
    pub fn default(broker: S) -> Layers<S>
    where
        S: Subscriber + Send + Sync + for<'b> LookupSpan<'b>,
        Logger<S, DefaultFields, EventFormatter>: Layer<S>,
    {
        let stdout_appender = AppenderId("stdout".to_string());
        let appenders =
            Appenders::new(literally::hmap! {stdout_appender.clone() => Appender::new_console()});
        let default = Logger::new_erased(
            broker,
            LevelFilter::INFO,
            None,
            &[stdout_appender],
            &appenders,
            EventFormatter::Normal,
        );
        Layers::<S>::new(default, vec![], appenders)
    }

    /// Create a new `Layers` from a default layer and a pre-generated vec of
    /// sub-layers.
    fn new<B: Clone>(
        default: PolyLayer<B>,
        loggers: Vec<PolyLayer<B>>,
        appenders: Appenders,
    ) -> Layers<B> {
        Layers {
            enabled: true,
            default,
            loggers,
            appenders,
        }
    }

    /// Generate a `Layers` from a config and back it with `broker`.
    ///
    /// # Errors
    /// An error may occur while building the appenders.
    pub fn from_config<B>(broker: B, config: &Config) -> Result<Layers<B>>
    where
        B: Clone + Subscriber + Send + Sync + for<'b> LookupSpan<'b>,
        Logger<B>: Layer<B>,
    {
        let appenders = (&config.appenders).try_into()?;
        let layers: Vec<PolyLayer<_>> = config
            .loggers
            .iter()
            .map(|(targ, lg)| {
                Logger::new_erased(
                    broker.clone(),
                    lg.level.into(),
                    Some(targ.clone()),
                    lg.appenders.iter(),
                    &appenders,
                    lg.format.clone().into(),
                )
            })
            .collect();

        let default = Logger::new_erased(
            broker,
            config.default.level.into(),
            None,
            config.default.appenders.iter(),
            &appenders,
            config.default.format.clone().into(),
        );

        Ok(Layers::<B>::new(default, layers, appenders))
    }
}

impl<S: Clone> Layers<S> {
    /// Disable this subscriber.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enable this subscriber.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

impl<S> Layer<S> for Layers<S>
where
    S: Subscriber + Clone,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if !self.enabled {
            return;
        }
        let mut any = false;
        let normalized_metadata = NormalizeEvent::normalized_metadata(event);
        let metadata = normalized_metadata
            .as_ref()
            .unwrap_or_else(|| event.metadata());

        for layer in &self.loggers {
            let enabled = layer.enabled(metadata, ctx.clone());
            any |= enabled;
            if enabled {
                layer.on_event(event, ctx.clone());
            }
        }
        // If no other layer logged this then the default one will
        if !any && self.default.enabled(metadata, ctx.clone()) {
            self.default.on_event(event, ctx);
        }
    }
}
