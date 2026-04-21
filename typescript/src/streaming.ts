/** Default chunk payload size: 256 KiB (matches Rust CHUNK_SIZE). */
export const CHUNK_SIZE = 256 * 1024;

/** Messages larger than this are chunked (matches Rust CHUNK_THRESHOLD). */
export const CHUNK_THRESHOLD = 512 * 1024;

/** Maximum total_chunks accepted from the wire. */
export const MAX_TOTAL_CHUNKS = 256;

/** Maximum concurrent streams in a single ReassemblyBuffer. */
export const MAX_CONCURRENT_STREAMS = 8;

export class StreamError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "StreamError";
  }
}

interface StreamState {
  chunks: (Uint8Array | null)[];
  total: number;
  received: number;
  createdAt: number;
}

/** Stream timeout in milliseconds (60 seconds, matches Rust STREAM_TTL). */
const STREAM_TTL_MS = 60_000;

/**
 * Reassembly buffer keyed by stream ID. Supports concurrent streams.
 * Port of Rust's ReassemblyBuffer from rust/src/client_api/streaming.rs.
 */
export class ReassemblyBuffer {
  private streams = new Map<number, StreamState>();

  /**
   * Feed a chunk into the buffer. Returns the fully reassembled payload
   * when all chunks for a stream have arrived, or null if more chunks needed.
   */
  receiveChunk(
    streamId: number,
    index: number,
    total: number,
    data: Uint8Array
  ): Uint8Array | null {
    if (total === 0) {
      throw new StreamError("total_chunks is zero");
    }
    if (total > MAX_TOTAL_CHUNKS) {
      throw new StreamError(
        `total_chunks ${total} exceeds maximum ${MAX_TOTAL_CHUNKS}`
      );
    }
    if (index >= total) {
      throw new StreamError(
        `chunk index ${index} out of range for stream ${streamId} (total ${total})`
      );
    }

    this.evictStale();

    if (
      !this.streams.has(streamId) &&
      this.streams.size >= MAX_CONCURRENT_STREAMS
    ) {
      throw new StreamError(
        `too many concurrent streams (${this.streams.size}), maximum is ${MAX_CONCURRENT_STREAMS}`
      );
    }

    let state = this.streams.get(streamId);
    if (!state) {
      state = {
        chunks: new Array(total).fill(null),
        total,
        received: 0,
        createdAt: Date.now(),
      };
      this.streams.set(streamId, state);
    }

    if (state.total !== total) {
      throw new StreamError(
        `total_chunks mismatch for stream ${streamId} (expected ${state.total}, got ${total})`
      );
    }

    if (state.chunks[index] !== null) {
      throw new StreamError(
        `duplicate chunk index ${index} for stream ${streamId}`
      );
    }

    state.chunks[index] = data;
    state.received += 1;

    if (state.received === state.total) {
      this.streams.delete(streamId);
      let totalLen = 0;
      for (const chunk of state.chunks) {
        totalLen += chunk!.length;
      }
      const result = new Uint8Array(totalLen);
      let offset = 0;
      for (const chunk of state.chunks) {
        result.set(chunk!, offset);
        offset += chunk!.length;
      }
      return result;
    }

    return null;
  }

  /** Remove a stream by ID, returning true if it existed. */
  removeStream(streamId: number): boolean {
    return this.streams.delete(streamId);
  }

  private evictStale(): void {
    const now = Date.now();
    for (const [id, state] of this.streams) {
      if (now - state.createdAt > STREAM_TTL_MS) {
        this.streams.delete(id);
      }
    }
  }
}
