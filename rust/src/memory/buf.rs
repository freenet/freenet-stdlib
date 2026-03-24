//! Memory buffers to interact with the WASM contracts.

use super::WasmLinearMem;

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferBuilder {
    start: i64,
    capacity: u32,
    last_read: i64,
    last_write: i64,
}

impl BufferBuilder {
    /// Return the buffer capacity.
    pub fn capacity(&self) -> usize {
        self.capacity as _
    }

    /// Returns the number of bytes written to the buffer.
    #[cfg(not(feature = "contract"))]
    pub fn bytes_written(&self, mem: &WasmLinearMem) -> usize {
        unsafe {
            let ptr = compute_ptr(self.last_write as *mut u32, mem);
            *ptr as usize
        }
    }

    #[cfg(feature = "contract")]
    pub fn bytes_written(&self) -> usize {
        unsafe { *(self.last_write as *mut u32) as usize }
    }

    /// Returns the number of bytes read from the buffer.
    #[cfg(feature = "contract")]
    pub fn bytes_read(&self) -> usize {
        unsafe { *(self.last_read as *mut u32) as usize }
    }

    /// Resets the read and write pointers to 0 (contract-side).
    #[cfg(feature = "contract")]
    pub fn reset_pointers(&mut self) {
        unsafe {
            *(self.last_read as *mut u32) = 0;
            *(self.last_write as *mut u32) = 0;
        }
    }

    /// Returns the first byte of buffer.
    pub fn start(&self) -> *mut u8 {
        self.start as _
    }

    /// Returns the raw pointer to the read position tracker (for host-side reset).
    pub fn last_read_ptr(&self) -> *mut u32 {
        self.last_read as *mut u32
    }

    /// Returns the raw pointer to the write position tracker (for host-side reset).
    pub fn last_write_ptr(&self) -> *mut u32 {
        self.last_write as *mut u32
    }

    /// # Safety
    /// Requires that there are no living references to the current
    /// underlying buffer or will trigger UB
    pub unsafe fn update_buffer(&mut self, data: Vec<u8>) {
        let read_ptr = Box::leak(Box::from_raw(self.last_read as *mut u32));
        let write_ptr = Box::leak(Box::from_raw(self.last_write as *mut u32));

        // drop previous buffer
        let prev = Vec::from_raw_parts(self.start as *mut u8, *write_ptr as usize, self.capacity());
        std::mem::drop(prev);

        // write the new buffer information
        let new_ptr = data.as_ptr();
        self.start = new_ptr as i64;
        self.capacity = data.capacity() as _;
        *read_ptr = 0;
        *write_ptr = data.len().saturating_sub(1) as _; // []
        std::mem::forget(data);
    }

    /// Returns a wrapped raw pointer to the buffer builder.
    pub fn to_ptr(self) -> *mut BufferBuilder {
        Box::into_raw(Box::new(self))
    }
}

/// Type of buffer errors.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Insufficient memory while trying to write to the buffer.
    #[error("insufficient memory, needed {req} bytes but had {free} bytes")]
    InsufficientMemory {
        /// Required memory available
        req: usize,
        /// Available memory.
        free: usize,
    },
}

/// A live mutable buffer in the WASM linear memory.
#[derive(Debug)]
pub struct BufferMut<'instance> {
    buffer: &'instance mut [u8],
    /// stores the last read in the buffer
    read_ptr: &'instance u32,
    /// stores the last write in the buffer
    write_ptr: &'instance mut u32,
    /// A pointer to the underlying builder
    builder_ptr: *mut BufferBuilder,
    /// Linear memory pointer and size in bytes
    mem: WasmLinearMem,
}

impl<'instance> BufferMut<'instance> {
    /// Tries to write data into the buffer, after any unread bytes.
    ///
    /// Will return an error if there is insufficient space.
    pub fn write<T>(&mut self, obj: T) -> Result<(), Error>
    where
        T: AsRef<[u8]>,
    {
        let obj = obj.as_ref();
        if obj.len() > self.buffer.len() {
            return Err(Error::InsufficientMemory {
                req: obj.len(),
                free: self.buffer.len(),
            });
        }
        let mut last_write = (*self.write_ptr) as usize;
        let free_right = self.buffer.len() - last_write;
        if obj.len() <= free_right {
            let copy_to = &mut self.buffer[last_write..last_write + obj.len()];
            copy_to.copy_from_slice(obj);
            last_write += obj.len();
            *self.write_ptr = last_write as u32;
            Ok(())
        } else {
            Err(Error::InsufficientMemory {
                req: obj.len(),
                free: free_right,
            })
        }
    }

