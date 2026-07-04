//! Packet framing helpers built on top of COBS.
//!
//! Because COBS-encoded data never contains a zero byte, a single `0x00` byte
//! can delimit encoded packets on a byte stream such as a serial/UART link.

use crate::{cobs, DELIMITER};

#[cfg(feature = "alloc")]
use crate::{cobsr, DecodeError};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Encodes `packet` with basic COBS into `dst` and appends the [`DELIMITER`],
/// returning the total number of bytes written (the encoding plus one).
///
/// # Panics
///
/// Panics if `dst` is shorter than [`crate::max_encoded_len`]`(packet.len()) + 1`.
#[must_use]
pub fn frame(packet: &[u8], dst: &mut [u8]) -> usize {
    let n = cobs::encode(packet, dst);
    dst[n] = DELIMITER;
    n + 1
}

/// Encodes `packet` with basic COBS and appends the [`DELIMITER`], returning a
/// newly allocated [`Vec`].
#[cfg(feature = "alloc")]
#[must_use]
pub fn frame_to_vec(packet: &[u8]) -> Vec<u8> {
    let mut v = cobs::encode_to_vec(packet);
    v.push(DELIMITER);
    v
}

/// Encodes `packet` with COBS/R and appends the [`DELIMITER`], returning a newly
/// allocated [`Vec`].
#[cfg(feature = "alloc")]
#[must_use]
pub fn frame_reduced_to_vec(packet: &[u8]) -> Vec<u8> {
    let mut v = cobsr::encode_to_vec(packet);
    v.push(DELIMITER);
    v
}

/// A streaming decoder that turns a byte stream of [`DELIMITER`]-framed data
/// into decoded packets, buffering across arbitrarily sized chunks.
///
/// This is the natural way to read COBS packets from a serial/UART link. Feed
/// raw bytes with [`push`](Self::push); the callback is invoked once per
/// completed frame.
///
/// ```
/// # #[cfg(feature = "alloc")] {
/// use cobs_codec_rs::framing::{frame_to_vec, FrameDecoder};
///
/// let wire = frame_to_vec(&[0x11, 0x00, 0x22]);
/// let mut rx = FrameDecoder::new();
/// let mut packets = Vec::new();
/// // Feed one byte at a time to prove reassembly works across chunks.
/// for b in &wire {
///     rx.push(&[*b], |frame| packets.push(frame.unwrap()));
/// }
/// assert_eq!(packets, vec![vec![0x11, 0x00, 0x22]]);
/// # }
/// ```
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
    reduced: bool,
    skip_empty: bool,
    max_frame_len: usize,
}

#[cfg(feature = "alloc")]
impl FrameDecoder {
    /// Creates a decoder for basic COBS that skips empty frames.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            reduced: false,
            skip_empty: true,
            max_frame_len: 0,
        }
    }

    /// Decode frames as COBS/R instead of basic COBS.
    #[must_use]
    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }

    /// Whether to skip empty frames (from consecutive or leading delimiters)
    /// rather than emit empty packets. Defaults to `true`.
    #[must_use]
    pub fn skip_empty(mut self, skip_empty: bool) -> Self {
        self.skip_empty = skip_empty;
        self
    }

    /// Bound the number of bytes buffered for a single unterminated frame. When
    /// exceeded, the buffer is discarded and the callback receives
    /// [`DecodeError::FrameTooLong`]. `0` (the default) means unbounded.
    #[must_use]
    pub fn max_frame_len(mut self, max_frame_len: usize) -> Self {
        self.max_frame_len = max_frame_len;
        self
    }

    /// Feeds a chunk of raw bytes, invoking `on_frame` once per completed frame
    /// (`Ok` with the decoded packet, or `Err` if a frame failed to decode).
    pub fn push(&mut self, chunk: &[u8], mut on_frame: impl FnMut(Result<Vec<u8>, DecodeError>)) {
        let mut start = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b != DELIMITER {
                continue;
            }
            self.buf.extend_from_slice(&chunk[start..i]);
            start = i + 1;
            let frame = core::mem::take(&mut self.buf);
            if frame.is_empty() {
                if !self.skip_empty {
                    on_frame(Ok(Vec::new()));
                }
                continue;
            }
            let decoded = if self.reduced {
                cobsr::decode_to_vec(&frame)
            } else {
                cobs::decode_to_vec(&frame)
            };
            on_frame(decoded);
        }
        if start < chunk.len() {
            self.buf.extend_from_slice(&chunk[start..]);
            if self.max_frame_len != 0 && self.buf.len() > self.max_frame_len {
                let len = self.buf.len();
                self.buf.clear();
                on_frame(Err(DecodeError::FrameTooLong { len }));
            }
        }
    }

    /// Discards any buffered partial frame.
    pub fn reset(&mut self) {
        self.buf.clear();
    }
}
