//! WebSocket message streaming protocol for large payloads (client-side).
//!
//! Protocol constants are duplicated from the server-side module to avoid
//! adding a crate dependency from stdlib to core.
//!
//! All WebSocket messages use a 1-byte type prefix:
//! - `0x00` + payload = complete message
//! - `0x01` + 4 bytes (total_chunks LE) + payload = stream chunk
//!
//! The chunk header is 5 bytes (`CHUNK_HEADER_SIZE`): the 1-byte type prefix
//! followed by a single little-endian `u32` total_chunks field.

const MSG_COMPLETE: u8 = 0x00;
const MSG_CHUNK: u8 = 0x01;

/// 1 (type) + 4 (total_chunks).
pub const CHUNK_HEADER_SIZE: usize = 5;

/// Default chunk payload size: 256 KiB.
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

/// Messages larger than this threshold are chunked.
pub const CHUNK_THRESHOLD: usize = 512 * 1024;

/// Maximum `total_chunks` accepted from the wire.
/// Based on MAX_STATE_SIZE (50 MiB) / DEFAULT_CHUNK_SIZE.
const MAX_TOTAL_CHUNKS: u32 = 256;

/// Parsed streaming message.
#[derive(Debug)]
pub enum StreamMessage<'a> {
    Complete(&'a [u8]),
    Chunk {
        total_chunks: u32,
        payload: &'a [u8],
    },
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("message too short: expected at least {expected} bytes, got {actual}")]
    MessageTooShort { expected: usize, actual: usize },
    #[error("unknown message type prefix: 0x{0:02x}")]
    UnknownMessageType(u8),
    #[error("total_chunks is zero")]
    ZeroTotalChunks,
    #[error("total_chunks {total_chunks} exceeds maximum {max}")]
    TotalChunksTooLarge { total_chunks: u32, max: u32 },
    #[error("total_chunks mismatch (expected {expected}, got {actual})")]
    TotalChunksMismatch { expected: u32, actual: u32 },
}

/// Wraps a serialized payload as a complete (non-chunked) streaming message.
pub fn wrap_complete(data: Vec<u8>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + data.len());
    buf.push(MSG_COMPLETE);
    buf.extend_from_slice(&data);
    buf
}

/// Splits a serialized payload into chunked streaming messages.
pub fn chunk_payload(data: &[u8]) -> Vec<Vec<u8>> {
    if data.is_empty() {
        let mut buf = Vec::with_capacity(CHUNK_HEADER_SIZE);
        buf.push(MSG_CHUNK);
        buf.extend_from_slice(&1u32.to_le_bytes());
        return vec![buf];
    }

    let total_chunks = data.len().div_ceil(DEFAULT_CHUNK_SIZE);
    let mut chunks = Vec::with_capacity(total_chunks);

    for chunk_data in data.chunks(DEFAULT_CHUNK_SIZE) {
        let mut buf = Vec::with_capacity(CHUNK_HEADER_SIZE + chunk_data.len());
        buf.push(MSG_CHUNK);
        buf.extend_from_slice(&(total_chunks as u32).to_le_bytes());
        buf.extend_from_slice(chunk_data);
        chunks.push(buf);
    }

    chunks
}

/// Parses a raw WebSocket binary message into a streaming protocol message.
pub fn parse_message(data: &[u8]) -> Result<StreamMessage<'_>, StreamError> {
    if data.is_empty() {
        return Err(StreamError::MessageTooShort {
            expected: 1,
            actual: 0,
        });
    }

    match data[0] {
        MSG_COMPLETE => Ok(StreamMessage::Complete(&data[1..])),
        MSG_CHUNK => {
            if data.len() < CHUNK_HEADER_SIZE {
                return Err(StreamError::MessageTooShort {
                    expected: CHUNK_HEADER_SIZE,
                    actual: data.len(),
                });
            }
            let total_chunks = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);

            if total_chunks == 0 {
                return Err(StreamError::ZeroTotalChunks);
            }
            if total_chunks > MAX_TOTAL_CHUNKS {
                return Err(StreamError::TotalChunksTooLarge {
                    total_chunks,
                    max: MAX_TOTAL_CHUNKS,
                });
            }

            Ok(StreamMessage::Chunk {
                total_chunks,
                payload: &data[CHUNK_HEADER_SIZE..],
            })
        }
        other => Err(StreamError::UnknownMessageType(other)),
    }
}

/// Sequential reassembly buffer for chunked streams.
///
/// TCP guarantees ordered delivery and the select loop serializes message sends,
/// so chunks always arrive in order. This buffer simply appends incoming chunks
/// and returns the complete payload when all arrive.
pub struct ChunkReassemblyBuffer {
    data: Vec<u8>,
    total_chunks: u32,
    received: u32,
}