    /// Read bytes specified number of bytes from the buffer.
    ///
    /// Always reads from the beginning.
    pub fn read_bytes(&self, len: usize) -> &[u8] {
        let next_offset = *self.read_ptr as usize;
        // don't update the read ptr
        &self.buffer[next_offset..next_offset + len]
    }

    /// Give ownership of the buffer back to the guest.
    pub fn shared(self) -> Buffer<'instance> {
        let BufferMut {
            builder_ptr, mem, ..
        } = self;
        let BuilderInfo {
            buffer,
            read_ptr,
            write_ptr,
            ..
        } = from_raw_builder(builder_ptr, mem);
        Buffer {
            buffer,
            read_ptr,
            write_ptr,
            builder_ptr,
            mem,
        }
    }

    /// Return the buffer capacity.
    pub fn capacity(&self) -> usize {
        unsafe {
            let p = &*compute_ptr(self.builder_ptr, &self.mem);
            p.capacity as _
        }
    }

    /// # Safety
    /// The pointer passed come from a previous call to `initiate_buffer` exported function from the contract.
    pub unsafe fn from_ptr(
        builder_ptr: *mut BufferBuilder,
        linear_mem_space: WasmLinearMem,
    ) -> Self {
        let BuilderInfo {
            buffer,
            read_ptr,
            write_ptr,
        } = from_raw_builder(builder_ptr, linear_mem_space);
        BufferMut {
            buffer,
            read_ptr,
            write_ptr,
            builder_ptr,
            mem: linear_mem_space,
        }
    }

    /// A pointer to the linear memory address.
    pub fn ptr(&self) -> *mut BufferBuilder {
        self.builder_ptr
    }
}

impl std::io::Write for BufferMut<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let last_write = (*self.write_ptr) as usize;
        let free = self.buffer.len() - last_write;
        let n = buf.len().min(free);
        if n == 0 && !buf.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "buffer full",
            ));
        }
        self.buffer[last_write..last_write + n].copy_from_slice(&buf[..n]);
        *self.write_ptr = (last_write + n) as u32;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[inline(always)]
pub fn compute_ptr<T>(ptr: *mut T, linear_mem_space: &WasmLinearMem) -> *mut T {
    let mem_start_ptr = linear_mem_space.start_ptr;
    (mem_start_ptr as isize + ptr as isize) as _
}

struct BuilderInfo<'instance> {
    buffer: &'instance mut [u8],
    read_ptr: &'instance mut u32,
    write_ptr: &'instance mut u32,
}

fn from_raw_builder<'a>(builder_ptr: *mut BufferBuilder, mem: WasmLinearMem) -> BuilderInfo<'a> {
    unsafe {
        #[cfg(feature = "trace")]
        {
            if !mem.start_ptr.is_null() && mem.size > 0 {
                let contract_mem = std::slice::from_raw_parts(mem.start_ptr, mem.size as usize);
                tracing::trace!(
                    "*mut BufferBuilder <- offset: {}; in mem: {:?}",
                    builder_ptr as usize,
                    &contract_mem[builder_ptr as usize
                        ..builder_ptr as usize + std::mem::size_of::<BufferBuilder>()]
                );
            }
            // use std::{fs::File, io::Write};
            // let mut f = File::create(std::env::temp_dir().join("dump.mem")).unwrap();
            // f.write_all(contract_mem).unwrap();
        }

        let builder_ptr = compute_ptr(builder_ptr, &mem);
        let buf_builder: &'static mut BufferBuilder = Box::leak(Box::from_raw(builder_ptr));
        #[cfg(feature = "trace")]
        {
            tracing::trace!("buf builder from FFI: {buf_builder:?}");
        }

        let read_ptr = Box::leak(Box::from_raw(compute_ptr(
            buf_builder.last_read as *mut u32,
            &mem,
        )));
        let write_ptr = Box::leak(Box::from_raw(compute_ptr(
            buf_builder.last_write as *mut u32,
            &mem,
        )));
        let buffer_ptr = compute_ptr(buf_builder.start as *mut u8, &mem);
        let buffer =
            &mut *std::ptr::slice_from_raw_parts_mut(buffer_ptr, buf_builder.capacity as usize);
        BuilderInfo {
            buffer,
            read_ptr,
            write_ptr,
        }
    }
}

