//! Framing Protocol Buffers over a serial link with COBS.
//!
//! Protobuf is a *serialization* format, not a *wire* format: it turns a message
//! into a compact byte string but says nothing about where one message ends and
//! the next begins. Push raw protobuf bytes down a UART and the receiver sees an
//! undifferentiated stream. COBS supplies the missing framing:
//!
//! 1. **Serialize** the message with protobuf (the bytes usually contain `0x00`).
//! 2. **COBS-encode** it so the payload contains no `0x00`, then append a single
//!    `0x00` **delimiter**. That byte now unambiguously marks the end of a frame.
//! 3. On the far end, split the stream on `0x00`, COBS-decode each frame, and hand
//!    the bytes back to protobuf.
//!
//! The payoff over length-prefixing: if line noise corrupts a packet, the receiver
//! just waits for the next `0x00` and **resynchronises on the very next message**,
//! discarding only the broken frame instead of losing sync forever. This example
//! shows exactly that.
//!
//! Run it with `cargo run --example protobuf_cobs`.
//!
//! On a microcontroller you would use the allocation-free API — [`framing::frame`]
//! to encode into a fixed `[u8; N]`, and [`framing::StreamDecoder`] to decode an
//! incoming stream into another fixed buffer — for the same result with zero heap.
//!
//! [`framing::frame`]: cobs_codec_rs::framing::frame
//! [`framing::StreamDecoder`]: cobs_codec_rs::framing::StreamDecoder
#![allow(missing_docs)]

use cobs_codec_rs::framing::{FrameDecoder, frame_to_vec};
use prost::Message;

/// A telemetry packet a sensor node might stream to a host computer.
#[derive(Clone, PartialEq, prost::Message)]
struct SensorReading {
    #[prost(uint32, tag = "1")]
    node_id: u32,
    #[prost(float, tag = "2")]
    temperature_c: f32,
    #[prost(uint64, tag = "3")]
    uptime_ms: u64,
    #[prost(string, tag = "4")]
    label: String,
}

fn main() {
    let readings = [
        SensorReading {
            node_id: 1,
            temperature_c: 21.5,
            uptime_ms: 1_000,
            label: "cabin".into(),
        },
        SensorReading {
            node_id: 2,
            temperature_c: 100.0,
            uptime_ms: 1_256,
            label: "boiler".into(),
        },
        SensorReading {
            node_id: 7,
            temperature_c: -4.25,
            uptime_ms: 99_999,
            label: "outside".into(),
        },
    ];

    // --- Device side: serialize each reading with protobuf, then COBS-frame it. ---
    let mut wire: Vec<u8> = Vec::new();
    let mut frame_starts: Vec<usize> = Vec::new();
    for reading in &readings {
        frame_starts.push(wire.len());
        let payload = reading.encode_to_vec(); // protobuf bytes (the float alone brings 0x00s)
        wire.extend_from_slice(&frame_to_vec(&payload)); // COBS-encode + trailing 0x00
    }
    // COBS guarantees a zero-free payload, so every 0x00 on the wire is a frame
    // delimiter — provably one per frame.
    println!(
        "device: {} readings -> {} wire bytes; the only zeros are the {} frame delimiters",
        readings.len(),
        wire.len(),
        readings.len(),
    );

    // --- Inject line noise: corrupt reading #2's leading COBS code byte. ---
    // A bogus code byte makes the frame claim more data than it holds, so it fails
    // to decode — a stand-in for any bit-flip a noisy UART might introduce.
    wire[frame_starts[1]] = 0xFF;
    println!("noise:  flipped a byte inside reading #2's frame\n");

    // --- Host side: reassemble across misaligned UART reads and protobuf-decode. ---
    let mut rx = FrameDecoder::new().max_frame_len(256);
    let mut received = 0usize;
    // A UART hands you whatever bytes have arrived, never aligned to frames.
    for chunk in wire.chunks(5) {
        rx.push(chunk, |frame| match frame {
            Ok(bytes) => match SensorReading::decode(&*bytes) {
                Ok(msg) => {
                    received += 1;
                    println!(
                        "host <- node {:>2}: {:+6.2} C, up {:>6} ms, \"{}\"",
                        msg.node_id, msg.temperature_c, msg.uptime_ms, msg.label,
                    );
                }
                Err(err) => println!("host <- valid COBS frame but bad protobuf: {err}"),
            },
            Err(err) => println!("host <- dropped a corrupt frame ({err}); resyncing on next 0x00"),
        });
    }

    println!(
        "\nreceived {received}/{} readings: the corrupted frame was skipped and the \
         stream resynced on the next delimiter.",
        readings.len(),
    );
    assert_eq!(received, readings.len() - 1);
}
