#![no_main]
//! Any payload must round-trip through encode -> decode for both COBS and
//! COBS/R, and the encoding must never contain a `0x00` byte.

use cobs_codec_rs::{cobs, cobsr};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let encoded = cobs::encode_to_vec(data);
    assert!(!encoded.contains(&0), "COBS output contains 0x00");
    assert_eq!(cobs::decode_to_vec(&encoded).unwrap(), data);

    let encoded = cobsr::encode_to_vec(data);
    assert!(!encoded.contains(&0), "COBS/R output contains 0x00");
    assert_eq!(cobsr::decode_to_vec(&encoded).unwrap(), data);
});
