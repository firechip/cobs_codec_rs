//! Basic Consistent Overhead Byte Stuffing (COBS).

use crate::DecodeError;

#[cfg(feature = "alloc")]
use crate::max_encoded_len;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Encodes `src` with basic COBS into `dst`, returning the number of bytes
/// written.
///
/// The output never contains a `0x00` byte, so a `0x00` may be used to delimit
/// encoded packets on the wire. Encoding never fails: any input is encodable,
/// and the empty input encodes to `[0x01]`.
///
/// # Panics
///
/// Panics if `dst` is shorter than [`max_encoded_len`]`(src.len())`.
#[must_use]
pub fn encode(src: &[u8], dst: &mut [u8]) -> usize {
    if src.is_empty() {
        dst[0] = 0x01;
        return 1;
    }

    let src_len = src.len();
    let mut code_index = 0;
    let mut write_index = 1;
    let mut code: u8 = 1;
    let mut read_index = 0;

    loop {
        let byte = src[read_index];
        read_index += 1;
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
            // Terminate before the 0xFF split so a run of exactly 254 non-zero
            // bytes does not emit a spurious trailing block.
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
    dst[code_index] = code;

    write_index
}

/// Decodes basic-COBS `src` into `dst`, returning the number of bytes written.
///
/// The empty input decodes to nothing (returns `0`). `src` must be a single
/// encoded packet with no surrounding `0x00` delimiter bytes.
///
/// # Errors
///
/// Returns [`DecodeError::ZeroByte`] or [`DecodeError::Truncated`] if `src` is
/// not valid COBS, or [`DecodeError::OutputTooSmall`] if `dst` is shorter than
/// the decoded output (which never exceeds `src.len()`).
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
        if block_end > src_len {
            return Err(DecodeError::Truncated {
                index: block_end - usize::from(code),
            });
        }
        if block_end < src_len {
            if code < 0xFF {
                *dst.get_mut(write_index)
                    .ok_or(DecodeError::OutputTooSmall)? = 0;
                write_index += 1;
            }
        } else {
            break;
        }
    }

    Ok(write_index)
}

/// Encodes `src` with basic COBS, returning a newly allocated [`Vec`].
#[cfg(feature = "alloc")]
#[must_use]
pub fn encode_to_vec(src: &[u8]) -> Vec<u8> {
    let mut dst = alloc::vec![0u8; max_encoded_len(src.len())];
    let n = encode(src, &mut dst);
    dst.truncate(n);
    dst
}

/// Decodes basic-COBS `src`, returning a newly allocated [`Vec`].
///
/// # Errors
///
/// Returns a [`DecodeError`] if `src` is not valid COBS.
#[cfg(feature = "alloc")]
pub fn decode_to_vec(src: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut dst = alloc::vec![0u8; src.len()];
    let n = decode(src, &mut dst)?;
    dst.truncate(n);
    Ok(dst)
}