#[derive(Debug)]
/// A live buffer in the WASM linear memory.
pub struct Buffer<'instance> {
    buffer: &'instance mut [u8],
    /// stores the last read in the buffer
    read_ptr: &'instance mut u32,
    write_ptr: &'instance u32,
    builder_ptr: *mut BufferBuilder,
    mem: WasmLinearMem,
}

impl<'instance> Buffer<'instance> {
    /// # Safety
    /// In order for this to be a safe T must be properly aligned and cannot re-use the buffer
    /// trying to read the same memory region again (that would create more than one copy to
    /// the same underlying data and break aliasing rules).
    pub unsafe fn read<T: Sized>(&mut self) -> T {
        let next_offset = *self.read_ptr as usize;
        let bytes = &self.buffer[next_offset..next_offset + std::mem::size_of::<T>()];
        let t = std::ptr::read(bytes.as_ptr() as *const T);
        *self.read_ptr += std::mem::size_of::<T>() as u32;
        t
    }

    /// Read the specified number of bytes from the buffer.
    pub fn read_bytes(&mut self, len: usize) -> &[u8] {
        let next_offset = *self.read_ptr as usize;
        *self.read_ptr += len as u32;
        &self.buffer[next_offset..next_offset + len]
    }

    /// Reads all the bytes from the buffer.
    pub fn read_all(&mut self) -> &[u8] {
        let next_offset = *self.read_ptr as usize;
        *self.read_ptr += self.buffer.len() as u32;
        &self.buffer[next_offset..=*self.write_ptr as usize]
    }

    /// Give ownership of the buffer back to the guest.
    ///
    /// # Safety
    /// Must guarantee that there are not underlying alive shared references.
    #[doc(hidden)]
    pub unsafe fn exclusive(self) -> BufferMut<'instance> {
        let Buffer {
            builder_ptr, mem, ..
        } = self;
        let BuilderInfo {
            buffer,
            read_ptr,
            write_ptr,
        } = from_raw_builder(builder_ptr, mem);
        BufferMut {
            buffer,
            read_ptr,
            write_ptr,
            builder_ptr,
            mem,
        }
    }
}

// ---------------------------------------------------------------------------
// Streaming refill buffer (contract-side only)
// ---------------------------------------------------------------------------

// Host import for refilling a buffer. Called by the contract when it has
// exhausted the current buffer contents and needs more data from the host.
// Returns the number of bytes the host wrote into the buffer, or 0 for EOF.
#[cfg(all(feature = "contract", not(test)))]
#[link(wasm_import_module = "freenet_contract_io")]
extern "C" {
    fn __frnt__fill_buffer(id: i64, buf_ptr: i64) -> u32;
}

// Test stub: returns 0 (EOF) since tests don't have a WASM host.
// This means tests can only exercise the non-refill path.
#[cfg(all(feature = "contract", test))]
unsafe extern "C" fn __frnt__fill_buffer(_id: i64, _buf_ptr: i64) -> u32 {
    0
}

/// Contract-side streaming reader for refill-pattern buffers.
///
/// The host writes a `[total_len: u32]` header followed by as much data as
/// fits into a small buffer. The contract reads through this wrapper; when
/// the buffer is exhausted, [`Read::read`] calls the host to refill it.
#[cfg(feature = "contract")]
pub struct StreamingBuffer {
    buf_ptr: *mut BufferBuilder,
    /// Total bytes remaining to be read (initialized from the header).
    total_remaining: usize,
}

#[cfg(feature = "contract")]
impl StreamingBuffer {
    /// Create a streaming reader from a buffer pointer.
    ///
    /// Reads the `[total_len: u32]` header and prepares for streaming.
    ///
    /// # Safety
    /// `ptr` must point to a valid `BufferBuilder` in WASM linear memory
    /// whose first 4 bytes of data contain the total payload length as LE u32.
    /// Returns the total number of payload bytes remaining to be read.
    pub fn total_remaining(&self) -> usize {
        self.total_remaining
    }

    pub unsafe fn from_ptr(ptr: i64) -> Self {
        let buf_ptr = ptr as *mut BufferBuilder;
        let builder = &*buf_ptr;
        // Read the total_len header (first 4 bytes)
        let data_start = builder.start() as *const u8;
        let total_len = u32::from_le_bytes([
            *data_start,
            *data_start.add(1),
            *data_start.add(2),
            *data_start.add(3),
        ]) as usize;
        // Advance the read pointer past the header
        let read_ptr = builder.last_read as *mut u32;
        *read_ptr = 4;
        StreamingBuffer {
            buf_ptr,
            total_remaining: total_len,
        }
    }
}

