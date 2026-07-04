//! Error type shared by the COBS and COBS/R decoders.

use core::fmt;

/// An error returned when decoding fails.
///
/// `serde::{Serialize, Deserialize}` and `defmt::Format` implementations are
/// available behind the optional `serde` and `defmt` features, respectively;
/// both are off by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum DecodeError {
    /// A zero (`0x00`) byte appeared in the encoded input; a valid COBS stream
    /// never contains one.
    ZeroByte {
        /// Index of the offending zero byte within the input.
        index: usize,
    },
    /// A length code claimed more bytes than remain in the input (basic COBS
    /// only; COBS/R interprets that situation as its reduced final block).
    Truncated {
        /// Index of the offending length code within the input.
        index: usize,
    },
    /// The destination buffer was too small to hold the decoded output.
    ///
    /// Provide a buffer of at least `src.len()` bytes.
    OutputTooSmall,
    /// A streaming frame decoder buffered more than its configured maximum frame
    /// length without seeing a delimiter.
    FrameTooLong {
        /// The number of buffered bytes when the limit was exceeded.
        len: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroByte { index } => {
                write!(f, "zero byte in encoded input at index {index}")
            }
            Self::Truncated { index } => {
                write!(f, "length code at index {index} points past end of input")
            }
            Self::OutputTooSmall => f.write_str("destination buffer too small"),
            Self::FrameTooLong { len } => {
                write!(f, "unterminated frame exceeds maximum length ({len} bytes)")
            }
        }
    }
}

impl core::error::Error for DecodeError {}
