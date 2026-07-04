//! Proves the optional `serde` derive on [`DecodeError`] works by round-tripping
//! every variant through JSON.
//!
//! The whole file is gated on the `serde` feature, so without it the test crate
//! is empty. Run it with `cargo test --features serde`.
#![cfg(feature = "serde")]

use cobs_codec_rs::DecodeError;

/// Serializing then deserializing each variant must yield an equal value.
#[test]
fn decode_error_serde_json_round_trips() {
    let cases = [
        DecodeError::ZeroByte { index: 7 },
        DecodeError::Truncated { index: 3 },
        DecodeError::OutputTooSmall,
        DecodeError::FrameTooLong { len: 42 },
    ];

    for original in cases {
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: DecodeError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored, "round-trip mismatch via {json}");
    }
}
