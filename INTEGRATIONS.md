# Integrating with async & streaming frameworks

`cobs_codec_rs` deliberately keeps its core `#![no_std]` and dependency-free: it
encodes, decodes, and frames over plain `&[u8]`, and nothing else. Adapters to a
specific async runtime or I/O framework live in **your** project, built on the
crate's public API — so the core never drags `tokio`, `bytes`, or an embedded
HAL into everyone's dependency tree.

The recipes below are copy-paste starting points, **not** part of the published
crate. Each is verified against the crate's real API.

## Tokio — a `tokio_util::codec` for `Framed` streams

A complete `Encoder` + `Decoder` that frames each packet as COBS plus a `0x00`
delimiter, so it drops straight into `Framed` / `FramedRead` / `FramedWrite`
over any `AsyncRead` / `AsyncWrite` (a TCP socket, or a serial port via
`tokio-serial`, …).

In *your* `Cargo.toml`:

```toml
[dependencies]
cobs_codec_rs = "1"
tokio-util = { version = "0.7", features = ["codec"] }
bytes = "1"
```

```rust
use bytes::{Buf, BytesMut};
use cobs_codec_rs::{cobs, max_encoded_len, DELIMITER};
use tokio_util::codec::{Decoder, Encoder};

/// Frames each packet with COBS and a trailing `0x00` delimiter.
#[derive(Default)]
pub struct CobsFrameCodec;

impl Encoder<&[u8]> for CobsFrameCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> Result<(), Self::Error> {
        let start = dst.len();
        // Worst-case COBS output, plus one byte for the frame delimiter.
        dst.resize(start + max_encoded_len(item.len()) + 1, 0);
        let n = cobs::encode(item, &mut dst[start..]);
        dst[start + n] = DELIMITER;
        dst.truncate(start + n + 1);
        Ok(())
    }
}

impl Decoder for CobsFrameCodec {
    type Item = Vec<u8>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // A frame is everything up to the next delimiter; wait if it is absent.
        let Some(pos) = src.iter().position(|&b| b == DELIMITER) else {
            return Ok(None);
        };
        let frame = src.split_to(pos);
        src.advance(1); // discard the delimiter
        cobs::decode_to_vec(&frame)
            .map(Some)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
```

Then:

```rust
use futures::{SinkExt, StreamExt};
use tokio_util::codec::{FramedRead, FramedWrite};

// write side
let mut w = FramedWrite::new(writer, CobsFrameCodec);
w.send(&b"\x11\x00\x22"[..]).await?;

// read side — yields each decoded packet
let mut r = FramedRead::new(reader, CobsFrameCodec);
while let Some(packet) = r.next().await {
    let packet = packet?;
    // ...
}
```

## Embedded — `embedded-io` / `embedded-io-async`

The same shape works `no_std`. Scan an `embedded_io::Read` for the `0x00`
delimiter into a fixed `[u8; N]`, then call `cobs::decode` into a decoded buffer
— or `cobs::decode_in_place` to decode within the same buffer. Because the core
needs no allocator, the whole path is allocation-free. The crate's
`framing::FrameDecoder` already does the incremental delimiter scanning if you
would rather reuse it than hand-roll the loop.

## What stays in the core

Everything you build on: `cobs` / `cobsr` encode/decode (with sentinel and
in-place variants), `framing` for `0x00`-delimited streams, and the size helpers
`max_encoded_len` / `encoding_overhead`. The adapters above are just the seam
where your chosen framework meets that API.
