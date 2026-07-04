#![allow(clippy::pedantic, missing_docs)]

use cobs_codec_rs::framing::{frame, frame_reduced_to_vec, frame_to_vec, FrameDecoder};
use cobs_codec_rs::DELIMITER;

#[test]
fn frame_appends_delimiter() {
    let f = frame_to_vec(&[0x11, 0x00, 0x22]);
    assert_eq!(f, [0x02, 0x11, 0x02, 0x22, 0x00]);
    assert_eq!(*f.last().unwrap(), DELIMITER);
}

#[test]
fn frame_no_std_variant() {
    let mut buf = [0u8; 32];
    let n = frame(&[0x11, 0x00, 0x22], &mut buf);
    assert_eq!(&buf[..n], &[0x02, 0x11, 0x02, 0x22, 0x00]);
}

#[test]
fn stream_decoder_reassembles_byte_at_a_time() {
    let wire = [frame_to_vec(&[0xAA, 0x00, 0xBB]), frame_to_vec(&[0x01])].concat();
    let mut rx = FrameDecoder::new();
    let mut out: Vec<Vec<u8>> = Vec::new();
    for b in &wire {
        rx.push(&[*b], |r| out.push(r.unwrap()));
    }
    assert_eq!(out, vec![vec![0xAA, 0x00, 0xBB], vec![0x01]]);
}

#[test]
fn stream_decoder_skips_empty_frames() {
    let mut wire = vec![DELIMITER];
    wire.extend(frame_to_vec(&[0x42]));
    wire.push(DELIMITER);
    let mut rx = FrameDecoder::new();
    let mut out = Vec::new();
    rx.push(&wire, |r| out.push(r.unwrap()));
    assert_eq!(out, vec![vec![0x42]]);
}

#[test]
fn stream_decoder_reduced() {
    let f = frame_reduced_to_vec(b"12345");
    assert_eq!(f, [0x35, b'1', b'2', b'3', b'4', 0x00]);
    let mut rx = FrameDecoder::new().reduced(true);
    let mut out = Vec::new();
    rx.push(&f, |r| out.push(r.unwrap()));
    assert_eq!(out, vec![b"12345".to_vec()]);
}

#[test]
fn stream_decoder_max_frame_len_guard() {
    let mut rx = FrameDecoder::new().max_frame_len(100);
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut errors = 0;
    rx.push(&[0x01u8; 50], |r| match r {
        Ok(v) => out.push(v),
        Err(_) => errors += 1,
    });
    rx.push(&[0x01u8; 60], |r| match r {
        Ok(v) => out.push(v),
        Err(_) => errors += 1,
    }); // 110 buffered > 100 -> discard
    rx.push(&frame_to_vec(&[0x22]), |r| out.push(r.unwrap()));
    assert_eq!(errors, 1);
    assert_eq!(out, vec![vec![0x22]]);
}

#[test]
fn stream_decoder_reports_bad_frame_and_continues() {
    let mut wire = frame_to_vec(&[0x11]);
    wire.extend_from_slice(&[0x05, 0x01, 0x00]); // invalid frame (length past end)
    wire.extend(frame_to_vec(&[0x22]));
    let mut rx = FrameDecoder::new();
    let mut oks: Vec<Vec<u8>> = Vec::new();
    let mut errors = 0;
    rx.push(&wire, |r| match r {
        Ok(v) => oks.push(v),
        Err(_) => errors += 1,
    });
    assert_eq!(oks, vec![vec![0x11], vec![0x22]]);
    assert_eq!(errors, 1);
}
