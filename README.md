# cobs_codec_rs

[![CI](https://github.com/firechip/cobs_codec_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/firechip/cobs_codec_rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cobs_codec_rs.svg)](https://crates.io/crates/cobs_codec_rs)
[![docs.rs](https://img.shields.io/docsrs/cobs_codec_rs)](https://docs.rs/cobs_codec_rs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`#![no_std]`, dependency-free **Consistent Overhead Byte Stuffing (COBS)** and
**COBS/R** for Rust — the Rust member of the Firechip COBS family (alongside the
Dart [`cobs_codec`](https://pub.dev/packages/cobs_codec) and the Kotlin
[`cobs_codec_kt`](https://github.com/firechip/cobs_codec_kt)), verified
byte-identical against the shared
[conformance vectors](https://github.com/firechip/cobs-conformance).

COBS encodes an arbitrary byte sequence into one that contains no zero (`0x00`)
byte, at a small and *predictable* cost: at most one extra byte per 254 bytes,
plus one. That makes a single `0x00` a reliable packet delimiter for serial/UART,
USB, TCP and other byte streams — ideal for embedded and robotics protocols.

## Features

- **Basic COBS** and **COBS/R (Reduced)** encode/decode.
- **`no_std` and zero dependencies** — the core `encode`/`decode` work on
  caller-provided slices, allocating nothing.
- **Configurable sentinel** — `*_with_sentinel` variants frame with any
  delimiter byte, not just `0x00`.
- **In-place decoding** — `cobs::decode_in_place` decodes without a second
  buffer.
- **Allocation-free streaming** — `framing::StreamDecoder` reassembles
  delimited frames into a fixed buffer in pure `no_std`; the `alloc` feature adds
  the owned-`Vec`
  [`FrameDecoder`](https://docs.rs/cobs_codec_rs/latest/cobs_codec_rs/framing/struct.FrameDecoder.html)
  and `*_to_vec` conveniences.
- **`const fn`** size helpers (`max_encoded_len`, `encoding_overhead`) for
  compile-time buffer sizing.

## Install

```toml
[dependencies]
cobs_codec_rs = "1.2"

# no_std, no allocator:
# cobs_codec_rs = { version = "1.2", default-features = false }
```

## Usage

```rust
use cobs_codec_rs::{cobs, cobsr};

// With alloc (default):
let encoded = cobs::encode_to_vec(&[0x11, 0x22, 0x00, 0x33]);
assert_eq!(encoded, [0x03, 0x11, 0x22, 0x02, 0x33]); // no 0x00
assert_eq!(cobs::decode_to_vec(&encoded).unwrap(), [0x11, 0x22, 0x00, 0x33]);

// COBS/R often saves the trailing overhead byte:
assert_eq!(cobsr::encode_to_vec(b"12345"), b"51234"); // same length as input
```

`no_std`, into a fixed buffer:

```rust
use cobs_codec_rs::{cobs, max_encoded_len};

let src = [0x11, 0x00, 0x22];
let mut buf = [0u8; max_encoded_len(3)];
let n = cobs::encode(&src, &mut buf);
assert_eq!(&buf[..n], &[0x02, 0x11, 0x02, 0x22]);
```

With a custom sentinel byte, so a non-`0x00` byte delimits frames (both `cobs`
and `cobsr`, slice and `*_to_vec` variants):

```rust
use cobs_codec_rs::cobs;

// 0xAA delimits frames instead of 0x00; the encoded output never contains it.
// (`sentinel == 0` is identical to the plain codec.)
let encoded = cobs::encode_to_vec_with_sentinel(&[0x11, 0xAA, 0x22], 0xAA);
assert_eq!(encoded, [0xAE, 0xBB, 0x00, 0x88]); // no 0xAA byte
assert_eq!(
    cobs::decode_to_vec_with_sentinel(&encoded, 0xAA).unwrap(),
    [0x11, 0xAA, 0x22],
);
```

Decoding in place, without a second buffer (basic COBS only):

```rust
use cobs_codec_rs::cobs;

// COBS never expands on decode, so it can decode within the input buffer; the
// decoded bytes end up in `buf[..len]`.
let mut buf = [0x03, 0x11, 0x22, 0x02, 0x33];
let len = cobs::decode_in_place(&mut buf).unwrap();
assert_eq!(&buf[..len], &[0x11, 0x22, 0x00, 0x33]);
```

Reassembling a sentinel-delimited stream with no allocator, into a fixed buffer:

```rust
use cobs_codec_rs::cobs;
use cobs_codec_rs::framing::StreamDecoder;

// Encode a packet with sentinel 0xAA, then delimit it with an 0xAA byte.
let mut wire = [0u8; 16];
let n = cobs::encode_with_sentinel(&[0x11, 0x00, 0x22], &mut wire, 0xAA);
wire[n] = 0xAA;

// Reassemble it into a fixed scratch buffer — no allocation anywhere.
let mut scratch = [0u8; 8];
let mut decoder = StreamDecoder::new(&mut scratch).sentinel(0xAA); // .reduced(true) for COBS/R

let mut out = [0u8; 8];
let mut out_len = 0;
decoder.push(&wire[..n + 1], |frame| {
    let frame = frame.unwrap();
    out[..frame.len()].copy_from_slice(frame);
    out_len = frame.len();
});
assert_eq!(&out[..out_len], &[0x11, 0x00, 0x22]);
```

Reading a delimited serial stream (needs `alloc`):

```rust
use cobs_codec_rs::framing::{frame_to_vec, FrameDecoder};

let mut rx = FrameDecoder::new().max_frame_len(4096);
// `chunk` is any &[u8] read from the link; chunks need not align with frames.
# let chunk = frame_to_vec(&[0x01, 0x02]);
rx.push(&chunk, |frame| match frame {
    Ok(packet) => { /* handle packet */ }
    Err(err)   => { /* corrupt frame; keep receiving */ let _ = err; }
});
```

### Framing Protocol Buffers over serial

COBS is the standard way to frame **Protobuf** on a UART/RS-485 link: protobuf
serializes a message but doesn't delimit it, and COBS supplies the missing
`0x00`-delimited framing — with instant resync after line noise, unlike
length-prefixing. See [`examples/protobuf_cobs.rs`](examples/protobuf_cobs.rs)
for a runnable device→host demo (`cargo run --example protobuf_cobs`) that
survives a corrupted frame.

## Overhead

COBS overhead is *data-independent*. Encoding an `n`-byte packet produces at most

$$ n + \left\lceil \frac{n}{254} \right\rceil $$

bytes (one extra byte per 254, rounded up), so the overhead is bounded by
$\left\lceil n/254 \right\rceil$ and is always at least one byte. By contrast,
escape-based schemes (PPP, SLIP, HDLC) can *double* the packet in the worst case.
`cobsr` (COBS/R) can reach zero overhead. These bounds are what `max_encoded_len`
and `encoding_overhead` return.

## Background

Stuart Cheshire and Mary Baker, "Consistent Overhead Byte Stuffing",
*IEEE/ACM Transactions on Networking*, Vol. 7, No. 2, April 1999. **COBS/R** is a
variant by Craig McQueen.

## License

MIT © 2026 Alexander Salas Bastidas ([Firechip](https://firechip.dev)). See
[LICENSE](LICENSE).
