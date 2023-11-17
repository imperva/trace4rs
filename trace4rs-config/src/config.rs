//! Configuration structures which can be used for file based `trace4rs` config.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    result,
    str::FromStr,
};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smart_default::SmartDefault;
use tracing::level_filters;

use crate::error::{Error, Result};

/// The root configuration object containing everything necessary to build a
/// `trace4rs::Handle`.
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Config {
    /// The default logger, which must be configured.
    #[cfg_attr(feature = "serde", serde(rename = "root", alias = "default"))]
    pub default: Logger,
    /// Appenders are assigned an id of your choice and configure actual log
    /// message output.
    #[cfg_attr(
        feature = "in-order-serialization",
        serde(serialize_with = "ordered_map")
    )]
    pub appenders: HashMap<AppenderId, Appender>,
    /// Loggers receive events which match their target and may filter by
    /// message level.
    #[cfg_attr(
        feature = "in-order-serialization",
        serde(serialize_with = "ordered_map")
    )]
    pub loggers: HashMap<Target, Logger>,
}

/// # Errors
/// Returns an error if serialization fails
#[cfg(feature = "in-order-serialization")]
pub fn ordered_map<K, V, S>(
    value: &HashMap<K, V>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    K: Ord + Serialize,
    V: Serialize,
    S: Serializer,
{
    let ordered: std::collections::BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

/// # Errors
/// Returns an error if serialization fails
#[cfg(feature = "in-order-serialization")]
pub fn ordered_set<K, S>(value: &HashSet<K>, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    K: Ord + Serialize,
    S: Serializer,
{
    let ordered: std::collections::BTreeSet<_> = value.iter().collect();
    ordered.serialize(serializer)
}

impl Default for Config {
    fn default() -> Self {
        Self::console_config()
    }
}

impl Config {
    /// A configuration for `INFO` and above to be logged to stdout.
    fn console_config() -> Config {
        use literally::{hmap, hset};

        Config {
            default: Logger {
                level: LevelFilter::INFO,
                appenders: hset! { "stdout" },
                format: Format::default(),
            },
            loggers: hmap! {},
            appenders: hmap! {
                "stdout" => Appender::Console
            },
        }
    }
}
/// A log target, for example to capture all log messages in `trace4rs::config`
/// the target would be `trace4rs::config`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Target(pub String);
impl Target {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<&str> for Target {
    fn from(s: &str) -> Self {
        Target(s.to_string())
    }
}
impl ToString for Target {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

/// An `AppenderId` is an arbitrary string which in the context of a config must
/// be unique.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct AppenderId(pub String);

/// A logger allows for filtering events and delegating to multiple appenders.
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Logger {
    #[cfg_attr(
        feature = "in-order-serialization",
        serde(serialize_with = "ordered_set")
    )]
    pub appenders: HashSet<AppenderId>,
    pub level: LevelFilter,
    #[cfg_attr(
        feature = "serde",
        serde(default = "Format::default", skip_serializing_if = "Format::is_normal")
    )]
    pub format: Format,
}

#[cfg(feature = "serde")]
macro_rules! named_unit_variant {
    ($variant:ident) => {
        pub mod $variant {
            pub fn serialize<S>(serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(stringify!($variant))
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = ();

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str(concat!("\"", stringify!($variant), "\""))
                    }

                    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                        if value == stringify!($variant) {
                            Ok(())
                        } else {
                            Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
                        }
                    }
                }
                deserializer.deserialize_str(V)
            }
        }
    };
}

#[cfg(feature = "serde")]
mod format {
    named_unit_variant!(normal);
    named_unit_variant!(messageonly);

    pub mod custom {
        pub fn serialize<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(value)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct V;
            impl<'de> serde::de::Visitor<'de> for V {
                type Value = String;

                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str(concat!(r#"{ "custom": "<format string>" }"#))
                }

                fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                    if value != "messageonly" && value != "normal" {
                        Ok(value.to_string())
                    } else {
                        Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
                    }
                }
            }
            deserializer.deserialize_str(V)
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, SmartDefault)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(untagged, rename_all = "lowercase")
)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum Format {
    #[default]
    #[cfg_attr(feature = "serde", serde(with = "format::normal"))]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Normal,
    #[cfg_attr(feature = "serde", serde(with = "format::messageonly"))]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    MessageOnly,
    #[cfg_attr(feature = "serde", serde(with = "format::custom"))]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Custom(String),
}
impl Format {
    #[cfg(feature = "serde")]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }
}

/// Simply a wrapper around `tracing::LevelFilter` such that it can be used by
/// `serde`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema), schemars(transparent))]
pub struct LevelFilter(
    #[cfg_attr(feature = "schemars", schemars(with = "String"))] level_filters::LevelFilter,
);
impl From<LevelFilter> for level_filters::LevelFilter {
    fn from(l: LevelFilter) -> Self {
        l.0
    }
}