#[cfg(feature = "contract")]
impl std::io::Read for StreamingBuffer {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.total_remaining == 0 {
            return Ok(0); // EOF — all expected data has been read
        }
        let builder = unsafe { &*self.buf_ptr };
        let mut available = builder.bytes_written().saturating_sub(builder.bytes_read());
        if available == 0 {
            // Buffer exhausted — ask host to refill
            let filled =
                unsafe { __frnt__fill_buffer(crate::global::INSTANCE_ID, self.buf_ptr as i64) };
            if filled == 0 {
                return Ok(0); // Host says EOF
            }
            available = filled as usize;
        }
        let n = out.len().min(available).min(self.total_remaining);
        // Copy from buffer at current read position
        let read_pos = builder.bytes_read();
        unsafe {
            let src = builder.start().add(read_pos);
            std::ptr::copy_nonoverlapping(src, out.as_mut_ptr(), n);
            // Advance the read pointer
            *(builder.last_read as *mut u32) = (read_pos + n) as u32;
        }
        self.total_remaining -= n;
        Ok(n)
    }
}

/// Returns the pointer to a new BufferBuilder.
///
/// This buffer leaks it's own memory and will only be freed by the runtime when a contract instance is dropped.
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
#[cfg(any(feature = "contract", test))]
fn __frnt__initiate_buffer(capacity: u32) -> i64 {
    let buf: Vec<u8> = Vec::with_capacity(capacity as usize);
    let start = buf.as_ptr() as i64;

    let last_read = Box::into_raw(Box::new(0u32));
    let last_write = Box::into_raw(Box::new(0u32));
    let buffer = Box::into_raw(Box::new(BufferBuilder {
        start,
        capacity,
        last_read: last_read as _,
        last_write: last_write as _,
    }));
    #[cfg(feature = "trace")]
    {
        tracing::trace!(
            "new buffer ptr: {:p} -> {} as i64 w/ cap: {capacity}",
            buf.as_ptr(),
            start
        );
        tracing::trace!(
            "last read ptr: {last_read:p} -> {} as i64",
            last_read as i64
        );
        tracing::trace!(
            "last write ptr: {last_write:p} -> {} as i64",
            last_write as i64
        );
        tracing::trace!("buffer ptr: {buffer:p} -> {} as i64", buffer as i64);
    }
    std::mem::forget(buf);
    buffer as i64
}

#[cfg(test)]
mod test_io_write {
    use super::*;
    use std::io::Write;

    /// Create a BufferMut backed by host memory (no WASM runtime needed).
    /// Uses `__frnt__initiate_buffer` which allocates in host memory during tests,
    /// and a null-base WasmLinearMem so compute_ptr is a no-op on absolute pointers.
    unsafe fn host_buffer_mut(capacity: u32) -> BufferMut<'static> {
        let builder_ptr = __frnt__initiate_buffer(capacity) as *mut BufferBuilder;
        let linear_mem = WasmLinearMem {
            start_ptr: std::ptr::null(),
            size: 0,
        };
        BufferMut::from_ptr(builder_ptr, linear_mem)
    }

    /// Call std::io::Write::write (not BufferMut::write which has different signature)
    fn io_write(buf: &mut BufferMut<'_>, data: &[u8]) -> std::io::Result<usize> {
        Write::write(buf, data)
    }

    #[test]
    fn write_trait_basic() {
        let mut buf = unsafe { host_buffer_mut(32) };
        let n = io_write(&mut buf, b"hello").unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf.read_bytes(5), b"hello");
    }

    #[test]
    fn write_trait_fills_exactly() {
        let mut buf = unsafe { host_buffer_mut(4) };
        let n = io_write(&mut buf, b"abcd").unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf.read_bytes(4), b"abcd");
    }

    #[test]
    fn write_trait_partial_when_near_full() {
        let mut buf = unsafe { host_buffer_mut(4) };
        io_write(&mut buf, b"ab").unwrap();
        // Only 2 bytes free, writing 3 should write 2
        let n = io_write(&mut buf, b"xyz").unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf.read_bytes(4), b"abxy");
    }

    #[test]
    fn write_trait_error_when_full() {
        let mut buf = unsafe { host_buffer_mut(2) };
        io_write(&mut buf, b"ab").unwrap();
        let err = io_write(&mut buf, b"c").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WriteZero);
    }

    #[test]
    fn write_trait_empty_slice_ok() {
        let mut buf = unsafe { host_buffer_mut(4) };
        let n = io_write(&mut buf, b"").unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn write_all_trait() {
        let mut buf = unsafe { host_buffer_mut(16) };
        buf.write_all(b"hello world").unwrap();
        assert_eq!(buf.read_bytes(11), b"hello world");
    }

    #[test]
    fn write_all_insufficient_space() {
        let mut buf = unsafe { host_buffer_mut(4) };
        let err = buf.write_all(b"hello").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WriteZero);
    }

    #[test]
    fn bincode_serialize_into() {
        let data: Vec<u32> = vec![1, 2, 3, 4, 5];
        let size = bincode::serialized_size(&data).unwrap() as usize;
        let mut buf = unsafe { host_buffer_mut(size as u32) };
        bincode::serialize_into(&mut buf, &data).unwrap();
        let result: Vec<u32> = bincode::deserialize(buf.read_bytes(size)).unwrap();
        assert_eq!(result, data);
    }
}

