#![no_main]
//! Decoding arbitrary bytes must never panic, and the slice decoder must agree
//! with the in-place decoder on both the accept/reject decision and the output.

use cobs_codec_rs::cobs;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut out = vec![0u8; data.len() + 1];
    let slice = cobs::decode(data, &mut out);

    let mut buf = data.to_vec();
    let in_place = cobs::decode_in_place(&mut buf);

    match (slice, in_place) {
        (Ok(m), Ok(n)) => {
            assert_eq!(m, n, "length mismatch");
            assert_eq!(&out[..m], &buf[..n], "byte mismatch");
        }
        (Err(_), Err(_)) => {}
        (a, b) => panic!("slice/in-place disagree: {a:?} vs {b:?}"),
    }
});