#[rustfmt::skip] // eas: retain order
impl LevelFilter {
    pub const TRACE: Self = LevelFilter(level_filters::LevelFilter::TRACE);
    pub const DEBUG: Self = LevelFilter(level_filters::LevelFilter::DEBUG);
    pub const INFO: Self = LevelFilter(level_filters::LevelFilter::INFO);
    pub const WARN: Self = LevelFilter(level_filters::LevelFilter::WARN);
    pub const ERROR: Self = LevelFilter(level_filters::LevelFilter::ERROR);
    pub const OFF: Self = LevelFilter(level_filters::LevelFilter::OFF);
    #[must_use] pub const fn maximum() -> Self {
        Self::TRACE
    }
}

#[cfg(feature = "serde")]
impl Serialize for LevelFilter {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string().to_ascii_uppercase())
    }
}
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for LevelFilter {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}
impl FromStr for LevelFilter {
    type Err = <level_filters::LevelFilter as FromStr>::Err;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        Ok(Self(FromStr::from_str(s)?))
    }
}

/// An Appender specifies a single event sink.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(tag = "kind", rename_all = "lowercase")
)]
pub enum Appender {
    Null,
    Console,
    File {
        path: String,
    },
    RollingFile {
        path: String,
        #[cfg_attr(feature = "serde", serde(rename = "rolloverPolicy"))]
        policy: Policy,
    },
}

impl Appender {
    pub fn file(path: impl Into<String>) -> Self {
        Self::File { path: path.into() }
    }

    #[must_use]
    pub fn console() -> Self {
        Self::Console
    }
}
impl From<&str> for AppenderId {
    fn from(s: &str) -> Self {
        AppenderId(s.to_string())
    }
}

/// A Policy specifies how a `RollingFile` appender should be rolled.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct Policy {
    pub maximum_file_size: String,
    pub max_size_roll_backups: u32,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pattern: Option<String>,
}

impl Policy {
    /// Takes a string like 10kb and returns the number of bytes as a u64.
    ///
    /// # Examples
    ///
    /// ```text
    /// 10, 10b
    /// 10kb 10kib
    /// 10mb 10mib
    /// 10gb 10gib
    /// 10tb 10tib // please no
    /// ```
    ///
    /// # Errors
    /// If the size is not of the aforementioned form we will fail to parse.
    pub fn calculate_maximum_file_size(size: &str) -> Result<u64> {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        // This is lifted from log4rs. We need to replace this..or something.
        let (number, unit) = match size.find(|c: char| !c.is_ascii_digit()) {
            Some(n) => {
                let mut chars = size.chars();
                let (first, rest) = (
                    chars.by_ref().take(n).collect::<String>(),
                    chars.collect::<String>(),
                );
                (
                    Cow::Owned(first.trim().to_string()),
                    Some(rest.trim().to_string()),
                )
            },
            None => (Cow::Borrowed(size.trim()), None),
        };

        let number = match number.parse::<u64>() {
            Ok(n) => n,
            Err(e) => return Err(e.into()),
        };

        let Some(unit) = unit else { return Ok(number) };

        let bytes_number = if unit.eq_ignore_ascii_case("b") {
            Some(number)
        } else if unit.eq_ignore_ascii_case("kb") || unit.eq_ignore_ascii_case("kib") {
            number.checked_mul(KB)
        } else if unit.eq_ignore_ascii_case("mb") || unit.eq_ignore_ascii_case("mib") {
            number.checked_mul(MB)
        } else if unit.eq_ignore_ascii_case("gb") || unit.eq_ignore_ascii_case("gib") {
            number.checked_mul(GB)
        } else if unit.eq_ignore_ascii_case("tb") || unit.eq_ignore_ascii_case("tib") {
            number.checked_mul(TB)
        } else {
            return Err(Error::UnexpectedUnit(unit));
        };

        match bytes_number {
            Some(n) => Ok(n),
            None => Err(Error::Overflow { number, unit }),
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod test {
    use literally::hset;

    use super::{LevelFilter, Logger};
    use crate::config::Format;

    #[test]
    fn test_format_serde() {
        let lgr = Logger {
            appenders: hset! {},
            level: LevelFilter::OFF,
            format: Format::Normal,
        };
        let lgr_value = dbg!(serde_json::to_value(&lgr).unwrap());
        assert!(lgr_value.get("format").is_none());
        let lgr_parsed: Logger = serde_json::from_value(lgr_value).unwrap();
        assert_eq!(lgr_parsed.format, Format::Normal);

        let lgr = Logger {
            appenders: hset! {},
            level: LevelFilter::OFF,
            format: Format::MessageOnly,
        };
        let lgr_value = dbg!(serde_json::to_value(&lgr).unwrap());
        let fmt = lgr_value.get("format").unwrap().as_str().unwrap();
        assert_eq!(fmt, "messageonly");
        let lgr_parsed: Logger = serde_json::from_value(lgr_value).unwrap();
        assert_eq!(lgr_parsed.format, Format::MessageOnly);

        let lgr = Logger {
            appenders: hset! {},
            level: LevelFilter::OFF,
            format: Format::Custom("foobar".to_string()),
        };
        let lgr_value = dbg!(serde_json::to_value(&lgr).unwrap());
        let fmt = lgr_value.get("format").unwrap().as_str().unwrap();
        assert_eq!(fmt, "foobar");
        let lgr_parsed: Logger = serde_json::from_value(lgr_value).unwrap();
        assert_eq!(lgr_parsed.format, Format::Custom("foobar".to_string()));
    }
}
