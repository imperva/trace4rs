use std::{
    cmp,
    fs,
    io::{
        self,
        LineWriter,
        Write,
    },
};

use camino::{
    Utf8Path,
    Utf8PathBuf,
};

use crate::{
    env::try_expand_env_vars,
    error::{
        Error,
        Result,
    },
};

/// `LogFileMeta` allows us to keep track of an estimated length for a given
/// file.
#[derive(Clone, Default, Debug)]
pub struct LogFileMeta {
    est_len: u64,
}
impl LogFileMeta {
    pub fn from_meta(meta: &fs::Metadata) -> Self {
        Self {
            est_len: meta.len(),
        }
    }

    pub fn try_from_file(file: &fs::File) -> io::Result<Self> {
        Ok(Self::from_meta(&file.metadata()?))
    }

    fn wrote(&mut self, bs_count: usize) {
        self.est_len = self.est_len.saturating_add(bs_count as u64);
    }

    fn len_estimate(&self) -> u64 {
        self.est_len
    }
}

/// A Trigger which specifies when to roll a file.
#[derive(Clone, Debug)]
pub enum Trigger {
    Size { limit: u64 },
}
impl Trigger {
    /// Has the trigger been met.
    fn should_roll(&self, meta: &LogFileMeta) -> bool {
        matches!(self, Self::Size{limit} if *limit < meta.len_estimate())
    }
}
/// Ex. If count is `3` and pattern is 'log/foo.{}' we'll have the following
/// layout
/// ```text
/// /log
///   - foo.0 # the latest rolled log file
///   - foo.1
///   - foo.2 # the oldest rolled log file
/// ```
#[derive(Clone, Debug)]
pub struct FixedWindow {
    /// invariant last < count
    last:    Option<usize>,
    count:   usize,
    pattern: String,
}
impl FixedWindow {
    const COUNT_BASE: usize = 0;
    pub(crate) const INDEX_TOKEN: &'static str = "{}";

    /// Increment the last rolled file (highest index)
    fn inc_last(&mut self) -> usize {
        match &mut self.last {
            None => {
                self.last.replace(Self::COUNT_BASE);
                Self::COUNT_BASE
            },
            // invariant: current < count
            Some(x) if (x.saturating_add(1)) == self.count => *x,
            Some(x) => {
                *x = x.saturating_add(1);
                *x
            },
        }
    }

    // eas: Idk why im so dumb but this function is _bad_.
    fn roll(&mut self, path: &Utf8Path) -> io::Result<()> {
        // if None, we just need to roll to zero, which happens after this block

        'outer: {
            if let Some(mut c) = self.last {
                // holding max rolls, saturation should be fine
                if c.saturating_add(1) == self.count {
                    if c == 0 {
                        break 'outer;
                    }
                    // We skip the last file if we're at the max so it'll get overwritten.
                    c = c.saturating_sub(1);
                }

                while c > cmp::max(0, Self::COUNT_BASE) {
                    Self::pattern_roll(&self.pattern, c, c.saturating_add(1))?;
                    c = c.saturating_sub(1);
                }
                // current == COUNT_BASE
                Self::pattern_roll(&self.pattern, c, c.saturating_add(1))?;
            }
        }
        self.inc_last();

        let new_path = self
            .pattern
            .replace(Self::INDEX_TOKEN, &Self::COUNT_BASE.to_string());

        fs::rename(path, new_path)
    }

    /// Roll from for example `./foo.0` to `./foo.1`
    fn pattern_roll(pattern: &str, from: usize, to: usize) -> io::Result<()> {
        fs::rename(
            pattern.replace(Self::INDEX_TOKEN, &from.to_string()),
            pattern.replace(Self::INDEX_TOKEN, &to.to_string()),
        )
    }
}

/// Roller specifies how to roll a file.
#[derive(Clone, Debug)]
pub enum Roller {
    Delete,
    FixedWindow(FixedWindow),
}
impl Roller {
    /// Construct a new fixed window roller.
    pub fn new_fixed(pattern: String, count: usize) -> Self {
        Self::FixedWindow(FixedWindow {
            last: None,
            pattern,
            count,
        })
    }

    /// Perform the roll.
    pub fn roll(
        &mut self,
        path: &Utf8Path,
        writer: &mut Option<LineWriter<fs::File>>,
    ) -> io::Result<()> {
        if let Some(w) = writer {
            w.flush()?;
        }
        writer.take();
        match self {
            Self::FixedWindow(x) => {
                x.roll(path)?;
            },
            Self::Delete => fs::remove_file(path)?,
        }
        writer.replace(Rolling::new_writer(path)?);
        Ok(())
    }
}

/// An appender which writes to a file and manages rolling said file, either to
/// backups or by deletion.
#[derive(Debug)]
pub struct Rolling {
    path:    Utf8PathBuf,
    /// Writer will always be some except when it is being rolled or if there
    /// was an error initing a new writer after abandonment of the previous.
    writer:  Option<LineWriter<fs::File>>,
    meta:    LogFileMeta,
    trigger: Trigger,
    roller:  Roller,
}
impl Rolling {
    const DEFAULT_FILE_NAME: &'static str = "log";
    const DEFAULT_ROLL_PATTERN: &'static str = "{filename}.{}";
    const FILE_NAME_TOKEN: &'static str = "{filename}";

