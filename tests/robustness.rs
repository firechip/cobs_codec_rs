// Robustness / smoke-fuzz suite that runs under plain `cargo test` (no nightly).
// It hammers the decoders with arbitrary, mostly-malformed input to prove they
// are *total* (never panic, only Ok/Err) and that the decode variants agree.
// Deeper coverage lives in `fuzz/` (cargo-fuzz).
#![allow(clippy::pedantic, missing_docs)]

use cobs_codec_rs::{cobs, cobsr};

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
    fn byte(&mut self) -> u8 {
        (self.next() & 0xFF) as u8
    }
    // A byte biased toward the values that stress COBS code bytes: 0x00, 0xFF,
    // and small counts, with the occasional fully-random byte.
    fn stress_byte(&mut self) -> u8 {
        match self.next() % 4 {
            0 => 0x00,
            1 => 0xFF,
            2 => (self.next() % 6) as u8,
            _ => self.byte(),
        }
    }
    fn stress_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.stress_byte()).collect()
    }
}

#[test]
fn decode_is_total_on_arbitrary_input() {
    // Arbitrary (mostly invalid) bytes must never panic, and basic-COBS decode
    // must agree with in-place decode on both the accept/reject decision and the
    // decoded bytes.
    let mut rng = Rng::new(0xF0FA_D00D_1111_2222);
    let mut scratch = vec![0u8; 1024];
    for _ in 0..200_000 {
        let len = (rng.next() as usize) % 512;
        let input = rng.stress_bytes(len);

        // COBS/R decode must also never panic.
        let _ = cobsr::decode(&input, &mut scratch);
        // A custom sentinel decode path must never panic either.
        let s = rng.byte();
        let _ = cobs::decode_with_sentinel(&input, &mut scratch, s);

        // Slice decode vs in-place decode must always agree.
        let slice = cobs::decode(&input, &mut scratch);
        let mut buf = input.clone();
        let in_place = cobs::decode_in_place(&mut buf);
        match (slice, in_place) {
            (Ok(m), Ok(n)) => {
                assert_eq!(m, n, "length mismatch on {input:02x?}");
                assert_eq!(&scratch[..m], &buf[..n], "byte mismatch on {input:02x?}");
            }
            (Err(_), Err(_)) => {}
            (a, b) => panic!("slice/in-place disagree on {input:02x?}: {a:?} vs {b:?}"),
        }
    }
}

#[test]
fn encode_decode_round_trips_arbitrary_payloads() {
    // Any payload round-trips through encode -> decode for COBS, COBS/R, and the
    // sentinel variants; the encoding never contains the (sentinel) delimiter.
    let mut rng = Rng::new(0xC0FF_EE00_3333_4444);
    let mut enc = vec![0u8; 8192];
    let mut dec = vec![0u8; 8192];
    for _ in 0..100_000 {
        let len = (rng.next() as usize) % 700;
        let src = rng.stress_bytes(len);

        let n = cobs::encode(&src, &mut enc);
        assert!(!enc[..n].contains(&0), "COBS output contains 0x00");
        let m = cobs::decode(&enc[..n], &mut dec).unwrap();
        assert_eq!(&dec[..m], &src[..]);

        let n = cobsr::encode(&src, &mut enc);
        assert!(!enc[..n].contains(&0), "COBS/R output contains 0x00");
        let m = cobsr::decode(&enc[..n], &mut dec).unwrap();
        assert_eq!(&dec[..m], &src[..]);

        let s = rng.byte();
        let n = cobs::encode_with_sentinel(&src, &mut enc, s);
        if s != 0 {
            assert!(!enc[..n].contains(&s), "sentinel byte {s:#04x} in output");
        }
        let m = cobs::decode_with_sentinel(&enc[..n], &mut dec, s).unwrap();
        assert_eq!(&dec[..m], &src[..]);
    }
}
