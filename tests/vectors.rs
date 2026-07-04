// Golden vectors and round-trip tests, ported from the reference COBS/COBS-R
// suites. Test-internal casts don't need pedantic scrutiny.
#![allow(clippy::pedantic, missing_docs)]

use cobs_codec_rs::{cobs, cobsr, encoding_overhead, max_encoded_len, DecodeError};

fn ascii(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

fn range(start: u16, end: u16) -> Vec<u8> {
    (start..end).map(|i| i as u8).collect()
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// Deterministic non-zero byte stream, matching the reference test suites.
fn non_zero_bytes(length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(length);
    'outer: loop {
        for i in 1..50u32 {
            let mut j = 1u32;
            while j < 256 {
                if out.len() == length {
                    break 'outer;
                }
                out.push(j as u8);
                j += i;
            }
        }
    }
    out
}

fn simple_encode_non_zeros(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let end = (i + 254).min(input.len());
        out.push((end - i + 1) as u8);
        out.extend_from_slice(&input[i..end]);
        i += 254;
    }
    out
}

/// Tiny deterministic xorshift PRNG (keeps the crate dependency-free).
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xFF) as u8
    }
}

fn cobs_predefined() -> Vec<(Vec<u8>, Vec<u8>)> {
    vec![
        (ascii(""), vec![0x01]),
        (ascii("1"), vec![0x02, 0x31]),
        (ascii("12345"), cat(&[&[0x06], &ascii("12345")])),
        (
            cat(&[&ascii("12345"), &[0], &ascii("6789")]),
            cat(&[&[0x06], &ascii("12345"), &[0x05], &ascii("6789")]),
        ),
        (
            cat(&[&[0], &ascii("12345"), &[0], &ascii("6789")]),
            cat(&[&[0x01, 0x06], &ascii("12345"), &[0x05], &ascii("6789")]),
        ),
        (
            cat(&[&ascii("12345"), &[0], &ascii("6789"), &[0]]),
            cat(&[&[0x06], &ascii("12345"), &[0x05], &ascii("6789"), &[0x01]]),
        ),
        (vec![0], vec![0x01, 0x01]),
        (vec![0, 0], vec![0x01, 0x01, 0x01]),
        (vec![0, 0, 0], vec![0x01, 0x01, 0x01, 0x01]),
        (range(1, 254), cat(&[&[0xFE], &range(1, 254)])),
        (range(1, 255), cat(&[&[0xFF], &range(1, 255)])),
        (
            range(1, 256),
            cat(&[&[0xFF], &range(1, 255), &[0x02, 0xFF]]),
        ),
        (
            range(0, 256),
            cat(&[&[0x01, 0xFF], &range(1, 255), &[0x02, 0xFF]]),
        ),
    ]
}

fn cobsr_predefined() -> Vec<(Vec<u8>, Vec<u8>)> {
    vec![
        (ascii(""), vec![0x01]),
        (vec![0x01], vec![0x02, 0x01]),
        (vec![0x02], vec![0x02]),
        (vec![0x03], vec![0x03]),
        (vec![0x7E], vec![0x7E]),
        (vec![0xFE], vec![0xFE]),
        (vec![0xFF], vec![0xFF]),
        (
            cat(&[&ascii("a"), &[0x02]]),
            cat(&[&[0x03], &ascii("a"), &[0x02]]),
        ),
        (cat(&[&ascii("a"), &[0x03]]), cat(&[&[0x03], &ascii("a")])),
        (cat(&[&ascii("a"), &[0xFF]]), cat(&[&[0xFF], &ascii("a")])),
        (ascii("12345"), cat(&[&[0x35], &ascii("1234")])),
        (
            cat(&[&ascii("12345"), &[0], &ascii("6789")]),
            cat(&[&[0x06], &ascii("12345"), &ascii("9678")]),
        ),
        (vec![0], vec![0x01, 0x01]),
        (range(1, 254), cat(&[&[0xFE], &range(1, 254)])),
        (range(1, 255), cat(&[&[0xFF], &range(1, 255)])),
        (range(1, 256), cat(&[&[0xFF], &range(1, 255), &[0xFF]])),
        (
            range(0, 256),
            cat(&[&[0x01, 0xFF], &range(1, 255), &[0xFF]]),
        ),
        (range(2, 256), cat(&[&[0xFF], &range(2, 255)])),
    ]
}