/// Tests for StreamingBuffer (contract-side reader).
/// These test the non-refill path only — when all data fits in the initial buffer.
/// The refill path requires a WASM runtime and is tested via integration tests.
#[cfg(all(test, feature = "contract"))]
mod test_streaming_read {
    use super::*;
    use std::io::Read;

    /// Create a host-memory buffer pre-loaded with `[total_len: u32 LE][data...]`.
    unsafe fn host_streaming_buffer(data: &[u8]) -> StreamingBuffer {
        let total_with_header = data.len() + 4;
        let ptr = __frnt__initiate_buffer(total_with_header as u32);
        let builder = &mut *(ptr as *mut BufferBuilder);

        // Write total_len header (LE u32)
        let header = (data.len() as u32).to_le_bytes();
        let start = builder.start();
        std::ptr::copy_nonoverlapping(header.as_ptr(), start, 4);
        std::ptr::copy_nonoverlapping(data.as_ptr(), start.add(4), data.len());

        // Set write pointer to total bytes written
        *(builder.last_write as *mut u32) = total_with_header as u32;

        StreamingBuffer::from_ptr(ptr)
    }

    #[test]
    fn read_basic() {
        let data = b"hello streaming";
        let mut reader = unsafe { host_streaming_buffer(data) };
        let mut out = vec![0u8; data.len()];
        reader.read_exact(&mut out).unwrap();
        assert_eq!(&out, data);
    }

