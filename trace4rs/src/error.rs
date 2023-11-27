use std::io;

use camino::Utf8PathBuf;

/// A `trace4rs` Result.
pub type Result<T> = std::result::Result<T, Error>;

/// An enum representing the possible errors encountered.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to correct the output path at '{0}', perhaps it is un-writeable: {1}")]
    PathCorrectionFail(Utf8PathBuf, #[source] io::Error),

    #[error("Failed to flush appender for '{0}': {1}")]
    FlushFail(Utf8PathBuf, #[source] io::Error),

    #[error("error setting the global default logger: {0}")]
    SetGlobalDefaultError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error("error reloading logger: {0}")]
    Reload(#[from] tracing_subscriber::reload::Error),

    #[error("Failed to create file at '{path}': {source}")]
    CreateFailed {
        path:   Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Failed to get metadata for '{path}': {source}")]
    MetadataFailed {
        path:   Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Failed to absolutize input path")]
    AbsolutizeFailed(#[from] io::Error),

    #[error("Error in the config: {0}")]
    Config(#[from] trace4rs_config::error::Error),
}
