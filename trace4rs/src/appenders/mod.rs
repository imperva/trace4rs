#![allow(clippy::single_char_lifetime_names)]
use std::{
    collections::HashMap,
    convert::TryFrom,
    fs::{self,},
    io::{
        self,
        LineWriter,
        Write,
    },
    ops::Deref,
    path::Path,
    sync::Arc,
};

use camino::{
    Utf8Path,
    Utf8PathBuf,
};
use parking_lot::Mutex;
use path_absolutize::Absolutize;
use tracing_subscriber::fmt::MakeWriter;

use crate::{
    config::{
        self,
        AppenderId,
        Policy,
    },
    env::try_expand_env_vars,
    error::{
        Error,
        Result,
    },
};

mod rolling;
use rolling::RollingFile;

#[cfg(test)]
mod test;

/// Shorthand for the Map of `AppenderId` to Appender.
type AppenderMap = HashMap<AppenderId, Appender>;

/// Appenders holds the global map of appenders which can be referenced by
/// Layers, it may be cheaply cloned.
#[derive(Clone)]
pub struct Appenders {
    appenders: Arc<AppenderMap>,
}

impl<'a> IntoIterator for &'a Appenders {
    type IntoIter = std::collections::hash_map::Values<'a, AppenderId, Appender>;
    type Item = &'a Appender;

    fn into_iter(self) -> Self::IntoIter {
        self.appenders.as_ref().values()
    }
}

impl Appenders {
    pub fn new(m: AppenderMap) -> Self {
        Self {
            appenders: Arc::new(m),
        }
    }

    pub fn correct_paths(&self) -> Result<()> {
        for a in self {
            a.correct_path()?;
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        for a in self {
            Appender::flush_io(a)?;
        }
        Ok(())
    }
}
impl Deref for Appenders {
    type Target = AppenderMap;

    fn deref(&self) -> &Self::Target {
        &*self.appenders
    }
}
impl TryFrom<&HashMap<AppenderId, config::Appender>> for Appenders {
    type Error = Error;

    fn try_from(m: &HashMap<AppenderId, config::Appender>) -> Result<Self> {
        let mut out = HashMap::new();
        for (k, v) in m {
            out.insert(k.clone(), v.try_into()?);
        }
        Ok(Self::new(out))
    }
}
impl TryFrom<&config::Appender> for Appender {
    type Error = Error;

    fn try_from(value: &config::Appender) -> Result<Self> {
        match value {
            config::Appender::Null => Ok(crate::Appender::Null),
            config::Appender::Console { .. } => Ok(crate::Appender::new_console()),
            config::Appender::File { path, .. } => crate::Appender::new_file(path),
            config::Appender::RollingFile {
                path,
                policy:
                    Policy {
                        max_size_roll_backups,
                        maximum_file_size,
                        pattern,
                    },
                ..
            } => Appender::new_rolling(
                path,
                pattern.as_deref(),
                *max_size_roll_backups as usize,
                maximum_file_size,
            ),
        }
    }
}

/// An Appender represents a sink where logs can be written.
#[derive(Clone)]
pub enum Appender {
    /// Logs are written to stdout.
    Console(Console),
    /// A file appender.
    File(Arc<Mutex<File>>),
    /// A file appender which rolls files.
    RollingFile(Arc<Mutex<RollingFile>>),
    /// Logs are ignored
    Null,
}
impl Appender {
    /// Construct a new null appender.
    #[must_use]
    pub fn new_null() -> Self {
        Self::Null
    }

    /// Construct a new console appender.
    #[must_use]
    pub fn new_console() -> Self {
        Self::Console(Console::new())
    }

    /// Construct a new file appender.
    ///
    /// # Errors
    /// - We may fail to open the file for write.
    pub fn new_file(p: impl AsRef<Utf8Path>) -> Result<Self> {
        Ok(Self::File(Arc::new(Mutex::new(File::new(p)?))))
    }

    /// Construct a new rolling file appender.
    ///
    /// # Errors
    /// - We may fail to calculate the size limit for the roll trigger.
    /// - We may fail to open the file for write.
    pub fn new_rolling(
        path_str: impl AsRef<str>,
        pattern_opt: Option<&str>,
        count: usize,
        size: &str,
    ) -> Result<Self> {
        use rolling::{
            Roller,
            Trigger,
        };
        let abs_path = {
            let ps = path_str.as_ref();
            let cp = Utf8Path::new(ps);

            let p = Path::new(ps);

            p.absolutize()
                .ok()
                .and_then(|p| Utf8PathBuf::from_path_buf(p.into_owned()).ok())
                .unwrap_or_else(|| cp.to_path_buf())
                .to_path_buf()
        };
        let pattern = RollingFile::make_qualified_pattern(&abs_path, pattern_opt);

        let trigger = Trigger::Size {
            limit: config::Policy::calculate_maximum_file_size(size)?,
        };
        let roller = if count == 0 {
            Roller::Delete
        } else {
            Roller::new_fixed(pattern, count)
        };
        Ok(Self::RollingFile(Arc::new(Mutex::new(RollingFile::new(
            abs_path, trigger, roller,
        )?))))
    }