impl ChunkReassemblyBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            total_chunks: 0,
            received: 0,
        }
    }

    /// Receives a chunk and returns the fully reassembled payload when all chunks arrive.
    ///
    /// Returns `Ok(None)` if more chunks are needed.
    pub fn receive_chunk(
        &mut self,
        total_chunks: u32,
        payload: &[u8],
    ) -> Result<Option<Vec<u8>>, StreamError> {
        if self.received == 0 {
            self.total_chunks = total_chunks;
            self.data
                .reserve(total_chunks as usize * DEFAULT_CHUNK_SIZE);
        } else if self.total_chunks != total_chunks {
            return Err(StreamError::TotalChunksMismatch {
                expected: self.total_chunks,
                actual: total_chunks,
            });
        }

        self.data.extend_from_slice(payload);
        self.received += 1;

        if self.received == self.total_chunks {
            let result = std::mem::take(&mut self.data);
            self.received = 0;
            self.total_chunks = 0;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_complete_roundtrip() {
        let data = vec![1, 2, 3, 4, 5];
        let wrapped = wrap_complete(data.clone());
        assert_eq!(wrapped[0], MSG_COMPLETE);
        match parse_message(&wrapped).unwrap() {
            StreamMessage::Complete(payload) => assert_eq!(payload, &data[..]),
            StreamMessage::Chunk { .. } => panic!("expected Complete"),
        }
    }

    #[test]
    fn chunk_small_payload_roundtrip() {
        let data = vec![42u8; 1024];
        let chunks = chunk_payload(&data);
        assert_eq!(chunks.len(), 1);

        match parse_message(&chunks[0]).unwrap() {
            StreamMessage::Chunk {
                total_chunks,
                payload,
            } => {
                assert_eq!(total_chunks, 1);
                assert_eq!(payload, &data[..]);
            }
            StreamMessage::Complete(_) => panic!("expected Chunk"),
        }
    }

    #[test]
    fn chunk_large_payload_roundtrip() {
        let data: Vec<u8> = (0..600 * 1024).map(|i| (i % 256) as u8).collect();
        let chunks = chunk_payload(&data);
        assert_eq!(chunks.len(), 3);

        let mut reassembly = ChunkReassemblyBuffer::new();
        for (i, chunk) in chunks.iter().enumerate() {
            match parse_message(chunk).unwrap() {
                StreamMessage::Chunk {
                    total_chunks,
                    payload,
                } => {
                    let result = reassembly.receive_chunk(total_chunks, payload).unwrap();
                    if i < 2 {
                        assert!(result.is_none());
                    } else {
                        assert_eq!(result.unwrap(), data);
                    }
                }
                StreamMessage::Complete(_) => panic!("expected Chunk"),
            }
        }
    }

    #[test]
    fn chunk_empty_payload() {
        let chunks = chunk_payload(&[]);
        assert_eq!(chunks.len(), 1);

        match parse_message(&chunks[0]).unwrap() {
            StreamMessage::Chunk {
                total_chunks,
                payload,
            } => {
                assert_eq!(total_chunks, 1);
                assert!(payload.is_empty());

                let mut reassembly = ChunkReassemblyBuffer::new();
                let result = reassembly.receive_chunk(total_chunks, payload).unwrap();
                assert_eq!(result.unwrap(), Vec::<u8>::new());
            }
            StreamMessage::Complete(_) => panic!("expected Chunk"),
        }
    }

    #[test]
    fn parse_errors() {
        assert!(matches!(
            parse_message(&[]).unwrap_err(),
            StreamError::MessageTooShort { .. }
        ));
        assert!(matches!(
            parse_message(&[0xFF, 1, 2, 3]).unwrap_err(),
            StreamError::UnknownMessageType(0xFF)
        ));
        assert!(matches!(
            parse_message(&[MSG_CHUNK, 0, 0]).unwrap_err(),
            StreamError::MessageTooShort { .. }
        ));

        let mut zero_chunks = vec![MSG_CHUNK];
        zero_chunks.extend_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_message(&zero_chunks).unwrap_err(),
            StreamError::ZeroTotalChunks
        ));

        let mut too_large = vec![MSG_CHUNK];
        too_large.extend_from_slice(&1000u32.to_le_bytes());
        assert!(matches!(
            parse_message(&too_large).unwrap_err(),
            StreamError::TotalChunksTooLarge { .. }
        ));
    }

    #[test]
    fn total_chunks_mismatch() {
        let mut reassembly = ChunkReassemblyBuffer::new();
        reassembly.receive_chunk(3, &[1, 2, 3]).unwrap();
        assert!(matches!(
            reassembly.receive_chunk(5, &[4, 5, 6]).unwrap_err(),
            StreamError::TotalChunksMismatch { .. }
        ));
    }

    #[test]
    fn reassembly_resets_after_completion() {
        let data_a = vec![0xAA; DEFAULT_CHUNK_SIZE * 2];
        let data_b = vec![0xBB; DEFAULT_CHUNK_SIZE * 3];

        let mut reassembly = ChunkReassemblyBuffer::new();

        for chunk in &chunk_payload(&data_a) {
            if let StreamMessage::Chunk {
                total_chunks,
                payload,
            } = parse_message(chunk).unwrap()
            {
                if let Some(r) = reassembly.receive_chunk(total_chunks, payload).unwrap() {
                    assert_eq!(r, data_a);
                }
            }
        }

        for chunk in &chunk_payload(&data_b) {
            if let StreamMessage::Chunk {
                total_chunks,
                payload,
            } = parse_message(chunk).unwrap()
            {
                if let Some(r) = reassembly.receive_chunk(total_chunks, payload).unwrap() {
                    assert_eq!(r, data_b);
                }
            }
        }
    }
}
