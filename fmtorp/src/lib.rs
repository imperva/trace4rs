#![allow(clippy::single_char_lifetime_names)]
use core::fmt;
use std::{
    borrow::Cow,
    collections::HashSet,
    ops::RangeInclusive,
};

use tracing_subscriber::fmt::format;

pub struct Fmtr<'s> {
    /// The owned or static borrowed format string.
    fmt_str:      Cow<'s, str>,
    /// The ranges indexing `fmt_str` which 1-1 index `ordered_fields`.
    field_ranges: Vec<RangeInclusive<usize>>,
    /// The names of fields indexed identically to field_ranges.
    field_names:  Vec<&'static str>,
}
impl<'fmtstr> Fmtr<'fmtstr> {
    /// Unrecognized fields should be an error
    pub fn new(fmt_str: impl Into<Cow<'fmtstr, str>>, fields: &HashSet<&'static str>) -> Self {
        let fmt_str = fmt_str.into();
        let mut start = None;
        let mut in_escape = false;
        let mut field_ranges: Vec<RangeInclusive<usize>> = Vec::with_capacity(fields.len());
        let mut field_names: Vec<&'static str> = Vec::with_capacity(fields.len());

        for (xi, x) in fmt_str.char_indices() {
            // inside a field match
            if let Some(strt) = start {
                // illegal chars
                if x == '{' || x == '\\' {
                    panic!("illegal char inside match block: {x}")
                }
                // end match
                if x == '}' {
                    field_ranges.push(strt..=xi);
                    let ff = &fmt_str[(strt + 1)..xi];
                    // unwrap None is a bug
                    let f = fields.get(ff).unwrap();
                    field_names.push(*f);
                    start = None;
                }
            } else {
                // match escape
                if x == '\\' {
                    in_escape = true;
                    continue; // avoid the reset at bottom
                }
                // match unescaped brackets
                if !in_escape {
                    if x == '}' {
                        panic!("unmatched closing brace");
                    } else if x == '{' {
                        start = Some(xi);
                    }
                }
            }
            in_escape = false;
        }
        Self {
            fmt_str,
            field_ranges,
            field_names,
        }
    }

    pub fn field_from_id<'this>(&'this self, i: usize) -> Option<&'static str> {
        self.field_names.get(i).map(|f| *f)
    }

    pub fn write<'writer>(
        &self,
        mut writer: format::Writer<'writer>,
        value_writer: &impl FieldValueWriter,
    ) -> fmt::Result {
        let mut last = 0;
        for (i, range) in self.field_ranges.iter().enumerate() {
            // write everything from the last field to start of next
            write!(writer.by_ref(), "{}", &self.fmt_str[last..*range.start()])?;

            // unwrap ok since idxs coming from same vec
            let field = self.field_from_id(i).unwrap();

            value_writer.write_value(writer.by_ref(), field)?;

            // safe since we inserted above
            last = range.end() + 1;
        }
        write!(writer, "{}", &self.fmt_str[last..])?;
        writeln!(writer)
    }
}

pub trait FieldValueWriter {
    fn write_value<'writer>(
        &self,
        writer: format::Writer<'writer>,
        field: &'static str,
    ) -> fmt::Result;
}
