use tracing::{metadata::LevelFilter, Event, Level, Subscriber};
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

type DynLayer<S> = Box<dyn Layer<S> + Send + Sync>;

pub struct T4Layer<S = SharedRegistry> {
    enabled: bool,
    default: Logger<S>,
    loggers: Vec<Logger<S>>,
    extra: Vec<Box<dyn Layer<S> + Send + Sync>>,
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
}

impl T4Layer {
    /// The default `Layers` backed by `broker` (`INFO` and above goes to
    /// stdout).
    pub fn default<Reg>() -> T4Layer<Reg>
    where
        Reg: Layer<Reg> + Subscriber + Send + Sync + for<'s> LookupSpan<'s>,
        Logger<Reg>: Layer<Reg>,
    {
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

        T4Layer::new(default, vec![], appenders, Self::mk_extra())
    }

    /// Create a new `Layers` from a default layer and a pre-generated vec of
    /// sub-layers.
    fn new<Reg>(
        default: Logger<Reg>,
        loggers: Vec<Logger<Reg>>,
        appenders: Appenders,
        extra: Vec<DynLayer<Reg>>,
    ) -> T4Layer<Reg> {
        T4Layer {
            enabled: true,
            default,
            loggers,
            appenders,
            extra,
        }
    }

    /// Generate a `Layers` from a config and back it with `broker`.
    ///
    /// # Errors
    /// An error may occur while building the appenders.
    pub fn from_config<Reg>(config: &Config) -> Result<T4Layer<Reg>>
    where
        Reg: Layer<Reg> + Subscriber + Send + Sync + for<'s> LookupSpan<'s>,
        Logger<Reg>: Layer<Reg>,
    {
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

        Ok(T4Layer::new(default, layers, appenders, Self::mk_extra()))
    }
    fn mk_extra<Reg>() -> Vec<DynLayer<Reg>>
    where
        Reg: Layer<Reg> + Subscriber + Send + Sync + for<'s> LookupSpan<'s>,
    {
        let layer = tracing_tree::HierarchicalLayer::default()
            .with_indent_lines(true)
            .with_indent_amount(2)
            .with_thread_names(true)
            .with_thread_ids(true)
            .with_verbose_exit(true)
            .with_verbose_entry(true)
            .with_targets(true)
            .with_higher_precision(true);

        let filter = tracing_subscriber::filter::targets::Targets::new()
            .with_target("rasp_ffi", Level::TRACE);

        let filtered = layer.with_filter(filter);

        vec![Box::new(filtered) as DynLayer<Reg>]
    }
}

impl<S> T4Layer<S> {
    /// Disable this subscriber.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enable this subscriber.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

impl<S> Layer<S> for T4Layer<S>
where
    S: Subscriber,
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
        for layer in &self.extra {
            if layer.enabled(metadata, ctx.clone()) {
                layer.on_event(event, ctx.clone());
            }
        }
        // If no other layer logged this then the default one will
        if !any && self.default.enabled(metadata, ctx.clone()) {
            self.default.on_event(event, ctx);
        }
    }
}
