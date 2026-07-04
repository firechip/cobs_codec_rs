// Tests for the sentinel, in-place, and no-alloc streaming features. Differential
// against the slice codecs is the strongest check, so most of this is randomized.
#![allow(clippy::pedantic, missing_docs)]

use cobs_codec_rs::framing::StreamDecoder;
use cobs_codec_rs::{cobs, cobsr, max_encoded_len, DecodeError};

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
    fn len(&mut self, max: usize) -> usize {
        (self.next() as usize) % (max + 1)
    }
}

fn random_packet(rng: &mut Rng, max: usize) -> Vec<u8> {
    let n = rng.len(max);
    (0..n).map(|_| rng.byte()).collect()
}

const SENTINELS: [u8; 6] = [0x00, 0x01, 0x2A, 0x7F, 0xAA, 0xFF];

#[test]
fn sentinel_encoding_avoids_the_sentinel_and_round_trips() {
    let mut rng = Rng::new(0x5E17_0000_1111_2222);
    for &s in &SENTINELS {
        for _ in 0..2000 {
            let packet = random_packet(&mut rng, 600);

            // COBS.
            let enc = cobs::encode_to_vec_with_sentinel(&packet, s);
            assert!(
                !enc.contains(&s),
                "COBS output must not contain sentinel {s:#04x}"
            );
            assert_eq!(cobs::decode_to_vec_with_sentinel(&enc, s).unwrap(), packet);

            // COBS/R.
            let encr = cobsr::encode_to_vec_with_sentinel(&packet, s);
            assert!(
                !encr.contains(&s),
                "COBS/R output must not contain sentinel {s:#04x}"
            );
            assert_eq!(
                cobsr::decode_to_vec_with_sentinel(&encr, s).unwrap(),
                packet
            );
        }
    }
}

#[test]
fn sentinel_zero_matches_plain_codecs() {
    let mut rng = Rng::new(0x0000_5E17_3333_4444);
    for _ in 0..2000 {
        let packet = random_packet(&mut rng, 600);
        assert_eq!(
            cobs::encode_to_vec_with_sentinel(&packet, 0),
            cobs::encode_to_vec(&packet)
        );
        assert_eq!(
            cobsr::encode_to_vec_with_sentinel(&packet, 0),
            cobsr::encode_to_vec(&packet)
        );
    }
}

#[test]
fn decode_in_place_matches_slice_decode() {
    let mut rng = Rng::new(0x1234_A17A_C0DE_0001);
    for &s in &SENTINELS {
        for _ in 0..4000 {
            let packet = random_packet(&mut rng, 700);
            let encoded = cobs::encode_to_vec_with_sentinel(&packet, s);

            let expected = cobs::decode_to_vec_with_sentinel(&encoded, s).unwrap();
            let mut buf = encoded.clone();
            let n = cobs::decode_in_place_with_sentinel(&mut buf, s).unwrap();
            assert_eq!(&buf[..n], &expected[..]);
            assert_eq!(&buf[..n], &packet[..]);
        }
    }
}

/// Frame a list of packets onto one wire buffer (encoding + trailing delimiter).
fn frame_all(packets: &[Vec<u8>], reduced: bool, sentinel: u8) -> Vec<u8> {
    let mut wire = Vec::new();
    for p in packets {
        let enc = if reduced {
            cobsr::encode_to_vec_with_sentinel(p, sentinel)
        } else {
            cobs::encode_to_vec_with_sentinel(p, sentinel)
        };
        wire.extend_from_slice(&enc);
        wire.push(sentinel);
    }
    wire
}

fn stream_decode(wire: &[u8], scratch_len: usize, reduced: bool, sentinel: u8) -> Vec<Vec<u8>> {
    let mut scratch = vec![0u8; scratch_len];
    let mut dec = StreamDecoder::new(&mut scratch)
        .reduced(reduced)
        .sentinel(sentinel);
    let mut out = Vec::new();
    dec.push(wire, |frame| out.push(frame.unwrap().to_vec()));
    out
}

#[test]
fn stream_decoder_reassembles_frames() {
    let mut rng = Rng::new(0x57EA_9000_ABCD_1234);
    for &s in &SENTINELS {
        for &reduced in &[false, true] {
            for _ in 0..300 {
                let count = 1 + rng.len(6);
                let packets: Vec<Vec<u8>> =
                    (0..count).map(|_| random_packet(&mut rng, 300)).collect();
                let scratch = packets.iter().map(Vec::len).max().unwrap_or(0) + 1;
                let wire = frame_all(&packets, reduced, s);

                // Whole-buffer push.
                assert_eq!(stream_decode(&wire, scratch, reduced, s), packets);

                // Byte-at-a-time push must reassemble identically across chunks.
                let mut buf = vec![0u8; scratch];
                let mut dec = StreamDecoder::new(&mut buf).reduced(reduced).sentinel(s);
                let mut got = Vec::new();
                for &b in &wire {
                    dec.push(&[b], |frame| got.push(frame.unwrap().to_vec()));
                }
                assert_eq!(got, packets);
            }
        }
    }
}

#[test]
fn stream_decoder_reports_truncated_frame() {
    // A delimiter arriving mid-block is a truncated basic-COBS frame: the code
    // claims 4 data bytes but only 3 arrive before the delimiter.
    let encoded = cobs::encode_to_vec(&[0x11, 0x22, 0x33, 0x44]); // [0x05, 0x11..0x44]
    let mut wire = encoded[..encoded.len() - 1].to_vec(); // drop one data byte
    wire.push(0x00); // ...then delimit

    let mut scratch = [0u8; 16];
    let mut dec = StreamDecoder::new(&mut scratch);
    let mut errs = Vec::new();
    dec.push(&wire, |f| {
        if let Err(e) = f {
            errs.push(e);
        }
    });
    assert!(matches!(errs.as_slice(), [DecodeError::Truncated { .. }]));

    // In COBS/R mode the same bytes are a valid reduced frame, not an error.
    let mut scratch2 = [0u8; 16];
    let mut dec2 = StreamDecoder::new(&mut scratch2).reduced(true);
    let mut ok = 0;
    dec2.push(&wire, |f| {
        if f.is_ok() {
            ok += 1;
        }
    });
    assert_eq!(ok, 1);
}

#[test]
fn stream_decoder_signals_output_too_small() {
    let packet: Vec<u8> = (0..40u8).collect();
    let mut wire = cobs::encode_to_vec(&packet);
    wire.push(0x00);

    let mut scratch = [0u8; 8]; // too small for a 40-byte packet
    let mut dec = StreamDecoder::new(&mut scratch);
    let mut errs = Vec::new();
    dec.push(&wire, |f| {
        if let Err(e) = f {
            errs.push(e);
        }
    });
    assert!(errs
        .iter()
        .any(|e| matches!(e, DecodeError::OutputTooSmall)));
}

#[test]
fn stream_decoder_emits_empty_frames() {
    // Two consecutive delimiters => one empty frame between them.
    let mut scratch = [0u8; 8];
    let mut dec = StreamDecoder::new(&mut scratch);
    let mut lens = Vec::new();
    dec.push(&[0x00, 0x00], |f| lens.push(f.unwrap().len()));
    assert_eq!(lens, vec![0, 0]);
}

#[test]
fn size_helper_bounds_the_sentinel_encoding() {
    let mut rng = Rng::new(0xABCD_0000_FFFF_1111);
    for _ in 0..1000 {
        let packet = random_packet(&mut rng, 800);
        let enc = cobs::encode_to_vec_with_sentinel(&packet, 0xAA);
        assert!(enc.len() <= max_encoded_len(packet.len()));
    }
}
