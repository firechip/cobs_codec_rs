//! Consistent Overhead Byte Stuffing, Reduced (COBS/R).
//!
//! COBS/R is identical to basic [`crate::cobs`] except that, when the final data
//! byte's value is greater than or equal to the final length code, that byte is
//! used as the length code and dropped from the tail, saving one byte. This
//! often avoids the `+1` byte that basic COBS always adds, which is valuable for
//! small messages. The output is never larger than the basic-COBS encoding.

use crate::DecodeError;
use core::cmp::Ordering;

#[cfg(feature = "alloc")]
use crate::max_encoded_len;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Encodes `src` with COBS/R into `dst`, returning the number of bytes written.
///
/// The output never contains a `0x00` byte and is never larger than the basic
/// COBS encoding. The empty input encodes to `[0x01]`.
///
/// # Panics
///
/// Panics if `dst` is shorter than [`max_encoded_len`]`(src.len())`.
#[must_use]
pub fn encode(src: &[u8], dst: &mut [u8]) -> usize {
    let src_len = src.len();
    let mut code_index = 0;
    let mut write_index = 1;
    let mut code: u8 = 1;
    let mut last_byte: u8 = 0;

    if src_len != 0 {
        let mut read_index = 0;
        loop {
            let byte = src[read_index];
            read_index += 1;
            last_byte = byte;
            if byte == 0 {
                dst[code_index] = code;
                code_index = write_index;
                write_index += 1;
                code = 1;
                if read_index >= src_len {
                    break;
                }
            } else {
                dst[write_index] = byte;
                write_index += 1;
                code += 1;
                if read_index >= src_len {
                    break;
                }
                if code == 0xFF {
                    dst[code_index] = code;
                    code_index = write_index;
                    write_index += 1;
                    code = 1;
                }
            }
        }
    }

    // Reduction: if the final data byte's value is >= the length code basic COBS
    // would write, use that byte as the length code and drop it from the tail.
    if last_byte < code {
        dst[code_index] = code;
    } else {
        dst[code_index] = last_byte;
        write_index -= 1;
    }

    write_index
}

/// Decodes COBS/R `src` into `dst`, returning the number of bytes written.
///
/// The empty input decodes to nothing (returns `0`).
///
/// # Errors
///
/// Returns [`DecodeError::ZeroByte`] if `src` contains a `0x00` byte, or
/// [`DecodeError::OutputTooSmall`] if `dst` is shorter than the decoded output
/// (which never exceeds `src.len()`). Unlike basic COBS, a length code that
/// points past the end of the input is not an error: it signals the reduced
/// final block.
pub fn decode(src: &[u8], dst: &mut [u8]) -> Result<usize, DecodeError> {
    let src_len = src.len();
    if src_len == 0 {
        return Ok(0);
    }

    let mut write_index = 0;
    let mut index = 0;

    loop {
        let code = src[index];
        if code == 0 {
            return Err(DecodeError::ZeroByte { index });
        }
        index += 1;
        let block_end = index + usize::from(code) - 1;
        let copy_end = block_end.min(src_len);
        while index < copy_end {
            let byte = src[index];
            if byte == 0 {
                return Err(DecodeError::ZeroByte { index });
            }
            *dst.get_mut(write_index)
                .ok_or(DecodeError::OutputTooSmall)? = byte;
            write_index += 1;
            index += 1;
        }
        match block_end.cmp(&src_len) {
            Ordering::Greater => {
                // Reduced encoding: the length code was really the final data byte.
                *dst.get_mut(write_index)
                    .ok_or(DecodeError::OutputTooSmall)? = code;
                write_index += 1;
                break;
            }
            Ordering::Less => {
                if code < 0xFF {
                    *dst.get_mut(write_index)
                        .ok_or(DecodeError::OutputTooSmall)? = 0;
                    write_index += 1;
                }
            }
            Ordering::Equal => break,
        }
    }

    Ok(write_index)
}

/// Encodes `src` with COBS/R, returning a newly allocated [`Vec`].
#[cfg(feature = "alloc")]
#[must_use]
pub fn encode_to_vec(src: &[u8]) -> Vec<u8> {
    let mut dst = alloc::vec![0u8; max_encoded_len(src.len())];
    let n = encode(src, &mut dst);
    dst.truncate(n);
    dst
}

/// Decodes COBS/R `src`, returning a newly allocated [`Vec`].
///
/// # Errors
///
/// Returns a [`DecodeError`] if `src` is not valid COBS/R.
#[cfg(feature = "alloc")]
pub fn decode_to_vec(src: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut dst = alloc::vec![0u8; src.len()];
    let n = decode(src, &mut dst)?;
    dst.truncate(n);
    Ok(dst)
}
