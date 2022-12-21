#![allow(clippy::doc_markdown, clippy::missing_errors_doc)]

use std::{
    process::Command,
    str,
    sync::RwLock,
};

use once_cell::sync::Lazy;
use thiserror::Error;
use time::{
    error::{
        ComponentRange,
        Format,
    },
    format_description::{
        parse,
        FormatItem,
    },
    Duration,
    OffsetDateTime,
    UtcOffset,
};

/// Possible errors
#[derive(Error, Debug)]
pub enum Error {
    /// Failure acquiring the write lock as it was likely poisoned.
    #[error("Unable to acquire a write lock")]
    WriteLock,

    /// Failure acquiring the read lock as it was likely poisoned.
    #[error("Unable to acquire a read lock")]
    ReadLock,

    /// The values used to create a UTC Offset were invalid.
    #[error("Unable to construct offset from offset hours/minutes: {0}")]
    Time(#[from] ComponentRange),

    /// The library was failed to create a timestamp string from a date/time
    /// struct
    #[error("Unable to format timestamp: {0}")]
    TimeFormat(#[from] Format),

    /// An invalid value for the offset hours was passed in.
    #[error("Invalid offset hours: {0}")]
    InvalidOffsetHours(i8),

    /// An invalid value for the offset minutes was passed in.
    #[error("Invalid offset minutes: {0}")]
    InvalidOffsetMinutes(i8),

    /// An invalid value for the offset minutes was passed in.
    #[error("Unable to parse offset string.")]
    InvalidOffsetString,
}

static OFFSET: Lazy<RwLock<Option<UtcOffset>>> = Lazy::new(|| RwLock::new(None));
static TIME_FORMAT: Lazy<Vec<FormatItem<'static>>> = Lazy::new(|| {
    parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_second]",
    )
    .unwrap_or_default()
});
static PARSE_FORMAT: Lazy<Vec<FormatItem<'static>>> =
    Lazy::new(|| parse("[offset_hour][offset_minute]").unwrap_or_default());
static PARSE_FORMAT_WITH_COLON: Lazy<Vec<FormatItem<'static>>> =
    Lazy::new(|| parse("[offset_hour]:[offset_minute]").unwrap_or_default());

/// Sets a static UTC offset, from an input string, to use with future calls to
/// `get_local_timestamp_rfc3339`. The format should be [+/-]HHMM.
///
/// # Arguments
/// * input - The UTC offset as a string. Example values are: +0900, -0930,
///   1000, +09:00, -09:30, 10:00
///
/// # Returns
/// Returns a `Result` of either the inputed offset hours/minutes or an Error if
/// the method fails.
pub fn set_global_offset_from_str(input: &str) -> Result<(i8, i8), Error> {
    let trimmed = trim_new_lines(input);
    if let Ok(o) = UtcOffset::parse(trimmed, &PARSE_FORMAT) {
        init_from_utc_offset(o)
    } else if let Ok(o) = UtcOffset::parse(trimmed, &PARSE_FORMAT_WITH_COLON) {
        init_from_utc_offset(o)
    } else {
        Err(Error::InvalidOffsetString)
    }
}

/// Sets a static UTC offset to use with future calls to
/// `get_local_timestamp_rfc3339`
///
/// # Arguments
/// * offset_hours - the hour value of the UTC offset, cannot be less than -12
///   or greater than 14
/// * offset_minutes - the minute value of the UTC offset, cannot be less than 0
///   or greater than 59
///
/// # Returns
/// Returns a `Result` of either the inputed offset hours/minutes or an Error if
/// the method fails.
#[allow(clippy::manual_range_contains)]
pub fn set_global_offset(offset_hours: i8, offset_minutes: i8) -> Result<(i8, i8), Error> {
    if offset_hours < -12 || offset_hours > 14 {
        Err(Error::InvalidOffsetHours(offset_hours))
    } else if !(0..=59).contains(&offset_minutes) {
        Err(Error::InvalidOffsetMinutes(offset_minutes))
    } else {
        let o = UtcOffset::from_hms(offset_hours, offset_minutes, 0)?;
        init_from_utc_offset(o)
    }
}