#[test]
fn cobs_golden_vectors() {
    for (decoded, encoded) in cobs_predefined() {
        assert_eq!(
            cobs::encode_to_vec(&decoded),
            encoded,
            "encoding {decoded:?}"
        );
        assert_eq!(
            cobs::decode_to_vec(&encoded).unwrap(),
            decoded,
            "decoding {encoded:?}"
        );
    }
}

#[test]
fn cobsr_golden_vectors() {
    for (decoded, encoded) in cobsr_predefined() {
        assert_eq!(
            cobsr::encode_to_vec(&decoded),
            encoded,
            "encoding {decoded:?}"
        );
        assert_eq!(
            cobsr::decode_to_vec(&encoded).unwrap(),
            decoded,
            "decoding {encoded:?}"
        );
    }
}

#[test]
fn cobs_decode_errors() {
    assert_eq!(
        cobs::decode_to_vec(&[0x00]),
        Err(DecodeError::ZeroByte { index: 0 })
    );
    assert!(matches!(
        cobs::decode_to_vec(&[0x05, b'1', b'2', b'3']),
        Err(DecodeError::Truncated { .. })
    ));
    assert!(matches!(
        cobs::decode_to_vec(&[0x05, b'1', b'2', 0x00, b'4']),
        Err(DecodeError::ZeroByte { .. })
    ));
}

#[test]
fn zeros_of_every_length() {
    for len in 0..520usize {
        let data = vec![0u8; len];
        let encoded = cobs::encode_to_vec(&data);
        assert_eq!(encoded, vec![0x01u8; len + 1], "cobs {len} zeros");
        assert_eq!(cobs::decode_to_vec(&encoded).unwrap(), data);
        // COBS/R encodes all-zeros identically.
        assert_eq!(cobsr::encode_to_vec(&data), vec![0x01u8; len + 1]);
        assert_eq!(
            cobsr::decode_to_vec(&cobsr::encode_to_vec(&data)).unwrap(),
            data
        );
    }
}

#[test]
fn non_zero_runs() {
    for len in 1..1000usize {
        let data = non_zero_bytes(len);
        assert_eq!(
            cobs::encode_to_vec(&data),
            simple_encode_non_zeros(&data),
            "len {len}"
        );
        assert_eq!(
            cobs::decode_to_vec(&cobs::encode_to_vec(&data)).unwrap(),
            data
        );
        assert_eq!(
            cobsr::decode_to_vec(&cobsr::encode_to_vec(&data)).unwrap(),
            data
        );
    }
}

#[test]
fn random_round_trip() {
    let mut rng = Rng::new(0xC0B5_C0DE_1234_5678);
    for _ in 0..5000 {
        let len = rng.below(2001);
        let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();

        let e = cobs::encode_to_vec(&data);
        assert!(!e.contains(&0));
        assert!(e.len() <= max_encoded_len(len));
        assert_eq!(cobs::decode_to_vec(&e).unwrap(), data);

        let r = cobsr::encode_to_vec(&data);
        assert!(!r.contains(&0));
        assert!(r.len() <= e.len());
        assert_eq!(cobsr::decode_to_vec(&r).unwrap(), data);
    }
}

#[test]
fn size_helpers() {
    assert_eq!(encoding_overhead(0), 1);
    assert_eq!(encoding_overhead(254), 1);
    assert_eq!(encoding_overhead(255), 2);
    assert_eq!(max_encoded_len(0), 1);
    assert_eq!(max_encoded_len(254), 255);
    assert_eq!(max_encoded_len(255), 257);
}
