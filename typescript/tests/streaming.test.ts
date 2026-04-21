import {
  ReassemblyBuffer,
  MAX_TOTAL_CHUNKS,
  MAX_CONCURRENT_STREAMS,
} from "../src/streaming";

describe("ReassemblyBuffer", () => {
  test("single chunk completes immediately", () => {
    const buf = new ReassemblyBuffer();
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const result = buf.receiveChunk(1, 0, 1, data);
    expect(result).not.toBeNull();
    expect(result!).toEqual(data);
  });

  test("multi-chunk reassembly in order", () => {
    const buf = new ReassemblyBuffer();
    expect(buf.receiveChunk(1, 0, 3, new Uint8Array([1, 2, 3]))).toBeNull();
    expect(buf.receiveChunk(1, 1, 3, new Uint8Array([4, 5, 6]))).toBeNull();
    const result = buf.receiveChunk(1, 2, 3, new Uint8Array([7, 8, 9]));
    expect(result).toEqual(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9]));
  });

  test("multi-chunk reassembly out of order", () => {
    const buf = new ReassemblyBuffer();
    expect(buf.receiveChunk(1, 2, 3, new Uint8Array([5, 6]))).toBeNull();
    expect(buf.receiveChunk(1, 0, 3, new Uint8Array([1, 2]))).toBeNull();
    const result = buf.receiveChunk(1, 1, 3, new Uint8Array([3, 4]));
    expect(result).toEqual(new Uint8Array([1, 2, 3, 4, 5, 6]));
  });

  test("throws on zero total chunks", () => {
    const buf = new ReassemblyBuffer();
    expect(() => buf.receiveChunk(1, 0, 0, new Uint8Array([1]))).toThrow(
      "total_chunks is zero"
    );
  });

  test("throws on total chunks too large", () => {
    const buf = new ReassemblyBuffer();
    expect(() =>
      buf.receiveChunk(1, 0, MAX_TOTAL_CHUNKS + 1, new Uint8Array([1]))
    ).toThrow("exceeds maximum");
  });

  test("throws on index out of range", () => {
    const buf = new ReassemblyBuffer();
    expect(() => buf.receiveChunk(1, 5, 3, new Uint8Array([1]))).toThrow(
      "out of range"
    );
  });

  test("throws on duplicate chunk", () => {
    const buf = new ReassemblyBuffer();
    buf.receiveChunk(1, 0, 3, new Uint8Array([1]));
    expect(() => buf.receiveChunk(1, 0, 3, new Uint8Array([2]))).toThrow(
      "duplicate"
    );
  });

  test("throws on total mismatch", () => {
    const buf = new ReassemblyBuffer();
    buf.receiveChunk(1, 0, 3, new Uint8Array([1]));
    expect(() => buf.receiveChunk(1, 1, 5, new Uint8Array([2]))).toThrow(
      "mismatch"
    );
  });

  test("throws on too many concurrent streams", () => {
    const buf = new ReassemblyBuffer();
    for (let i = 0; i < MAX_CONCURRENT_STREAMS; i++) {
      buf.receiveChunk(i, 0, 2, new Uint8Array([1]));
    }
    expect(() =>
      buf.receiveChunk(MAX_CONCURRENT_STREAMS, 0, 2, new Uint8Array([1]))
    ).toThrow("too many concurrent");
  });

  test("removeStream cleans up and allows reuse", () => {
    const buf = new ReassemblyBuffer();
    buf.receiveChunk(1, 0, 3, new Uint8Array([1]));
    expect(buf.removeStream(1)).toBe(true);
    expect(buf.removeStream(1)).toBe(false);
    buf.receiveChunk(1, 0, 2, new Uint8Array([4]));
  });

  test("completed stream is automatically cleaned up", () => {
    const buf = new ReassemblyBuffer();
    buf.receiveChunk(1, 0, 1, new Uint8Array([1]));
    expect(buf.removeStream(1)).toBe(false);
  });
});
