#![allow(clippy::single_char_lifetime_names)]
use core::fmt;
use std::{borrow::Cow, io};

use once_cell::sync::Lazy;
use trace4rs_fmtorp::FieldValueWriter;
use tracing::{field::Visit, metadata::LevelFilter, Event, Metadata, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    fmt::{
        format::{self, DefaultFields, Format, Full, Writer},
        time::FormatTime,
        writer::{BoxMakeWriter, MakeWriterExt},
        FmtContext, FormatEvent, FormatFields, Layer as FmtLayer,
    },
    layer::{Context, Layered},
    prelude::__tracing_subscriber_SubscriberExt,
    registry::LookupSpan,
    Layer, Registry,
};

use super::PolyLayer;
use crate::{
    appenders::Appenders,
    config::{AppenderId, Format as ConfigFormat, Target},
};

const TIME_FORMAT: time::format_description::well_known::Rfc3339 =
    time::format_description::well_known::Rfc3339;

static NORMAL_FMT: Lazy<Format<Full, UtcOffsetTime>> =
    Lazy::new(|| Format::default().with_timer(UtcOffsetTime).with_ansi(false));

pub struct Logger<Reg = Registry, N = DefaultFields, F = EventFormatter> {
    level: LevelFilter,
    target: Option<Target>,
    layer: Layered<FmtLayer<Reg, N, F, BoxMakeWriter>, Reg>,
}
impl<B> Logger<B> {
    fn is_enabled(&self, meta: &Metadata<'_>) -> bool {
        let match_level = meta.level() <= &self.level;
        let match_target = self
            .target
            .as_ref()
            .map_or(true, |t| meta.target().starts_with(t.as_str()));

        match_level && match_target
    }
}
impl Logger {
    pub fn new_erased<'a, B>(
        b: B,
        level: LevelFilter,
        target: Option<Target>,
        ids: impl IntoIterator<Item = &'a AppenderId>,
        appenders: &Appenders,
        format: EventFormatter,
    ) -> PolyLayer<B>
    where
        B: Subscriber + Send + Sync + for<'b> LookupSpan<'b>,
        Logger<B>: Layer<B>,
    {
        Box::new(Self::new(
            b,
            level,
            target,
            ids.into_iter(),
            appenders,
            format,
        ))
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

    pub fn new<'a, B>(
        r: B,
        level: LevelFilter,
        target: Option<Target>,
        ids: impl Iterator<Item = &'a AppenderId>,
        appenders: &Appenders,
        format: EventFormatter,
    ) -> Logger<B, DefaultFields, EventFormatter>
    where
        B: Subscriber + Send + Sync + for<'b> LookupSpan<'b>,
        tracing_subscriber::fmt::Layer<B, DefaultFields, EventFormatter, BoxMakeWriter>: Layer<B>,
    {
        let writer =
            Self::mk_writer(ids, appenders).unwrap_or_else(|| BoxMakeWriter::new(io::sink));

        let fmt_layer = FmtLayer::default().event_format(format).with_ansi(false);
        let append_layer = fmt_layer.with_writer(writer);
        // let layer = append_layer;
        let layer = r.with(append_layer);

        Logger {
            level,
            target,
            layer,
        }
    }
}
impl<Sub> Layer<Sub> for Logger<Sub>
where
    Sub: Subscriber,
    FmtLayer<Sub, DefaultFields, EventFormatter, BoxMakeWriter>: Layer<Sub>,
{
    fn enabled(&self, meta: &Metadata<'_>, _ctx: Context<'_, Sub>) -> bool {
        Logger::is_enabled(self, meta)
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, Sub>) {
        self.layer.event(event);
    }
}

#[derive(Debug)]
pub enum EventFormatter {
    Normal,
    MessageOnly,
    Custom(CustomFormatter),
}

impl Default for EventFormatter {
    fn default() -> Self {
        Self::Normal
    }
}

impl From<ConfigFormat> for EventFormatter {
    fn from(f: ConfigFormat) -> Self {
        match f {
            ConfigFormat::Normal => Self::Normal,
            ConfigFormat::MessageOnly => Self::MessageOnly,
            ConfigFormat::Custom(s) => {
                match CustomFormatter::new(s) {
                    Ok(c) => Self::Custom(c),
                    #[allow(clippy::print_stderr)] // necessary error surfacing
                    Err(e) => {
                        eprintln!(
                            "trace4rs: Error parsing logger custom format: {e}, using default \
                             formatter"
                        );
                        Self::default()
                    },
                }
            },
        }
    }
}

impl<'w, 'ctx, 'evt, S> FormatEvent<S, DefaultFields> for EventFormatter
where
    S: Subscriber + for<'b> LookupSpan<'b>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, DefaultFields>,
        writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Custom(fmtr) => fmtr.format_event(ctx, writer, event),
            Self::MessageOnly => {
                let mut vs = SingleFieldVisitor::new(true, writer, MESSAGE_FIELD_NAME);
                event.record(&mut vs);
                Ok(())
            },
            Self::Normal => NORMAL_FMT.format_event(ctx, writer, event),
        }
    }
}
mod fields {
    use std::collections::HashSet;

