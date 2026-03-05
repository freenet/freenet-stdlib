//! Chunking and reassembly helpers for WebSocket message streaming.
//!
//! Large serialized payloads are split into [`ClientRequest::StreamChunk`] or
//! [`HostResponse::StreamChunk`] variants. Each chunk carries a `stream_id` so
//! multiple streams can be reassembled concurrently.

use std::collections::HashMap;

use bytes::Bytes;

use super::{ClientRequest, HostResponse};

/// Default chunk payload size: 256 KiB.
pub const CHUNK_SIZE: usize = 256 * 1024;

/// Messages larger than this threshold are chunked.
pub const CHUNK_THRESHOLD: usize = 512 * 1024;

/// Maximum `total_chunks` accepted from the wire.
/// 256 chunks * 256 KiB = 64 MiB, enough headroom for MAX_STATE_SIZE (50 MiB)
/// plus serialization overhead.
pub const MAX_TOTAL_CHUNKS: u32 = 256;

/// Maximum concurrent streams in a single `ReassemblyBuffer`.
pub const MAX_CONCURRENT_STREAMS: usize = 8;

/// Zero-copy chunking: split `data` into (index, total, slice) tuples using `Bytes::slice()`.
fn chunk_bytes(data: &Bytes) -> Vec<(u32, u32, Bytes)> {
    let total = data.len().div_ceil(CHUNK_SIZE).max(1) as u32;
    if data.is_empty() {
        return vec![(0, 1, Bytes::new())];
    }
    (0..total as usize)
        .map(|i| {
            let start = i * CHUNK_SIZE;
            let end = (start + CHUNK_SIZE).min(data.len());
            (i as u32, total, data.slice(start..end))
        })
        .collect()
}

/// Split a serialized request payload into `StreamChunk` client request variants.
///
/// Uses `Bytes::slice()` internally for zero-copy: each chunk shares the
/// original allocation via reference counting instead of copying.
pub fn chunk_request(data: Vec<u8>, stream_id: u32) -> Vec<ClientRequest<'static>> {
    let data = Bytes::from(data);
    chunk_bytes(&data)
        .into_iter()
        .map(|(index, total, chunk)| ClientRequest::StreamChunk {
            stream_id,
            index,
            total,
            data: chunk,
        })
        .collect()
}

