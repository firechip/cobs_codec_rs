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
