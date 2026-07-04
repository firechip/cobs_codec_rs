// Verifies this crate against the shared conformance vectors from
// https://github.com/firechip/cobs-conformance
//
// Skipped unless COBS_CONFORMANCE_VECTORS points to a downloaded vectors.jsonl.
#![allow(clippy::pedantic, missing_docs)]

use cobs_codec_rs::{cobs, cobsr};
use std::io::{BufRead, BufReader};

fn from_hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Extracts the value of `"<key>":"<hex>"` from a compact JSON line.
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    let start = line.find(&pat).expect("missing key") + pat.len();
    let rest = &line[start..];
    let end = rest.find('"').expect("unterminated value");
    &rest[..end]
}

/// Extracts `"<key>":<value>` where the value is either a quoted hex string or
/// the JSON literal `null`. Returns `None` for `null` (i.e. decode must fail).
fn field_opt<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let start = line.find(&pat).expect("missing key") + pat.len();
    let rest = &line[start..];
    if rest.starts_with("null") {
        return None;
    }
    let rest = rest
        .strip_prefix('"')
        .expect("expected quoted value or null");
    let end = rest.find('"').expect("unterminated value");
    Some(&rest[..end])
}

#[test]
fn conforms_to_shared_vectors() {
    let path = match std::env::var("COBS_CONFORMANCE_VECTORS") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            eprintln!("skipping: set COBS_CONFORMANCE_VECTORS to run");
            return;
        }
    };

    let file = std::fs::File::open(&path).expect("open vectors file");
    let mut count = 0usize;
    for line in BufReader::new(file).lines() {
        let line = line.expect("read line");
        if line.trim().is_empty() {
            continue;
        }
        let decoded = from_hex(field(&line, "decoded"));
        let e_cobs = from_hex(field(&line, "cobs"));
        let e_cobsr = from_hex(field(&line, "cobsr"));

        assert_eq!(cobs::encode_to_vec(&decoded), e_cobs, "cobs encode");
        assert_eq!(cobsr::encode_to_vec(&decoded), e_cobsr, "cobsr encode");
        assert_eq!(
            cobs::decode_to_vec(&e_cobs).unwrap(),
            decoded,
            "cobs decode"
        );
        assert_eq!(
            cobsr::decode_to_vec(&e_cobsr).unwrap(),
            decoded,
            "cobsr decode"
        );
        count += 1;
    }
    assert!(count > 0, "no vectors found in {path}");
    eprintln!("conformance: {count} vectors verified");
}

#[test]
fn conforms_to_sentinel_vectors() {
    let path = match std::env::var("COBS_CONFORMANCE_SENTINEL") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            eprintln!("skipping: set COBS_CONFORMANCE_SENTINEL to run");
            return;
        }
    };

    let file = std::fs::File::open(&path).expect("open sentinel file");
    let mut count = 0usize;
    for line in BufReader::new(file).lines() {
        let line = line.expect("read line");
        if line.trim().is_empty() {
            continue;
        }
        let decoded = from_hex(field(&line, "decoded"));
        let sentinel = from_hex(field(&line, "sentinel"));
        assert_eq!(sentinel.len(), 1, "sentinel must be a single byte");
        let sentinel = sentinel[0];
        let e_cobs = from_hex(field(&line, "cobs"));
        let e_cobsr = from_hex(field(&line, "cobsr"));

        assert_eq!(
            cobs::encode_to_vec_with_sentinel(&decoded, sentinel),
            e_cobs,
            "cobs encode_with_sentinel"
        );
        assert_eq!(
            cobsr::encode_to_vec_with_sentinel(&decoded, sentinel),
            e_cobsr,
            "cobsr encode_with_sentinel"
        );
        assert_eq!(
            cobs::decode_to_vec_with_sentinel(&e_cobs, sentinel).unwrap(),
            decoded,
            "cobs decode_with_sentinel"
        );
        assert_eq!(
            cobsr::decode_to_vec_with_sentinel(&e_cobsr, sentinel).unwrap(),
            decoded,
            "cobsr decode_with_sentinel"
        );
        assert!(
            !e_cobs.contains(&sentinel),
            "sentinel byte must not appear in cobs output"
        );
        assert!(
            !e_cobsr.contains(&sentinel),
            "sentinel byte must not appear in cobsr output"
        );
        count += 1;
    }
    assert!(count > 0, "no sentinel vectors found in {path}");
    eprintln!("conformance: {count} sentinel vectors verified");
}

#[test]
fn conforms_to_error_vectors() {
    let path = match std::env::var("COBS_CONFORMANCE_ERRORS") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            eprintln!("skipping: set COBS_CONFORMANCE_ERRORS to run");
            return;
        }
    };

    let file = std::fs::File::open(&path).expect("open errors file");
    let mut count = 0usize;
    for line in BufReader::new(file).lines() {
        let line = line.expect("read line");
        if line.trim().is_empty() {
            continue;
        }
        let encoded = from_hex(field(&line, "encoded"));

        match field_opt(&line, "cobs") {
            Some(hex) => assert_eq!(
                cobs::decode_to_vec(&encoded).unwrap(),
                from_hex(hex),
                "cobs decode"
            ),
            None => assert!(
                cobs::decode_to_vec(&encoded).is_err(),
                "cobs decode must fail"
            ),
        }
        match field_opt(&line, "cobsr") {
            Some(hex) => assert_eq!(
                cobsr::decode_to_vec(&encoded).unwrap(),
                from_hex(hex),
                "cobsr decode"
            ),
            None => assert!(
                cobsr::decode_to_vec(&encoded).is_err(),
                "cobsr decode must fail"
            ),
        }
        count += 1;
    }
    assert!(count > 0, "no error vectors found in {path}");
    eprintln!("conformance: {count} error vectors verified");
}
