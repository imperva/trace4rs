use tracing::{metadata::LevelFilter, Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer, Registry};

use super::formatter::EventFormatter;
use super::logger::Logger;
use crate::{
    appenders::{Appender, Appenders},
    config::{AppenderId, Config},
    error::Result,
};

pub struct T4Layer<S = Registry> {
    enabled: bool,
    default: Logger<S>,
    loggers: Vec<Logger<S>>,
    appenders: Appenders,
}

impl<S> T4Layer<S> {
    /// If the files which are the target of appenders have been moved we
    /// abandon the moved files and remount at the correct path.
    pub fn correct_appender_paths(&self) -> Result<()> {
        self.appenders.correct_paths()
    }

    pub fn appenders(&self) -> &Appenders {
        &self.appenders
    }
    /// Disable this subscriber.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enable this subscriber.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

impl<Reg> T4Layer<Reg>
where
    Reg: Subscriber + Send + Sync + for<'s> LookupSpan<'s>,
{
    /// The default `Layers` backed by `broker` (`INFO` and above goes to
    /// stdout).
    pub fn default() -> Self {
        let stdout_appender = AppenderId("stdout".to_string());
        let appenders =
            Appenders::new(literally::hmap! {stdout_appender.clone() => Appender::new_console()});
        let default = Logger::new(
            LevelFilter::INFO,
            None,
            [stdout_appender].iter(),
            &appenders,
            EventFormatter::Normal,
        );

        Self::new(default, vec![], appenders)
    }

    /// Create a new `Layers` from a default layer and a pre-generated vec of
    /// sub-layers.
    fn new(default: Logger<Reg>, loggers: Vec<Logger<Reg>>, appenders: Appenders) -> Self {
        Self {
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
    pub fn from_config(config: &Config) -> Result<Self> {
        let appenders = (&config.appenders).try_into()?;
        let layers: Vec<Logger<_>> = config
            .loggers
            .iter()
            .map(|(targ, lg)| {
                Logger::new(
                    lg.level.into(),
                    Some(targ.clone()),
                    lg.appenders.iter(),
                    &appenders,
                    lg.format.clone().into(),
                )
            })
            .collect();

        let default = Logger::new(
            config.default.level.into(),
            None,
            config.default.appenders.iter(),
            &appenders,
            config.default.format.clone().into(),
        );

        Ok(T4Layer::new(default, layers, appenders))
    }
}

impl<S> Layer<S> for T4Layer<S>
where
    S: Subscriber + for<'s> LookupSpan<'s>,
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