    pub const TIMESTAMP: &str = "T";
    pub const TIMESTAMP_UTC: &str = "T(utc)";
    pub const TARGET: &str = "t";
    pub const MESSAGE: &str = "m";
    pub const FIELDS: &str = "f";
    pub const LEVEL: &str = "l";
    pub static FIELD_SET: once_cell::sync::Lazy<HashSet<&'static str>> =
        once_cell::sync::Lazy::new(|| {
            let mut set = HashSet::new();
            set.insert(TIMESTAMP);
            set.insert(TIMESTAMP_UTC);
            set.insert(TARGET);
            set.insert(MESSAGE);
            set.insert(FIELDS);
            set.insert(LEVEL);
            set
        });
}

struct CustomValueWriter<'ctx, 'evt, Broker> {
    ctx: &'ctx FmtContext<'ctx, Broker, DefaultFields>,
    event: &'evt Event<'evt>,
}
impl<'ctx, 'evt, Broker> CustomValueWriter<'ctx, 'evt, Broker> {
    fn format_timestamp(mut writer: format::Writer<'_>) -> fmt::Result {
        use tracing_subscriber::fmt::time::OffsetTime;

        let (o, _) = utc_offset::get_utc_offset();
        let t = OffsetTime::new(o, TIME_FORMAT);
        t.format_time(&mut writer)
    }

    fn format_timestamp_utc(mut writer: format::Writer<'_>) -> fmt::Result {
        let t = tracing_subscriber::fmt::time::UtcTime::rfc_3339();
        t.format_time(&mut writer)
    }
}
impl<'ctx, 'evt, Broker> FieldValueWriter for CustomValueWriter<'ctx, 'evt, Broker>
where
    Broker: 'static,
    for<'writer> FmtContext<'ctx, Broker, DefaultFields>: FormatFields<'writer>,
{
    fn write_value(&self, mut writer: format::Writer<'_>, field: &'static str) -> fmt::Result {
        let normalized_meta = self.event.normalized_metadata();
        let meta = normalized_meta
            .as_ref()
            .unwrap_or_else(|| self.event.metadata());

        if field == fields::TIMESTAMP {
            Self::format_timestamp(writer)?;
        } else if field == fields::TIMESTAMP_UTC {
            Self::format_timestamp_utc(writer)?;
        } else if field == fields::TARGET {
            write!(writer, "{}", meta.target())?;
        } else if field == fields::MESSAGE {
            let mut vs = SingleFieldVisitor::new(false, writer.by_ref(), MESSAGE_FIELD_NAME);
            self.event.record(&mut vs);
        } else if field == fields::FIELDS {
            self.ctx.format_fields(writer, self.event)?;
        } else if field == fields::LEVEL {
            write!(writer, "{}", meta.level())?;
        }
        Ok(())
    }
}
/// EAS: Follow strat from `NORMAL_FMT`
/// move Message only  and this to formatter.rs and utcoffsettime
#[derive(Debug)]
pub struct CustomFormatter {
    fmtr: trace4rs_fmtorp::Fmtr<'static>,
}
// SAFETY:
// `CustomFormatter` is safe to sync
unsafe impl Sync for CustomFormatter {}
// SAFETY:
// `CustomFormatter` is safe to send
unsafe impl Send for CustomFormatter {}
impl CustomFormatter {
    fn new(fmt_str: impl Into<Cow<'static, str>>) -> Result<Self, trace4rs_fmtorp::Error> {
        let fmtr = trace4rs_fmtorp::Fmtr::new(fmt_str, &fields::FIELD_SET)?;

        Ok(Self { fmtr })
    }

    fn format_event<'ctx, 'evt, 'w, S>(
        &self,
        ctx: &FmtContext<'ctx, S, DefaultFields>,
        writer: Writer<'w>,
        event: &Event<'evt>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'b> LookupSpan<'b>,
    {
        let value_writer = CustomValueWriter { ctx, event };
        self.fmtr.write(writer, &value_writer)
    }
}

const MESSAGE_FIELD_NAME: &str = "message";

struct SingleFieldVisitor<'w> {
    newline: bool,
    writer: Writer<'w>,
    field_name: Cow<'static, str>,
}
impl<'w> SingleFieldVisitor<'w> {
    fn new(newline: bool, writer: Writer<'w>, field_name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            newline,
            writer,
            field_name: field_name.into(),
        }
    }
}
impl<'w> Visit for SingleFieldVisitor<'w> {
    // todo(eas): Might be good to come back to this, looks like this is getting
    // called directly by tracing-subscriber on accident.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // eas: bummer to hardcode this but thats how tracing does it
        #[allow(unused_must_use, clippy::use_debug)]
        if field.name() == self.field_name {
            if self.newline {
                writeln!(self.writer, "{:?}", value);
            } else {
                write!(self.writer, "{:?}", value);
            }
        }
    }
}

struct UtcOffsetTime;

impl FormatTime for UtcOffsetTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        use tracing_subscriber::fmt::time::OffsetTime;

        let (o, _) = utc_offset::get_utc_offset();
        let t = OffsetTime::new(o, TIME_FORMAT);
        t.format_time(w)
    }
}
