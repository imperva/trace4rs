#![allow(clippy::single_char_lifetime_names)]
use std::io;

use tracing::{
    metadata::LevelFilter,
    Event,
    Metadata,
    Subscriber,
};
use tracing_subscriber::{
    fmt::{
        format::DefaultFields,
        writer::{
            BoxMakeWriter,
            MakeWriterExt,
        },
        Layer as FmtLayer,
    },
    layer::Context,
    registry::LookupSpan,
    Layer,
    Registry,
};

use super::formatter::EventFormatter;
use crate::{
    appenders::Appenders,
    config::{
        AppenderId,
        Target,
    },
};

pub struct Logger<Reg = Registry, N = DefaultFields, F = EventFormatter> {
    level:  LevelFilter,
    target: Option<Target>,
    layer:  FmtLayer<Reg, N, F, BoxMakeWriter>,
}

impl<Reg> Logger<Reg>
where
    Reg: Subscriber + for<'s> LookupSpan<'s>,
{
    pub fn new<'a>(
        level: LevelFilter,
        target: Option<Target>,
        ids: impl Iterator<Item = &'a AppenderId>,
        appenders: &Appenders,
        format: EventFormatter,
    ) -> Logger<Reg>
    where
        Reg: Subscriber + for<'s> LookupSpan<'s>,
    {
        let writer = mk_writer(ids, appenders).unwrap_or_else(|| BoxMakeWriter::new(io::sink));

        let fmt_layer = FmtLayer::default().event_format(format).with_ansi(false);
        let layer = fmt_layer.with_writer(writer);

        Logger {
            level,
            target,
            layer,
        }
    }
}

impl<Reg, N, F> Logger<Reg, N, F> {
    fn is_enabled(&self, meta: &Metadata<'_>) -> bool {
        let match_level = meta.level() <= &self.level;
        let match_target = self
            .target
            .as_ref()
            .map_or(true, |t| meta.target().starts_with(t.as_str()));

        match_level && match_target
    }
}

impl<Reg, N, F> Layer<Reg> for Logger<Reg, N, F>
where
    Reg: Subscriber + for<'a> LookupSpan<'a>,
    FmtLayer<Reg, N, F, BoxMakeWriter>: Layer<Reg>,
{
    fn enabled(&self, meta: &Metadata<'_>, _ctx: Context<'_, Reg>) -> bool {
        Logger::is_enabled(self, meta)
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, Reg>) {
        self.layer.on_event(event, ctx);
    }
}

fn mk_writer<'a>(
    ids: impl Iterator<Item = &'a AppenderId>,
    appenders: &Appenders,
) -> Option<BoxMakeWriter> {
    let mut acc_mw = None;
    for id in ids {
        if let Some(appender) = appenders.get(id).map(ToOwned::to_owned) {
            acc_mw = if let Some(acc) = acc_mw.take() {
                Some(BoxMakeWriter::new(MakeWriterExt::and(acc, appender)))
            } else {
                Some(BoxMakeWriter::new(appender))
            }
        }
    }
    acc_mw
}