    /// Create a new `RollingFile`
    ///
    /// `RollingFile` accepts a path that will be the target of logs. Env
    /// variables are expanded with the following syntax:
    /// `$ENV{var_name}`.
    ///
    /// Note: If the variable fails to resolve, `$ENV{var_name}` will NOT
    /// be replaced in the path.
    pub fn new(p: impl AsRef<Utf8Path>, trigger: Trigger, roller: Roller) -> Result<Self> {
        let expanded_path = try_expand_env_vars(p.as_ref());
        let (writer, meta) = {
            let writer = Self::new_writer(&expanded_path).map_err(|e| Error::CreateFailed {
                path:   expanded_path.clone().into_owned(),
                source: e,
            })?;
            let meta = writer
                .get_ref()
                .metadata()
                .map_err(|e| Error::MetadataFailed {
                    path:   expanded_path.clone().into_owned(),
                    source: e,
                })?;
            (writer, LogFileMeta::from_meta(&meta))
        };

        Ok(Self {
            path: expanded_path.into_owned(),
            writer: Some(writer),
            meta,
            trigger,
            roller,
        })
    }

    /// Verify that the currently open file is still at the original path.
    pub fn correct_path(&mut self) -> io::Result<()> {
        let correct = fs::metadata(&self.path);
        let existing = self.writer.as_ref().map(|w| w.get_ref().metadata());

        if needs_remount(existing, correct) {
            self.remount()?;
        }
        Ok(())
    }

    /// Get the target path as a string.
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
        self.writer
            .as_mut()
            .map(std::io::Write::flush)
            .transpose()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let p = self.path.clone();
        self.swap_writer(|| Self::new_writer(&p))?;
        self.meta = self
            .writer
            .as_mut()
            .map(|w| LogFileMeta::try_from_file(w.get_ref()))
            .transpose()?
            .unwrap_or_default();

        Ok(())
    }

    /// Takes the `path` of the active log file and makes the roll path pattern
    /// with the specified `pat_opt` or if `None` specified, the default
    /// pattern: "{filename}.{}".
    ///
    /// ```ignore
    /// 
    /// make_qualified_pattern(Path::from("./foo/bar.log"), None); // -> "./foo/bar.log.{}"
    /// make_qualified_pattern(Path::from("./foo/bar.log"), Some("bar_roll.{}")); // -> "./foo/bar_roll.{}"
    /// ```
    pub(crate) fn make_qualified_pattern(path: &Utf8Path, pat_opt: Option<&str>) -> String {
        let parent = path.parent().unwrap_or_else(|| Utf8Path::new("/"));
        if let Some(pat) = pat_opt {
            parent.join(pat).to_string()
        } else {
            let file_name = path.file_name().unwrap_or(Self::DEFAULT_FILE_NAME);

            let file_name_pattern =
                Self::DEFAULT_ROLL_PATTERN.replacen(Self::FILE_NAME_TOKEN, file_name, 1);

            parent.join(file_name_pattern).to_string()
        }
    }

    fn swap_writer(&mut self, f: impl Fn() -> io::Result<LineWriter<fs::File>>) -> io::Result<()> {
        self.writer.take();
        self.writer = Some(f()?);
        Ok(())
    }

    fn new_writer(path: &Utf8Path) -> io::Result<LineWriter<fs::File>> {
        let f = fs::File::options().append(true).create(true).open(path)?;

        Ok(LineWriter::new(f))
    }

    fn maybe_roll(&mut self) -> io::Result<()> {
        if self.trigger.should_roll(&self.meta) {
            self.roller.roll(&self.path, &mut self.writer)?;
            self.meta = self
                .writer
                .as_mut()
                .map(|w| LogFileMeta::try_from_file(w.get_ref()))
                .transpose()?
                .unwrap_or_default();
        }
        Ok(())
    }
}

impl io::Write for Rolling {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(w) = &mut self.writer {
            let bs_written = w.write(buf)?;
            self.meta.wrote(bs_written);
            self.maybe_roll()?;
            Ok(bs_written)
        } else {
            Ok(0)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer
            .as_mut()
            .map(std::io::Write::flush)
            .transpose()?;
        Ok(())
    }
}
pub(crate) fn needs_remount(
    existing: Option<io::Result<fs::Metadata>>,
    correct: io::Result<fs::Metadata>,
) -> bool {
    match (existing, correct) {
        // existing file get metadata success _or_ not provided while correct file path unsuccessful
        (Some(Ok(_)) | None, Err(_)) => true,
        (Some(Ok(e)), Ok(c)) => needs_remount_inner(&e, &c),
        _ => false,
    }
}

fn needs_remount_inner(existing: &fs::Metadata, correct: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        existing.dev() != correct.dev() || existing.ino() != correct.ino()
    }
    #[cfg(windows)]
    {
        // we can only really approximate file comparison to identify if the existing
        // file is the one we are writing to
        use std::os::windows::fs::MetadataExt;
        existing.file_size() != correct.file_size()
            || existing.creation_time() != correct.creation_time()
            || existing.last_write_time() != correct.last_write_time()
    }
    #[cfg(not(any(unix, windows)))]
    // unsupported
    {
        false
    }
}