/// Gets a timestamp string using in either the local offset or +00:00
///
/// # Returns
/// Returns a `Result` of either the timestamp in the following format
/// "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour
/// sign:mandatory]:[offset_second]", or an error if the method fails.
/// The timezone will be in the local offset IF any of the following succeed:
///     1.) set_global_offset is called.
///     2.) `time::UtcOffset::current_local_offset()` works
///     3.) The library is able to query the timezone using system commands.
/// If none succeed, we default to UTC.
pub fn get_local_timestamp_rfc3339() -> Result<String, Error> {
    get_local_timestamp_from_offset_rfc3339(get_local_offset())
}

/// Gets a timestamp string using the specified offset
///
/// # Arguments
/// * utc_offset - A caller specified offset
///
/// # Returns
/// Returns a `Result` of either the timestamp in the following format
/// "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour
/// sign:mandatory]:[offset_second]", or an error if the method fails.
#[allow(clippy::cast_lossless)]
pub fn get_local_timestamp_from_offset_rfc3339(utc_offset: UtcOffset) -> Result<String, Error> {
    let datetime_now = OffsetDateTime::now_utc();
    if utc_offset != UtcOffset::UTC {
        if let Some(t) = datetime_now.checked_add(Duration::hours(utc_offset.whole_hours() as i64))
        {
            let offset_datetime_now = t.replace_offset(utc_offset);
            return Ok(offset_datetime_now.format(&TIME_FORMAT)?);
        }
    }

    Ok(datetime_now.format(&TIME_FORMAT)?)
}

fn init_from_utc_offset(offset: UtcOffset) -> Result<(i8, i8), Error> {
    if let Ok(mut l) = OFFSET.write() {
        *l = Some(offset);
    } else {
        log::warn!("UTC Offset failed: {}", offset);
        return Err(Error::WriteLock);
    }
    log::info!("UTC Offset set to: {}", offset);
    Ok((offset.whole_hours(), offset.minutes_past_hour()))
}

pub fn get_local_offset() -> UtcOffset {
    if let Ok(reader) = OFFSET.read() {
        if let Some(o) = *reader {
            return o;
        }
    }

    let offset = if let Ok(o) = time::UtcOffset::current_local_offset() {
        o
    } else if let Some(o) = offset_from_process() {
        o
    } else {
        UtcOffset::UTC
    };

    if let Err(e) = init_from_utc_offset(offset) {
        log::warn!("Unable to initialize offset: {}", e);
    }

    offset
}

fn process_cmd_output(stdout: &[u8], formatter: &[FormatItem<'static>]) -> Option<UtcOffset> {
    match str::from_utf8(stdout) {
        Ok(v) => match UtcOffset::parse(trim_new_lines(v), &formatter) {
            Ok(o) => return Some(o),
            Err(e) => {
                log::warn!("Unable to parse output: {}", e);
            },
        },
        Err(e) => {
            log::warn!("Unable to convert output: {}", e);
        },
    }
    None
}

fn offset_from_process() -> Option<UtcOffset> {
    if cfg!(target_os = "windows") {
        if let Ok(output) = Command::new("powershell")
            .arg("Get-Date")
            .arg("-Format")
            .arg("\"K \"")
            .output()
        {
            // The space in "K " is intentional. Thanks Powershell
            return process_cmd_output(&output.stdout, &PARSE_FORMAT_WITH_COLON);
        }
    } else if let Ok(output) = Command::new("date").arg("+%z").output() {
        return process_cmd_output(&output.stdout, &PARSE_FORMAT);
    }

    None
}

fn trim_new_lines(s: &str) -> &str {
    s.trim().trim_end_matches("\r\n").trim_matches('\n')
}