    #[test]
    fn read_to_end_collects_all() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut reader = unsafe { host_streaming_buffer(data) };
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        assert_eq!(&out, data);
    }

    #[test]
    fn read_empty_payload() {
        let mut reader = unsafe { host_streaming_buffer(b"") };
        let mut out = Vec::new();
        let n = reader.read_to_end(&mut out).unwrap();
        assert_eq!(n, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn read_in_small_chunks() {
        let data = b"abcdefghij";
        let mut reader = unsafe { host_streaming_buffer(data) };
        let mut result = Vec::new();
        let mut buf = [0u8; 3];
        loop {
            let n = reader.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            result.extend_from_slice(&buf[..n]);
        }
        assert_eq!(&result, data);
    }

    #[test]
    fn total_remaining_decreases() {
        let data = b"1234567890";
        let mut reader = unsafe { host_streaming_buffer(data) };
        assert_eq!(reader.total_remaining(), 10);
        let mut buf = [0u8; 4];
        reader.read(&mut buf).unwrap();
        assert_eq!(reader.total_remaining(), 6);
    }

    #[test]
    fn eof_after_all_read() {
        let data = b"abc";
        let mut reader = unsafe { host_streaming_buffer(data) };
        let mut out = vec![0u8; 3];
        reader.read_exact(&mut out).unwrap();
        assert_eq!(reader.total_remaining(), 0);
        let n = reader.read(&mut out).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn bincode_roundtrip_through_streaming() {
        let original: Vec<u32> = vec![42, 99, 1337, 0, u32::MAX];
        let serialized = bincode::serialize(&original).unwrap();
        let mut reader = unsafe { host_streaming_buffer(&serialized) };
        let mut bytes = Vec::with_capacity(reader.total_remaining());
        reader.read_to_end(&mut bytes).unwrap();
        let result: Vec<u32> = bincode::deserialize(&bytes).unwrap();
        assert_eq!(result, original);
    }
}

#[cfg(all(test, any(unix, windows), feature = "wasmer-tests"))]
mod test {
    use super::*;
    use wasmer::{
        imports, wat2wasm, AsStoreMut, Cranelift, Function, Instance, Module, Store, TypedFunction,
    };

    const TEST_MODULE: &str = r#"
        (module
            (func $initiate_buffer (import "freenet" "initiate_buffer") (param i32) (result i64))
            (memory $locutus_mem (export "memory") 20)
            (export "initiate_buffer" (func $initiate_buffer))
        )"#;

    fn build_test_mod() -> Result<(Store, Instance), Box<dyn std::error::Error>> {
        let wasm_bytes = wat2wasm(TEST_MODULE.as_bytes())?;
        let mut store = Store::new(Cranelift::new());
        let module = Module::new(&store, wasm_bytes)?;

        let init_buf_fn = Function::new_typed(&mut store, __frnt__initiate_buffer);
        let imports = imports! {
            "freenet" => { "initiate_buffer" => init_buf_fn }
        };
        let instance = Instance::new(&mut store, &module, &imports).unwrap();
        Ok((store, instance))
    }

    fn init_buf(store: &mut impl AsStoreMut, instance: &Instance, size: u32) -> *mut BufferBuilder {
        let initiate_buffer: TypedFunction<u32, i64> = instance
            .exports
            .get_typed_function(&store, "initiate_buffer")
            .unwrap();
        initiate_buffer.call(store, size).unwrap() as *mut BufferBuilder
    }

    #[test]
    #[ignore]
    fn read_and_write() -> Result<(), Box<dyn std::error::Error>> {
        let (mut store, instance) = build_test_mod()?;
        let mem = instance.exports.get_memory("memory")?.view(&store);
        let linear_mem = WasmLinearMem {
            start_ptr: mem.data_ptr() as *const _,
            size: mem.data_size(),
        };

        let mut writer =
            unsafe { BufferMut::from_ptr(init_buf(&mut store, &instance, 10), linear_mem) };
        writer.write([1u8, 2])?;
        let mut reader = writer.shared();
        let r: [u8; 2] = unsafe { reader.read() };
        assert_eq!(r, [1, 2]);

        let mut writer = unsafe { reader.exclusive() };
        writer.write([3u8, 4])?;
        let mut reader = writer.shared();
        let r: [u8; 2] = unsafe { reader.read() };
        assert_eq!(r, [3, 4]);
        Ok(())
    }

    #[test]
    #[ignore]
    fn read_and_write_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let (mut store, instance) = build_test_mod()?;
        let mem = instance.exports.get_memory("memory")?.view(&store);
        let linear_mem = WasmLinearMem {
            start_ptr: mem.data_ptr() as *const _,
            size: mem.data_size(),
        };

        let mut writer =
            unsafe { BufferMut::from_ptr(init_buf(&mut store, &instance, 10), linear_mem) };
        writer.write([1u8, 2])?;
        let mut reader = writer.shared();
        let r = reader.read_bytes(2);
        assert_eq!(r, &[1, 2]);

        let mut writer = unsafe { reader.exclusive() };
        writer.write([3u8, 4])?;
        let mut reader = writer.shared();
        let r = reader.read_bytes(2);
        assert_eq!(r, &[3, 4]);
        Ok(())
    }

    #[test]
    #[ignore]
    fn update() -> Result<(), Box<dyn std::error::Error>> {
        let (mut store, instance) = build_test_mod()?;
        let mem = instance.exports.get_memory("memory")?.view(&store);
        let linear_mem = WasmLinearMem {
            start_ptr: mem.data_ptr() as *const _,
            size: mem.data_size(),
        };

        let ptr = {
            let mut writer =
                unsafe { BufferMut::from_ptr(init_buf(&mut store, &instance, 10), linear_mem) };
            writer.write([1u8, 2])?;
            writer.ptr()
        };

        let writer = unsafe {
            let builder = &mut *ptr;
            builder.update_buffer(vec![3, 5, 7]);
            BufferMut::from_ptr(ptr, linear_mem)
        };
        let mut reader = writer.shared();
        assert_eq!(reader.read_all(), &[3, 5, 7]);

        Ok(())
    }
}