/// Split a serialized response payload into `StreamChunk` host response variants.
///
/// Uses `Bytes::slice()` internally for zero-copy: each chunk shares the
/// original allocation via reference counting instead of copying.
pub fn chunk_response(data: Vec<u8>, stream_id: u32) -> Vec<HostResponse> {
    let data = Bytes::from(data);
    chunk_bytes(&data)
        .into_iter()
        .map(|(index, total, chunk)| HostResponse::StreamChunk {
            stream_id,
            index,
            total,
            data: chunk,
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("total_chunks is zero")]
    ZeroTotalChunks,
    #[error("total_chunks {total} exceeds maximum {max}")]
    TotalChunksTooLarge { total: u32, max: u32 },
    #[error("total_chunks mismatch for stream {stream_id} (expected {expected}, got {actual})")]
    TotalChunksMismatch {
        stream_id: u32,
        expected: u32,
        actual: u32,
    },
    #[error("duplicate chunk index {index} for stream {stream_id}")]
    DuplicateChunk { stream_id: u32, index: u32 },
    #[error("chunk index {index} out of range for stream {stream_id} (total {total})")]
    IndexOutOfRange {
        stream_id: u32,
        index: u32,
        total: u32,
    },
    #[error("too many concurrent streams ({count}), maximum is {max}")]
    TooManyConcurrentStreams { count: usize, max: usize },
    #[error("stream channel closed")]
    ChannelClosed,
    #[error("stream truncated: received {received} of {expected} bytes")]
    Truncated { received: u64, expected: u64 },
    #[error("stream overflow: received {received} bytes, expected at most {expected} bytes")]
    Overflow { received: u64, expected: u64 },
}

/// Timeout for incomplete streams in the reassembly buffer.
#[cfg(not(target_family = "wasm"))]
const STREAM_TTL: std::time::Duration = std::time::Duration::from_secs(60);

struct StreamState {
    chunks: Vec<Option<Bytes>>,
    total: u32,
    received: u32,
    #[cfg(not(target_family = "wasm"))]
    created_at: std::time::Instant,
}

/// Reassembly buffer keyed by stream ID. Supports concurrent streams.
pub struct ReassemblyBuffer {
    streams: HashMap<u32, StreamState>,
}

impl ReassemblyBuffer {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
        }
    }

    /// Feed a chunk into the buffer. Returns the fully reassembled payload
    /// when all chunks for a stream have arrived.
    pub fn receive_chunk(
        &mut self,
        stream_id: u32,
        index: u32,
        total: u32,
        data: Bytes,
    ) -> Result<Option<Vec<u8>>, StreamError> {
        if total == 0 {
            return Err(StreamError::ZeroTotalChunks);
        }
        if total > MAX_TOTAL_CHUNKS {
            return Err(StreamError::TotalChunksTooLarge {
                total,
                max: MAX_TOTAL_CHUNKS,
            });
        }
        if index >= total {
            return Err(StreamError::IndexOutOfRange {
                stream_id,
                index,
                total,
            });
        }

        // Evict stale entries before checking the concurrent limit.
        #[cfg(not(target_family = "wasm"))]
        self.evict_stale();

        // Reject new streams when the concurrent stream limit is reached.
        if !self.streams.contains_key(&stream_id) && self.streams.len() >= MAX_CONCURRENT_STREAMS {
            return Err(StreamError::TooManyConcurrentStreams {
                count: self.streams.len(),
                max: MAX_CONCURRENT_STREAMS,
            });
        }

        let state = self
            .streams
            .entry(stream_id)
            .or_insert_with(|| StreamState {
                chunks: vec![None; total as usize],
                total,
                received: 0,
                #[cfg(not(target_family = "wasm"))]
                created_at: std::time::Instant::now(),
            });

        if state.total != total {
            return Err(StreamError::TotalChunksMismatch {
                stream_id,
                expected: state.total,
                actual: total,
            });
        }

        let idx = index as usize;
        if state.chunks[idx].is_some() {
            return Err(StreamError::DuplicateChunk { stream_id, index });
        }

        state.chunks[idx] = Some(data);
        state.received += 1;

        if state.received == state.total {
            let state = self.streams.remove(&stream_id).unwrap();
            let exact_len: usize = state.chunks.iter().flatten().map(|c| c.len()).sum();
            let mut result = Vec::with_capacity(exact_len);
            for chunk in state.chunks.into_iter().flatten() {
                result.extend_from_slice(&chunk);
            }
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Remove a stream by ID, returning `true` if it existed.
    pub fn remove_stream(&mut self, stream_id: u32) -> bool {
        self.streams.remove(&stream_id).is_some()
    }

    #[cfg(not(target_family = "wasm"))]
    fn evict_stale(&mut self) {
        let now = std::time::Instant::now();
        self.streams
            .retain(|_id, state| now.duration_since(state.created_at) < STREAM_TTL);
    }
}

impl Default for ReassemblyBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// --- App-level streaming API (requires tokio) ---

#[cfg(all(feature = "net", not(target_family = "wasm")))]
pub use app_stream::*;

#[cfg(all(feature = "net", not(target_family = "wasm")))]
mod app_stream {
    use super::StreamError;
    use crate::client_api::client_events::StreamContent;
    use bytes::Bytes;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::sync::mpsc;

    /// Client-side handle for consuming a WebSocket stream incrementally.
    ///
    /// Created when the client receives a [`HostResponse::StreamHeader`] from the node.
    /// The corresponding [`WsStreamSender`] feeds chunks into this handle as they arrive.
    ///
    /// Two consumption modes:
    /// - [`into_stream()`](WsStreamHandle::into_stream) — incremental async `Stream<Item = Bytes>`
    /// - [`assemble()`](WsStreamHandle::assemble) — blocking wait for the complete payload
    pub struct WsStreamHandle {
        content: StreamContent,
        total_bytes: u64,
        chunk_rx: mpsc::UnboundedReceiver<Bytes>,
    }

    impl WsStreamHandle {
        /// Metadata describing what is being streamed.
        pub fn content(&self) -> &StreamContent {
            &self.content
        }

        /// Total expected bytes across all chunks.
        pub fn total_bytes(&self) -> u64 {
            self.total_bytes
        }

        /// Consume chunks incrementally as an async `Stream`.
        pub fn into_stream(self) -> WsStream {
            WsStream {
                chunk_rx: self.chunk_rx,
            }
        }

        /// Wait for all chunks and return the fully reassembled payload.
        ///
        /// Returns [`StreamError::Truncated`] if the sender closes before all
        /// expected bytes have been delivered, or [`StreamError::Overflow`] if
        /// more data is received than the header promised.
        pub async fn assemble(mut self) -> Result<Vec<u8>, StreamError> {
            // Reject total_bytes exceeding the protocol maximum before allocating.
            let protocol_max = super::MAX_TOTAL_CHUNKS as u64 * super::CHUNK_SIZE as u64;
            if self.total_bytes > protocol_max {
                return Err(StreamError::Overflow {
                    received: 0,
                    expected: protocol_max,
                });
            }
            // Cap pre-allocation to avoid OOM from a malicious total_bytes header.
            const MAX_PREALLOC: usize = 50 * 1024 * 1024;
            // Allow up to one extra chunk of slack beyond total_bytes.
            let max_bytes = (self.total_bytes as usize).saturating_add(super::CHUNK_SIZE);
            let mut buf = Vec::with_capacity((self.total_bytes as usize).min(MAX_PREALLOC));
            while let Some(chunk) = self.chunk_rx.recv().await {
                if buf.len().saturating_add(chunk.len()) > max_bytes {
                    return Err(StreamError::Overflow {
                        received: buf.len() as u64 + chunk.len() as u64,
                        expected: self.total_bytes,
                    });
                }
                buf.extend_from_slice(&chunk);
            }
            if (buf.len() as u64) < self.total_bytes {
                return Err(StreamError::Truncated {
                    received: buf.len() as u64,
                    expected: self.total_bytes,
                });
            }
            Ok(buf)
        }
    }

    /// Async stream of chunk data produced by [`WsStreamHandle::into_stream()`].
    pub struct WsStream {
        chunk_rx: mpsc::UnboundedReceiver<Bytes>,
    }

    impl futures::Stream for WsStream {
        type Item = Bytes;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.chunk_rx.poll_recv(cx)
        }
    }

    /// Sender side — held by the request handler to feed chunks into the handle.
    pub struct WsStreamSender {
        chunk_tx: mpsc::UnboundedSender<Bytes>,
    }

    impl WsStreamSender {
        /// Send a chunk of data to the corresponding [`WsStreamHandle`].
        pub fn send_chunk(&self, data: Bytes) -> Result<(), StreamError> {
            self.chunk_tx
                .send(data)
                .map_err(|_| StreamError::ChannelClosed)
        }
    }

    /// Create a paired (handle, sender) for a new stream.
    pub fn ws_stream_pair(
        content: StreamContent,
        total_bytes: u64,
    ) -> (WsStreamHandle, WsStreamSender) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            WsStreamHandle {
                content,
                total_bytes,
                chunk_rx: rx,
            },
            WsStreamSender { chunk_tx: tx },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_request_small() {
        let data = vec![42u8; 1024];
        let chunks = chunk_request(data.clone(), 1);
        assert_eq!(chunks.len(), 1);
        match &chunks[0] {
            ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data: chunk_data,
            } => {
                assert_eq!(*stream_id, 1);
                assert_eq!(*index, 0);
                assert_eq!(*total, 1);
                assert_eq!(chunk_data, &data);
            }
            _ => panic!("expected StreamChunk"),
        }
    }

    #[test]
    fn chunk_request_large_roundtrip() {
        let data: Vec<u8> = (0..600 * 1024).map(|i| (i % 256) as u8).collect();
        let chunks = chunk_request(data.clone(), 42);
        assert_eq!(chunks.len(), 3);

        let mut buf = ReassemblyBuffer::new();
        for chunk in &chunks {
            if let ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data: chunk_data,
            } = chunk
            {
                if let Some(result) = buf
                    .receive_chunk(*stream_id, *index, *total, chunk_data.clone())
                    .unwrap()
                {
                    assert_eq!(result, data);
                }
            }
        }
    }

    #[test]
    fn chunk_response_roundtrip() {
        let data = vec![0xAB; CHUNK_SIZE * 2];
        let chunks = chunk_response(data.clone(), 7);
        assert_eq!(chunks.len(), 2);

        let mut buf = ReassemblyBuffer::new();
        for chunk in &chunks {
            if let HostResponse::StreamChunk {
                stream_id,
                index,
                total,
                data: chunk_data,
            } = chunk
            {
                if let Some(result) = buf
                    .receive_chunk(*stream_id, *index, *total, chunk_data.clone())
                    .unwrap()
                {
                    assert_eq!(result, data);
                }
            }
        }
    }

    #[test]
    fn chunk_empty() {
        let chunks = chunk_request(Vec::new(), 1);
        assert_eq!(chunks.len(), 1);
        match &chunks[0] {
            ClientRequest::StreamChunk { total, data, .. } => {
                assert_eq!(*total, 1);
                assert!(data.is_empty());
            }
            _ => panic!("expected StreamChunk"),
        }
    }

    #[test]
    fn reassembly_resets_after_completion() {
        let data_a = vec![0xAA; CHUNK_SIZE * 2];
        let data_b = vec![0xBB; CHUNK_SIZE * 3];

        let mut buf = ReassemblyBuffer::new();

        for chunk in &chunk_request(data_a.clone(), 1) {
            if let ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data,
            } = chunk
            {
                if let Some(r) = buf
                    .receive_chunk(*stream_id, *index, *total, data.clone())
                    .unwrap()
                {
                    assert_eq!(r, data_a);
                }
            }
        }

        for chunk in &chunk_request(data_b.clone(), 2) {
            if let ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data,
            } = chunk
            {
                if let Some(r) = buf
                    .receive_chunk(*stream_id, *index, *total, data.clone())
                    .unwrap()
                {
                    assert_eq!(r, data_b);
                }
            }
        }
    }

    #[test]
    fn zero_total_chunks_error() {
        let mut buf = ReassemblyBuffer::new();
        let err = buf
            .receive_chunk(1, 0, 0, Bytes::from_static(&[1, 2, 3]))
            .unwrap_err();
        assert!(matches!(err, StreamError::ZeroTotalChunks));
    }

    #[test]
    fn total_too_large_error() {
        let mut buf = ReassemblyBuffer::new();
        let err = buf
            .receive_chunk(1, 0, 1000, Bytes::from_static(&[1, 2, 3]))
            .unwrap_err();
        assert!(matches!(err, StreamError::TotalChunksTooLarge { .. }));
    }

    #[test]
    fn total_mismatch_error() {
        let mut buf = ReassemblyBuffer::new();
        buf.receive_chunk(1, 0, 3, Bytes::from_static(&[1, 2, 3]))
            .unwrap();
        let err = buf
            .receive_chunk(1, 1, 5, Bytes::from_static(&[4, 5, 6]))
            .unwrap_err();
        assert!(matches!(err, StreamError::TotalChunksMismatch { .. }));
    }

    #[test]
    fn duplicate_chunk_error() {
        let mut buf = ReassemblyBuffer::new();
        buf.receive_chunk(1, 0, 3, Bytes::from_static(&[1, 2, 3]))
            .unwrap();
        let err = buf
            .receive_chunk(1, 0, 3, Bytes::from_static(&[4, 5, 6]))
            .unwrap_err();
        assert!(matches!(
            err,
            StreamError::DuplicateChunk {
                stream_id: 1,
                index: 0
            }
        ));
    }

    #[test]
    fn index_out_of_range_error() {
        let mut buf = ReassemblyBuffer::new();
        let err = buf
            .receive_chunk(1, 5, 3, Bytes::from_static(&[1, 2, 3]))
            .unwrap_err();
        assert!(matches!(
            err,
            StreamError::IndexOutOfRange {
                index: 5,
                total: 3,
                ..
            }
        ));
    }

    #[test]
    fn too_many_concurrent_streams_error() {
        let mut buf = ReassemblyBuffer::new();
        for i in 0..MAX_CONCURRENT_STREAMS as u32 {
            buf.receive_chunk(i, 0, 2, Bytes::from_static(&[1]))
                .unwrap();
        }
        let err = buf
            .receive_chunk(
                MAX_CONCURRENT_STREAMS as u32,
                0,
                2,
                Bytes::from_static(&[1]),
            )
            .unwrap_err();
        assert!(matches!(err, StreamError::TooManyConcurrentStreams { .. }));
    }

    #[test]
    fn reassembly_out_of_order() {
        let data: Vec<u8> = (0..CHUNK_SIZE * 3).map(|i| (i % 256) as u8).collect();
        let chunks = chunk_request(data.clone(), 10);
        assert_eq!(chunks.len(), 3);

        let mut buf = ReassemblyBuffer::new();
        // Feed in reverse order: 2, 0, 1
        let indices = [2, 0, 1];
        let mut result = None;
        for &i in &indices {
            if let ClientRequest::StreamChunk {
                stream_id,
                index,
                total,
                data: chunk_data,
            } = &chunks[i]
            {
                if let Some(r) = buf
                    .receive_chunk(*stream_id, *index, *total, chunk_data.clone())
                    .unwrap()
                {
                    result = Some(r);
                }
            }
        }
        assert_eq!(result.unwrap(), data);
    }

    #[test]
    fn chunk_exact_boundary() {
        // Exactly one chunk
        let data = vec![0xEE; CHUNK_SIZE];
        let chunks = chunk_request(data, 5);
        assert_eq!(chunks.len(), 1);

        // Exactly two chunks
        let data2 = vec![0xEE; CHUNK_SIZE * 2];
        let chunks2 = chunk_request(data2, 6);
        assert_eq!(chunks2.len(), 2);

        // One byte over two chunks
        let data3 = vec![0xEE; CHUNK_SIZE * 2 + 1];
        let chunks3 = chunk_request(data3, 7);
        assert_eq!(chunks3.len(), 3);
    }

    #[test]
    fn remove_stream_cleans_up() {
        let mut buf = ReassemblyBuffer::new();
        buf.receive_chunk(1, 0, 3, Bytes::from_static(&[1, 2, 3]))
            .unwrap();
        assert!(buf.remove_stream(1));
        assert!(!buf.remove_stream(1)); // already removed

        // Can start a new stream with the same id
        buf.receive_chunk(1, 0, 2, Bytes::from_static(&[4, 5]))
            .unwrap();
    }

    #[cfg(all(feature = "net", not(target_family = "wasm")))]
    mod stream_handle_tests {
        use super::super::*;
        use crate::client_api::client_events::StreamContent;
        use crate::prelude::{ContractCode, Parameters};
        use futures::StreamExt;

        #[tokio::test]
        async fn ws_stream_assemble() {
            let code = ContractCode::from(vec![1, 2, 3]);
            let key =
                crate::prelude::ContractKey::from_params_and_code(Parameters::from(vec![]), &code);
            let content = StreamContent::GetResponse {
                key,
                includes_contract: false,
            };
            let data = Bytes::from(vec![0xAB; CHUNK_SIZE * 3]);
            let (handle, sender) = ws_stream_pair(content, data.len() as u64);

            // Feed chunks in a background task
            let data_clone = data.clone();
            tokio::spawn(async move {
                for chunk in data_clone.chunks(CHUNK_SIZE) {
                    sender.send_chunk(Bytes::copy_from_slice(chunk)).unwrap();
                }
                // sender dropped here → handle's rx will close
            });

            let result = handle.assemble().await.unwrap();
            assert_eq!(result, &data[..]);
        }

        #[tokio::test]
        async fn ws_stream_incremental() {
            let content = StreamContent::Raw;
            let data = Bytes::from(vec![0xCD; CHUNK_SIZE * 2]);
            let (handle, sender) = ws_stream_pair(content, data.len() as u64);

            let data_clone = data.clone();
            tokio::spawn(async move {
                for chunk in data_clone.chunks(CHUNK_SIZE) {
                    sender.send_chunk(Bytes::copy_from_slice(chunk)).unwrap();
                }
            });

            let mut stream = handle.into_stream();
            let mut collected = Vec::new();
            while let Some(chunk) = stream.next().await {
                collected.extend_from_slice(&chunk);
            }
            assert_eq!(collected.len(), CHUNK_SIZE * 2);
            assert!(collected.iter().all(|&b| b == 0xCD));
        }

        #[tokio::test]
        async fn ws_stream_assemble_truncated() {
            let content = StreamContent::Raw;
            let (handle, sender) = ws_stream_pair(content, 1000);
            // Send less than promised, then drop sender
            sender.send_chunk(Bytes::from(vec![0; 100])).unwrap();
            drop(sender);
            let err = handle.assemble().await.unwrap_err();
            assert!(matches!(
                err,
                StreamError::Truncated {
                    received: 100,
                    expected: 1000
                }
            ));
        }

        #[tokio::test]
        async fn ws_stream_assemble_overflow() {
            let content = StreamContent::Raw;
            // Claim only 10 bytes
            let (handle, sender) = ws_stream_pair(content, 10);
            // Send way more than promised (over total_bytes + CHUNK_SIZE)
            let overflow_size = 10 + CHUNK_SIZE + 1;
            tokio::spawn(async move {
                sender
                    .send_chunk(Bytes::from(vec![0xFF; overflow_size]))
                    .unwrap();
            });
            let err = handle.assemble().await.unwrap_err();
            assert!(matches!(err, StreamError::Overflow { .. }));
        }
    }
}