    /// Correct the appender file path to what was originally opened by
    /// abandoning the current file handle and opening a new one.
    ///
    /// # Errors
    /// - We may fail to open the file for write at the given path.
    pub fn correct_path(&self) -> Result<()> {
        match self {
            Self::Null | Self::Console(_) => Ok(()),
            Self::File(x) => {
                let mut inner = x.lock();
                inner
                    .correct_path()
                    .map_err(|e| Error::PathCorrectionFail(inner.get_path_buf()(), e))
            },
            Self::RollingFile(x) => {
                let mut inner = x.lock();
                inner
                    .correct_path()
                    .map_err(|e| Error::PathCorrectionFail(inner.get_path_buf()(), e))
            },
        }
    }

    /// Flush the pending output
    ///
    /// # Errors
    /// - An io error may occur.
    pub fn flush_io(&self) -> Result<()> {
        match self {
            Self::Null | Self::Console(_) => Ok(()),
            Self::File(x) => {
                let mut inner = x.lock();
                inner
                    .flush()
                    .map_err(|e| Error::FlushFail(inner.get_path_buf()(), e))
            },
            Self::RollingFile(x) => {
                let mut inner = x.lock();
                inner
                    .flush()
                    .map_err(|e| Error::FlushFail(inner.get_path_buf()(), e))
            },
        }
    }
}
impl Default for Appender {
    fn default() -> Self {
        Self::Null
    }
}
impl<'a> MakeWriter<'a> for Appender {
    type Writer = Appender;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
impl io::Write for Appender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Console(x) => x.write(buf),
            Self::File(x) => x.deref().lock().write(buf),
            Self::RollingFile(x) => x.deref().lock().write(buf),
            Self::Null => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Console(x) => x.flush(),
            Self::File(x) => x.lock().flush(),
            Self::RollingFile(x) => x.lock().flush(),
            Self::Null => Ok(()),
        }
    }
}

/// An appender which writes to stdout.
#[derive(Clone, Default)]
pub struct Console;
impl Console {
    pub fn new() -> Self {
        Self::default()
    }
}
impl io::Write for Console {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::stdout().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// An appender which writes to a file.
pub struct File {
    path:   Utf8PathBuf,
    writer: LineWriter<fs::File>,
}
impl File {
    /// Create a new File
    ///
    ///
    /// `File` accepts a path that will be the target of logs. Env
    /// variables are expanded with the following syntax:
    /// `$ENV{var_name}`.
    ///
    /// Note: If the variable fails to resolve, `$ENV{var_name}` will NOT
    /// be replaced in the path.
    pub fn new(p: impl AsRef<Utf8Path>) -> Result<Self> {
        let expanded_path = try_expand_env_vars(p.as_ref());

        // if there is no parent then we're at the root... which already exists
        if let Some(parent) = expanded_path.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::CreateFailed {
                path: parent.to_owned(),
                source,
            })?;
        }

        let writer = Self::new_writer(&expanded_path).map_err(|source| Error::CreateFailed {
            path: expanded_path.clone().into_owned(),
            source,
        })?;

        Ok(Self {
            path: expanded_path.into_owned(),
            writer,
        })
    }

    /// Verify that the currently open file is still at the original path.
    pub fn correct_path(&mut self) -> io::Result<()> {
        let correct = fs::metadata(&self.path);
        let existing = self.writer.get_ref().metadata();

        if rolling::needs_remount(Some(existing), correct) {
            self.remount()?;
        }
        Ok(())
    }

    /// Get the target path as a str.
    pub fn path_str(&self) -> &str {
        self.path.as_str()
    }

    /// Get the target path
    pub fn get_path(&self) -> &Utf8Path {
        &self.path
    }

    /// Get the target path buf
    pub fn get_path_buf(&self) -> Utf8PathBuf {
        self.path.to_path_buf()
    }

    /// Remount the file at the specified path.
    /// This is useful when the file has been moved since the fd was originally
    /// mounted.
    fn remount(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.writer = Self::new_writer(&self.path)?;
        Ok(())
    }

    fn new_writer(path: &Utf8Path) -> io::Result<LineWriter<fs::File>> {
        let f = fs::File::options().append(true).create(true).open(path)?;

        Ok(LineWriter::new(f))
    }
}

impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
