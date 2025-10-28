//! nwd1-quic
//! QUIC transport for `nwd1` binary frames.
//!
//! NOTE: future optimization idea — replace `Vec<u8>` allocations with pooled or preallocated `BytesMut` buffers to reduce churn under high stream load.
//!
//! This crate integrates [`nwd1::Frame`] with the [`quinn`] QUIC implementation,
//! providing async send/receive helpers for bidirectional streams.

use bytes::BytesMut;
use nwd1::{Frame, MAGIC, decode, encode};
use quinn::{RecvStream, SendStream};

const HEADER_LEN: usize = 8;
const MAX_FRAME_LEN: usize = 8 * 1024 * 1024; // 8 MiB sanity cap to avoid pathological allocations

#[inline]
async fn read_exact_opt(
    stream: &mut RecvStream,
    buf: &mut [u8],
) -> Result<Option<()>, std::io::Error> {
    match stream.read_exact(buf).await {
        Ok(()) => Ok(Some(())),
        Err(quinn::ReadExactError::FinishedEarly(_)) => Ok(None),
        Err(quinn::ReadExactError::ReadError(e)) => Err(std::io::Error::from(e)),
    }
}

/// Send a single frame over a QUIC bidirectional stream.
///
/// This function writes the encoded frame bytes to the stream and returns immediately. The stream remains open for further writes.
pub async fn send_frame(stream: &mut SendStream, frame: &Frame) -> Result<(), quinn::WriteError> {
    let data = encode(frame);
    stream.write_all(&data).await?;
    Ok(())
}

/// Receive a single frame from a QUIC bidirectional stream.
///
/// This function reads until a complete frame is received and decodes it.
/// It returns `None` if the stream ends gracefully.
pub async fn recv_frame(stream: &mut RecvStream) -> Result<Option<Frame>, std::io::Error> {
    let mut header = [0u8; HEADER_LEN];
    if read_exact_opt(stream, &mut header).await?.is_none() {
        return Ok(None);
    }

    // Fast-fail on bad magic to avoid large allocations
    if &header[..4] != MAGIC {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "nwd1 bad magic"));
    }

    // Parse LEN (bytes 4..8) as big-endian u32
    let len = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;

    if len > MAX_FRAME_LEN {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "nwd1 frame too large"));
    }

    let mut body = vec![0u8; len];
    if read_exact_opt(stream, &mut body).await?.is_none() {
        return Ok(None);
    }

    let mut buf = BytesMut::with_capacity(8 + len);
    buf.extend_from_slice(&header);
    buf.extend_from_slice(&body);

    let frame = match decode(&buf.freeze()) {
        Ok(f) => f,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "nwd1 decode error",
            ));
        }
    };
    Ok(Some(frame))
}

/// Minimal self-test to ensure the functions compile and link.
#[cfg(test)]
mod tests {
	use netid64::NetId64;
    use super::*;
    use bytes::Bytes;

    #[tokio::test]
    async fn encode_decode_roundtrip() {
        let frame = Frame {
            id: NetId64::make(1, 7, 42),
            kind: 1,
            ver: 1,
            payload: Bytes::from_static(b"ping"),
        };

        // Encoding must produce non-empty bytes
        let data = encode(&frame);
        assert!(!data.is_empty());
        let decoded = decode(&data).unwrap();
        assert_eq!(decoded.id.raw(), frame.id.raw());
        assert_eq!(decoded.payload, frame.payload);
    }
}
