use tracing::{metadata::LevelFilter, Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    fmt::{format::DefaultFields, writer::BoxMakeWriter, Layer as FmtLayer},
    layer::Context,
    registry::LookupSpan,
    Layer,
};

use super::{
    loggers::{EventFormatter, Logger},
    shared_registry::SharedRegistry,
};
use crate::{
    appenders::{Appender, Appenders},
    config::{AppenderId, Config},
    error::Result,
};

pub struct Trace4Layers<S = SharedRegistry> {
    enabled: bool,
    default: Logger<S>,
    loggers: Vec<Logger<S>>,
    appenders: Appenders,
}

impl<S: Clone> Clone for Trace4Layers<S>
where
    Logger<S>: Clone,
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
impl<S> Trace4Layers<S> {
    /// If the files which are the target of appenders have been moved we
    /// abandon the moved files and remount at the correct path.
    pub fn correct_appender_paths(&self) -> Result<()> {
        self.appenders.correct_paths()
    }

    pub fn appenders(&self) -> &Appenders {
        &self.appenders
    }
}

impl Trace4Layers {
    /// The default `Layers` backed by `broker` (`INFO` and above goes to
    /// stdout).
    pub fn default<Reg>(registry: Reg) -> Trace4Layers<Reg>
    where
        Reg: Subscriber + Send + Sync + for<'b> LookupSpan<'b>,
        Logger<Reg>: Layer<Reg>,
    {
        let stdout_appender = AppenderId("stdout".to_string());
        let appenders =
            Appenders::new(literally::hmap! {stdout_appender.clone() => Appender::new_console()});
        let default = Logger::new(
            registry,
            LevelFilter::INFO,
            None,
            (&[stdout_appender]).into_iter(),
            &appenders,
            EventFormatter::Normal,
        );
        Trace4Layers::new(default, vec![], appenders)
    }

    /// Create a new `Layers` from a default layer and a pre-generated vec of
    /// sub-layers.
    fn new<Reg>(
        default: Logger<Reg>,
        loggers: Vec<Logger<Reg>>,
        appenders: Appenders,
    ) -> Trace4Layers<Reg> {
        Trace4Layers {
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
    pub fn from_config<Reg>(registry: Reg, config: &Config) -> Result<Trace4Layers<Reg>>
    where
        Reg: Clone + Subscriber + Send + Sync + for<'b> LookupSpan<'b>,
        Logger<Reg>: Layer<Reg>,
    {
        let appenders = (&config.appenders).try_into()?;
        let layers: Vec<Logger<_>> = config
            .loggers
            .iter()
            .map(|(targ, lg)| {
                Logger::new(
                    registry.clone(),
                    lg.level.into(),
                    Some(targ.clone()),
                    lg.appenders.iter(),
                    &appenders,
                    lg.format.clone().into(),
                )
            })
            .collect();

        let default = Logger::new(
            registry,
            config.default.level.into(),
            None,
            config.default.appenders.iter(),
            &appenders,
            config.default.format.clone().into(),
        );

        Ok(Trace4Layers::new(default, layers, appenders))
    }
}

impl<S> Trace4Layers<S> {
    /// Disable this subscriber.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enable this subscriber.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

impl<S> Layer<S> for Trace4Layers<S>
where
    S: Subscriber + Clone,
    FmtLayer<S, DefaultFields, EventFormatter, BoxMakeWriter>: Layer<S>,
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
