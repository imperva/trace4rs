use core::num::ParseIntError;
use std::result;

/// A `trace4rs_config` Result.
pub type Result<T> = result::Result<T, Error>;

/// An enum representing the possible errors encountered.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("While parsing policy size limit an unexpected unit was encountered: {0}")]
    UnexpectedUnit(String),

    #[error("Policy size overflow (byte size does not fit in u64): {number} {unit}")]
    Overflow { number: u64, unit: String },

    #[error("Failed to parse as an int from the config: {0}")]
    ParseIntError(#[from] ParseIntError),
}
