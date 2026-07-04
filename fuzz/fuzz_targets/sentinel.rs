#![no_main]
//! The configurable-sentinel codec must round-trip any payload, and the
//! encoding must never contain the chosen sentinel byte.

use cobs_codec_rs::cobs;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Use the first byte as the sentinel, the rest as the payload.
    let Some((&sentinel, payload)) = data.split_first() else {
        return;
    };

    let encoded = cobs::encode_to_vec_with_sentinel(payload, sentinel);
    if sentinel != 0 {
        assert!(!encoded.contains(&sentinel), "sentinel byte in output");
    }
    assert_eq!(
        cobs::decode_to_vec_with_sentinel(&encoded, sentinel).unwrap(),
        payload,
    );
});
