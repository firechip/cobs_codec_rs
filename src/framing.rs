//! Packet framing helpers built on top of COBS.
//!
//! Because COBS-encoded data never contains a zero byte, a single `0x00` byte
//! can delimit encoded packets on a byte stream such as a serial/UART link.
//!
//! [`StreamDecoder`] reassembles frames into a caller-provided buffer without
//! allocating (usable in pure `no_std`); [`FrameDecoder`] (behind the `alloc`
//! feature) does the same into owned [`Vec`]s.

use crate::{cobs, DecodeError, DELIMITER};

#[cfg(feature = "alloc")]
use crate::cobsr;
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

/// A no-allocation streaming decoder that reassembles [`DELIMITER`]-framed COBS
/// packets into a caller-provided buffer, across arbitrarily sized chunks.
///
/// This is the pure-`no_std` counterpart of [`FrameDecoder`]: decoded bytes are
/// written into the `&mut [u8]` handed to [`new`](Self::new) rather than an
/// allocated `Vec`. Feed raw bytes with [`feed`](Self::feed) (one byte) or
/// [`push`](Self::push) (a chunk); a frame completes when the delimiter is seen.
///
/// Configure [`reduced`](Self::reduced) for COBS/R and
/// [`sentinel`](Self::sentinel) for a non-`0x00` delimiter.
///
/// ```
/// use cobs_codec_rs::framing::{frame, StreamDecoder};
///
/// let mut wire = [0u8; 16];
/// let n = frame(&[0x11, 0x00, 0x22], &mut wire); // COBS + trailing delimiter
///
/// let mut scratch = [0u8; 8];
/// let mut decoder = StreamDecoder::new(&mut scratch);
///
/// let mut out = [0u8; 8];
/// let mut out_len = 0;
/// decoder.push(&wire[..n], |packet| {
///     let packet = packet.unwrap();
///     out[..packet.len()].copy_from_slice(packet);
///     out_len = packet.len();
/// });
/// assert_eq!(&out[..out_len], &[0x11, 0x00, 0x22]);
/// ```
#[derive(Debug)]
pub struct StreamDecoder<'a> {
    dst: &'a mut [u8],
    write_index: usize,
    remaining: u8,
    code: u8,
    sentinel: u8,
    reduced: bool,
}

impl<'a> StreamDecoder<'a> {
    /// Creates a decoder that writes decoded packets into `dst`, framed by the
    /// `0x00` [`DELIMITER`] and interpreted as basic COBS.
    ///
    /// `dst` must be large enough for the biggest packet you expect; otherwise
    /// [`feed`](Self::feed) returns [`DecodeError::OutputTooSmall`].
    #[must_use]
    pub fn new(dst: &'a mut [u8]) -> Self {
        Self {
            dst,
            write_index: 0,
            remaining: 0,
            code: 0,
            sentinel: DELIMITER,
            reduced: false,
        }
    }

    /// Decode frames as COBS/R instead of basic COBS.
    #[must_use]
    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }

    /// Use `sentinel` as the frame delimiter (and the byte the encoding avoids)
    /// instead of `0x00`. Must match the sentinel used to encode.
    #[must_use]
    pub fn sentinel(mut self, sentinel: u8) -> Self {
        self.sentinel = sentinel;
        self
    }

    /// Feeds a single raw byte. Returns `Ok(Some(len))` when a frame completes
    /// (its decoded bytes are then in `self.decoded()` / the first `len` bytes
    /// of the buffer, valid until the next byte is fed), or `Ok(None)` while a
    /// frame is still in progress.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::Truncated`] if a delimiter arrives mid-block in
    /// basic-COBS mode, or [`DecodeError::OutputTooSmall`] if the destination
    /// buffer fills up. On error the in-progress frame is discarded and decoding
    /// resumes with the next frame.
    pub fn feed(&mut self, byte: u8) -> Result<Option<usize>, DecodeError> {
        if byte == self.sentinel {
            if self.remaining > 0 {
                if self.reduced {
                    // Reduced final block: the length code was the last data byte.
                    self.put(self.code)?;
                } else {
                    let index = self.write_index;
                    self.reset();
                    return Err(DecodeError::Truncated { index });
                }
            }
            let len = self.write_index;
            self.reset();
            return Ok(Some(len));
        }

        let value = byte ^ self.sentinel;
        if self.remaining == 0 {
            // Start of a new block. Emit the delimiter that separated it from the
            // previous block (basic COBS inserts one after any code < 0xFF).
            if self.code != 0 && self.code < 0xFF {
                self.put(0)?;
            }
            self.code = value;
            self.remaining = value - 1;
        } else {
            self.put(value)?;
            self.remaining -= 1;
        }
        Ok(None)
    }

    /// Feeds a chunk of raw bytes, invoking `on_frame` once per completed frame
    /// (`Ok` with the decoded packet, borrowed from the internal buffer, or
    /// `Err` if a frame failed to decode).
    pub fn push(&mut self, chunk: &[u8], mut on_frame: impl FnMut(Result<&[u8], DecodeError>)) {
        for &byte in chunk {
            match self.feed(byte) {
                Ok(Some(len)) => on_frame(Ok(&self.dst[..len])),
                Ok(None) => {}
                Err(err) => on_frame(Err(err)),
            }
        }
    }

    /// The decoded bytes of the frame currently in progress (or the most recent
    /// one, until the next byte is fed).
    #[must_use]
    pub fn decoded(&self) -> &[u8] {
        &self.dst[..self.write_index]
    }

    /// Discards any partially decoded frame.
    pub fn reset(&mut self) {
        self.write_index = 0;
        self.remaining = 0;
        self.code = 0;
    }

    fn put(&mut self, byte: u8) -> Result<(), DecodeError> {
        *self
            .dst
            .get_mut(self.write_index)
            .ok_or(DecodeError::OutputTooSmall)? = byte;
        self.write_index += 1;
        Ok(())
    }
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
/// completed frame. See [`StreamDecoder`] for an allocation-free alternative.
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
