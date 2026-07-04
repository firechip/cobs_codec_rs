# Changelog

All notable changes to this crate are documented here. This project adheres to
[Semantic Versioning](https://semver.org).

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
