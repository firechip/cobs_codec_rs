//! Consistent Overhead Byte Stuffing (COBS) and COBS/R for Rust.
//!
//! COBS encodes an arbitrary byte sequence into one that contains no zero
//! (`0x00`) bytes, at a small and *predictable* cost: at most one extra byte per
//! 254 bytes, plus one. That makes a single `0x00` a reliable packet delimiter
//! for serial/UART, USB, TCP and other byte streams, which is why COBS is
//! popular in embedded and robotics protocols.
//!
//! This crate is `#![no_std]` and dependency-free. The core [`cobs`] and
//! [`cobsr`] `encode`/`decode` functions work on caller-provided slices, with
//! `*_with_sentinel` variants for a non-`0x00` delimiter and
//! [`cobs::decode_in_place`] for zero-copy decoding. The allocation-free
//! [`framing::StreamDecoder`] reassembles delimited frames into a fixed buffer.
//! The `alloc` feature (enabled by default via `std`) adds `*_to_vec`
//! conveniences and the owned-`Vec` [`framing::FrameDecoder`].
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
//! # Encoding overhead
//!
//! COBS has a *tight, data-independent* bound on overhead. Encoding an $n$-byte
//! packet yields at most
//!
//! $$ n + \left\lceil \frac{n}{254} \right\rceil $$
//!
//! bytes (one extra byte per 254, rounded up), so the overhead $o(n)$ obeys
//!
//! $$ 1 \le o(n) \le \left\lceil \frac{n}{254} \right\rceil \quad (n \ge 1) $$
//!
//! and [`max_encoded_len`] and [`encoding_overhead`] return exactly these
//! worst-case bounds. By contrast, escape-based framing (PPP, SLIP, HDLC) can
//! *double* a packet in the worst case, an overhead of up to $n$ bytes, because
//! any byte may need escaping.
//!
//! The bound follows from the block structure: COBS emits *blocks*, each a
//! *code byte* $c$ followed by $c - 1$ non-zero data bytes. A code $c \lt 255$
//! stands for those bytes then an implicit zero; $c = 255$ carries a full run of
//! $254$ non-zero bytes (see [`MAX_BLOCK_LEN`]) with no trailing zero. At most
//! one code byte is spent per 254 data bytes, so an $n$-byte packet needs at
//! most $\left\lceil \frac{n}{254} \right\rceil$ of them. [`cobsr`] (COBS/R)
//! drops the final code byte when the last data byte can stand in for it,
//! reaching a best case of *zero* overhead.
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
/// one byte for any message including the empty one. In closed form this is
/// $\lceil n/254 \rceil$ for a non-empty message of $n$ bytes, and $1$ for the
/// empty message.
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
/// buffer. For a message of $n$ bytes this is $n + \lceil n/254 \rceil$ (and
/// $1$ when $n = 0$).
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
