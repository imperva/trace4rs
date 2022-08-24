#![allow(clippy::single_char_lifetime_names)]
use std::io;

use once_cell::sync::Lazy;
use tracing::{
    field::Visit,
    metadata::LevelFilter,
    Event,
    Metadata,
};
use tracing_subscriber::{
    fmt::{
        format::{
            DefaultFields,
            Format,
            Full,
            Writer,
        },
        time::FormatTime,
        writer::{
            BoxMakeWriter,
            MakeWriterExt,
        },
        FormatEvent,
        Layer as FmtLayer,
    },
    layer::{
        Context,
        Layered,
    },
    prelude::__tracing_subscriber_SubscriberExt,
    Layer,
};

use super::{
    span_broker::SpanBroker,
    PolyLayer,
};
use crate::{
    appenders::Appenders,
    config::{
        AppenderId,
        Format as ConfigFormat,
        Target,
    },
};

static NORMAL_FMT: Lazy<Format<Full, UtcOffsetTime>> =
    Lazy::new(|| Format::default().with_timer(UtcOffsetTime).with_ansi(false));

pub struct Logger<N = DefaultFields, F = EventFormatter> {
    level:  LevelFilter,
    target: Option<Target>,
    layer:  Layered<FmtLayer<SpanBroker, N, F, BoxMakeWriter>, SpanBroker>,
}
impl Logger {
    pub fn new_erased<'a>(
        r: SpanBroker,
        level: LevelFilter,
        target: Option<Target>,
        ids: impl IntoIterator<Item = &'a AppenderId>,
        appenders: &Appenders,
        format: EventFormatter,
    ) -> PolyLayer<SpanBroker> {
        Box::new(Self::new(
            r,
            level,
            target,
            ids.into_iter(),
            appenders,
            format,
        ))
    }

    fn is_enabled(&self, meta: &Metadata<'_>) -> bool {
        let match_level = meta.level() <= &self.level;
        let match_target = self
            .target
            .as_ref()
            .map_or(true, |t| meta.target().starts_with(t.as_str()));

        match_level && match_target
    }

    fn mk_writer<'a>(
        ids: impl Iterator<Item = &'a AppenderId>,
        appenders: &Appenders,
    ) -> Option<BoxMakeWriter> {
        let mut accumulated_makewriter = None;
        for id in ids {
            if let Some(appender) = appenders.get(id).map(ToOwned::to_owned) {
                accumulated_makewriter = if let Some(acc) = accumulated_makewriter.take() {
                    Some(BoxMakeWriter::new(MakeWriterExt::and(acc, appender)))
                } else {
                    Some(BoxMakeWriter::new(appender))
                }
            }
        }
        accumulated_makewriter
    }

    pub fn new<'a>(
        r: SpanBroker,
        level: LevelFilter,
        target: Option<Target>,
        ids: impl Iterator<Item = &'a AppenderId>,
        appenders: &Appenders,
        format: EventFormatter,
    ) -> Self {
        let writer =
            Self::mk_writer(ids, appenders).unwrap_or_else(|| BoxMakeWriter::new(io::sink));

        let fmt_layer = FmtLayer::default().event_format(format).with_ansi(false);
        let append_layer = fmt_layer.with_writer(writer);
        let layer = r.with(append_layer);

        Self {
            level,
            target,
            layer,
        }
    }
}
impl Layer<SpanBroker> for Logger {
    fn enabled(&self, meta: &Metadata<'_>, _ctx: Context<'_, SpanBroker>) -> bool {
        Logger::is_enabled(self, meta)
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, SpanBroker>) {
        self.layer.on_event(event, ctx);
    }
}

pub enum EventFormatter {
    Normal,
    MessageOnly,
}

impl From<ConfigFormat> for EventFormatter {
    fn from(f: ConfigFormat) -> Self {
        match f {
            ConfigFormat::Normal => Self::Normal,
            ConfigFormat::MessageOnly => Self::MessageOnly,
        }
    }
}

impl FormatEvent<SpanBroker, DefaultFields> for EventFormatter {
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, SpanBroker, DefaultFields>,
        writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::MessageOnly => {
                let mut vs = MessageOnlyVisitor::new(writer);
                event.record(&mut vs);
                Ok(())
            },
            Self::Normal => NORMAL_FMT.format_event(ctx, writer, event),
        }
    }
}

struct MessageOnlyVisitor<'w> {
    writer: tracing_subscriber::fmt::format::Writer<'w>,
}
impl<'w> MessageOnlyVisitor<'w> {
    fn new(writer: tracing_subscriber::fmt::format::Writer<'w>) -> Self {
        Self { writer }
    }
}
impl<'w> Visit for MessageOnlyVisitor<'w> {
    // todo(eas): Might be good to come back to this, looks like this is getting
    // called directly by tracing-subscriber on accident.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // eas: bummer to hardcode this but thats how tracing does it
        #[allow(unused_must_use, clippy::use_debug)]
        if field.name() == "message" {
            writeln!(self.writer, "{:?}", value);
        }
    }
}

const TIME_FORMAT: time::format_description::well_known::Rfc3339 =
    time::format_description::well_known::Rfc3339;

struct UtcOffsetTime;

impl FormatTime for UtcOffsetTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let ts =
            time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
        let ts_str = ts.format(&TIME_FORMAT).unwrap_or_default();

        w.write_str(&ts_str)
    }
}
