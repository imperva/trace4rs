#![allow(clippy::single_char_lifetime_names)]
use core::fmt;
use std::{
    borrow::Cow,
    collections::HashSet,
    ops::RangeInclusive,
};

use tracing_subscriber::fmt::format;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Illegal character {0}: {1}")]
    IllegalCharacter(char, String),

    #[error("Unknown field: {0}")]
    UnknownField(String),

    #[error("Empty field found")]
    EmptyField,
}

pub struct Fmtr<'fmtstr> {
    /// The owned or static borrowed format string.
    fmt_str:      Cow<'fmtstr, str>,
    /// The ranges indexing `fmt_str` which 1-1 index `ordered_fields`.
    /// # Invariants
    /// Ranges are strictly within bounds of fmt_str
    field_ranges: Vec<RangeInclusive<usize>>,
    /// The names of fields indexed identically to field_ranges.
    field_names:  Vec<&'static str>,
}
impl<'fmtstr> Fmtr<'fmtstr> {
    /// Unrecognized fields should be an error
    /// # Errors
    /// We could encounter an illegal character or unknown field
    pub fn new(
        fmt_str: impl Into<Cow<'fmtstr, str>>,
        fields: &HashSet<&'static str>,
    ) -> Result<Self, Error> {
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
                    return Err(Error::IllegalCharacter(
                        x,
                        "found inside field name block".to_string(),
                    ));
                }
                // end match
                if x == '}' {
                    #[allow(clippy::integer_arithmetic)] // no overflow potential
                    if strt + 1 == xi {
                        return Err(Error::EmptyField);
                    }
                    field_ranges.push(strt..=xi);
                    // safe since we know the slice is non-empty and xi in bounds
                    // and no overflow potential
                    #[allow(clippy::indexing_slicing, clippy::integer_arithmetic)]
                    let ff = &fmt_str[(strt + 1)..xi];
                    if let Some(f) = fields.get(ff) {
                        field_names.push(*f);
                        start = None;
                    } else {
                        return Err(Error::UnknownField(ff.to_string()));
                    }
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
                        return Err(Error::IllegalCharacter(
                            x,
                            "found outside field name block".to_string(),
                        ));
                    } else if x == '{' {
                        start = Some(xi);
                    }
                }
            }
            in_escape = false;
        }
        Ok(Self {
            fmt_str,
            field_ranges,
            field_names,
        })
    }

    pub fn field_from_id(&self, i: usize) -> Option<&'static str> {
        self.field_names.get(i).copied()
    }

    /// # Errors
    /// If we fail to format the value a fmt Err will be returned
    ///
    /// # Panics
    /// Panics should only happen on bugs.
    #[allow(clippy::unwrap_in_result)]
    pub fn write<'writer>(
        &self,
        mut writer: format::Writer<'writer>,
        value_writer: &impl FieldValueWriter,
    ) -> fmt::Result {
        let mut last = 0;
        for (i, range) in self.field_ranges.iter().enumerate() {
            // write everything from the last field to start of next

            // safe since last range.start is inbounds as invariant of Fmtr.
            #[allow(clippy::indexing_slicing)]
            write!(writer.by_ref(), "{}", &self.fmt_str[last..*range.start()])?;

            // unwrap ok since idxs coming from same vec
            #[allow(clippy::unwrap_used)]
            let field = self.field_from_id(i).unwrap();

            value_writer.write_value(writer.by_ref(), field)?;

            // last may run off the end if last range
            #[allow(clippy::integer_arithmetic)]
            {
                last = range.end() + 1;
            }
        }
        // safe since if we're off the end it will just be an empty slice
        #[allow(clippy::indexing_slicing)]
        write!(writer, "{}", &self.fmt_str[last..])?;
        writeln!(writer)
    }
}

pub trait FieldValueWriter {
    /// # Errors
    /// If we fail to format the value a fmt Err will be returned
    fn write_value(&self, writer: format::Writer<'_>, field: &'static str) -> fmt::Result;
}
