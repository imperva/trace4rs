#![allow(clippy::single_char_lifetime_names)]
use core::fmt;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Write,
    marker::PhantomPinned,
    ops::RangeInclusive,
    pin::Pin,
    ptr::NonNull,
};

// todo(eas): O(1) equality
#[derive(Eq)]
pub struct Field<'f>(usize, Cow<'f, str>);
impl<'f> Field<'f> {
    fn new(i: usize, s: &'f str) -> Self {
        Self(i, Cow::Borrowed(s))
    }

    pub fn id(&self) -> usize {
        self.0
    }
}
impl<'f> PartialEq<Self> for Field<'f> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub struct Fmtr<'s> {
    /// The owned or static borrowed format string.
    fmt_str:      Cow<'s, str>,
    /// The ranges indexing `fmt_str` which 1-1 index `ordered_fields`.
    field_ranges: Vec<RangeInclusive<usize>>,
    /// Lookup table for field names.
    field_idxs:   Option<HashMap<NonNull<str>, usize>>,
    /// We need pin for field_idxs
    _pin:         PhantomPinned,
}
impl Fmtr<'static> {
    pub fn new(fmt_str: impl Into<Cow<'static, str>>) -> Pin<Box<Self>> {
        let fmt_str = fmt_str.into();
        let mut start = None;
        let mut in_escape = false;
        let mut field_ranges: Vec<RangeInclusive<usize>> = Default::default();

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
        let mut this = Box::pin(Self {
            fmt_str,
            field_ranges,
            field_idxs: None,
            _pin: PhantomPinned,
        });
        let mut field_idxs = HashMap::new();
        for (i, r) in this.field_ranges.iter().enumerate() {
            field_idxs.insert(NonNull::from(&this.fmt_str[r.clone()]), i);
        }
        unsafe {
            let mut_ref = Pin::as_mut(&mut this);
            Pin::get_unchecked_mut(mut_ref).field_idxs = Some(field_idxs);
        }
        // this.as_mut().map_unchecked_mut(|s| s.field_idxs.replace(field_idxs)) }
        this
    }

    pub fn field_from_id<'this>(&'this self, i: usize) -> Option<Field<'this>> {
        if let Some(rg) = self.field_ranges.get(i).cloned() {
            Some(Field::new(i, &self.fmt_str[rg]))
        } else {
            None
        }
    }

    pub fn field_id_from_name(&self, name: &str) -> Option<usize> {
        let nname = NonNull::from(name);
        // unwrap ok since invariant
        self.field_idxs.as_ref().unwrap().get(&nname).cloned()
    }

    pub fn write(
        &self,
        writer: &mut impl Write,
        value_writer: impl FieldValueWriter,
    ) -> fmt::Result {
        let mut last = 0;
        for (i, range) in self.field_ranges.iter().enumerate() {
            // todo: rm
            println!("range: {range:?}");

            // write everything from the last field to start of next
            write!(writer, "{}", &self.fmt_str[last..*range.start()])?;

            // unwrap ok since idxs coming from same vec
            let field = self.field_from_id(i).unwrap();

            value_writer.write_value(writer, field)?;

            // safe since we inserted above
            last = range.end() + 1;
        }
        write!(writer, "{}", &self.fmt_str[last..])?;
        writeln!(writer)
    }
}

pub trait FieldValueWriter {
    fn write_value(&self, writer: &mut impl Write, field: Field) -> fmt::Result;
}
