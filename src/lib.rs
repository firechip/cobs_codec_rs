//! Consistent Overhead Byte Stuffing (COBS) and COBS/R for Rust.
//!
//! COBS encodes an arbitrary byte sequence into one that contains no zero
//! (`0x00`) bytes, at a small and *predictable* cost: at most one extra byte per
//! 254 bytes, plus one. That makes a single `0x00` a reliable packet delimiter
//! for serial/UART, USB, TCP and other byte streams, which is why COBS is
//! popular in embedded and robotics protocols.
//!
//! This crate is `#![no_std]` and dependency-free. The core [`cobs`] and
//! [`cobsr`] `encode`/`decode` functions work on caller-provided slices; the
//! `alloc` feature (enabled by default via `std`) adds `*_to_vec` conveniences
//! and the [`framing::FrameDecoder`].
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "alloc")] {
//! use cobs_codec_rs::cobs;
//!
//! let encoded = cobs::encode_to_vec(&[0x11, 0x22, 0x00, 0x33]);
//! assert_eq!(encoded, [0x03, 0x11, 0x22, 0x02, 0x33]); // no 0x00
//! assert_eq!(cobs::decode_to_vec(&encoded).unwrap(), [0x11, 0x22, 0x00, 0x33]);
//! # }
//! ```
//!
//! # `no_std` usage
//!
//! ```
//! use cobs_codec_rs::{cobs, max_encoded_len};
//!
//! let src = [0x11, 0x00, 0x22];
//! let mut buf = [0u8; 16]; // >= max_encoded_len(src.len())
//! let n = cobs::encode(&src, &mut buf);
//! assert_eq!(&buf[..n], &[0x02, 0x11, 0x02, 0x22]);
//! ```
//!
//! See "Consistent Overhead Byte Stuffing" by Stuart Cheshire and Mary Baker,
//! IEEE/ACM Transactions on Networking, Vol. 7, No. 2, April 1999.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod cobs;
pub mod cobsr;
pub mod framing;

mod error;
pub use error::DecodeError;

/// The byte value used to delimit COBS-encoded frames on the wire.
pub const DELIMITER: u8 = 0x00;

/// The largest number of source bytes a single COBS code block can carry
/// without emitting an overhead byte.
pub const MAX_BLOCK_LEN: usize = 254;

/// Returns the maximum encoding overhead, in bytes, that COBS or COBS/R can add
/// when encoding a message of `source_len` bytes.
///
/// COBS adds at most one byte per 254 bytes of input (rounded up), and at least
/// one byte for any message including the empty one.
///
/// ```
/// use cobs_codec_rs::encoding_overhead;
/// assert_eq!(encoding_overhead(0), 1);
/// assert_eq!(encoding_overhead(254), 1);
/// assert_eq!(encoding_overhead(255), 2);
/// ```
#[must_use]
pub const fn encoding_overhead(source_len: usize) -> usize {
    if source_len == 0 {
        1
    } else {
        1 + (source_len - 1) / MAX_BLOCK_LEN
    }
}

/// Returns the maximum possible length, in bytes, of the COBS (or COBS/R)
/// encoding of a message of `source_len` bytes. Useful for sizing an encode
/// buffer.
///
/// ```
/// use cobs_codec_rs::max_encoded_len;
/// assert_eq!(max_encoded_len(254), 255);
/// assert_eq!(max_encoded_len(255), 257);
/// ```
#[must_use]
pub const fn max_encoded_len(source_len: usize) -> usize {
    source_len + encoding_overhead(source_len)
}
