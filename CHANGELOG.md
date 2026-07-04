# Changelog

All notable changes to this crate are documented here. This project adheres to
[Semantic Versioning](https://semver.org).

## 1.3.0

### Added

- **COBS/R in-place decoding**: `decode_in_place` and
  `decode_in_place_with_sentinel` for the reduced codec, decoding within the
  caller's buffer and returning the decoded length (COBS/R decoding never
  expands the data).
- **Optional `serde` and `defmt` derives** for `DecodeError`, behind the new
  `serde` and `defmt` feature flags, for serialising or logging decode errors on
  hosts and embedded targets.

### Tooling & tests

- Added a `criterion` throughput benchmark (`benches/throughput.rs`).
- Added fuzzing: `cargo-fuzz` targets plus a stable, dependency-free robustness
  test that runs in ordinary CI.
- Added a Protobuf + COBS framing example (`examples/protobuf_cobs.rs`).
- Extended the conformance suite to also check the configurable-sentinel and
  decode-error vectors from firechip/cobs-conformance.

## 1.2.0

### Changed

- Adopted **Rust edition 2024**; the minimum supported Rust version is now
  **1.85** (required by the edition). No API changes.
- Modernised CI: bumped `actions/checkout` to v7 and pinned the MSRV job's
  toolchain to Rust 1.85.

### Documentation

- The Cheshire & Baker (1999) overhead bounds now render as math on docs.rs (via
  KaTeX): the encoded length is at most $n + \lceil n/254 \rceil$ bytes, and
  `max_encoded_len` / `encoding_overhead` document their closed forms.

## 1.1.0

### Added

- **Configurable sentinel**: `encode_with_sentinel` / `decode_with_sentinel`
  (and `*_to_vec_with_sentinel`) on both `cobs` and `cobsr`, for framing with a
  delimiter other than `0x00`.
- **In-place decoding**: `cobs::decode_in_place` and
  `cobs::decode_in_place_with_sentinel`, which decode without a second buffer.
- **`framing::StreamDecoder`**: an allocation-free streaming frame decoder that
  reassembles delimited packets into a caller-provided buffer — the pure
  `no_std` counterpart of `FrameDecoder`, with `reduced` and `sentinel` options.

All additions are backward compatible.

## 1.0.0

Initial release.

### Added

- `#![no_std]`, dependency-free **basic COBS** and **COBS/R** encode/decode over
  caller-provided slices (`cobs`/`cobsr` modules).
- `alloc`-gated `*_to_vec` conveniences and a streaming `framing::FrameDecoder`
  with a `max_frame_len` bound.
- `const fn` size helpers `max_encoded_len` and `encoding_overhead`.
- `DecodeError` implementing `core::error::Error`.
- Golden-vector tests plus a conformance test against
  [firechip/cobs-conformance](https://github.com/firechip/cobs-conformance)
  (2261 vectors, byte-identical to the reference).
