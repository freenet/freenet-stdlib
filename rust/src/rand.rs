//! Random number generation.

/// Get the specified number of random bytes.
pub fn rand_bytes<'a>(number: u32) -> &'a [u8] {
    // usually when some random bytes may be needed
    // no more than a 512 cryptographic key
    const MAX_KEY_SIZE: u32 = 512;
    if number <= MAX_KEY_SIZE {
        static mut BUF: [u8; MAX_KEY_SIZE as usize] = [0u8; MAX_KEY_SIZE as usize];
        // Safety: this should be fine to do as long as is called within a single threaded context,
        // which should be the case since the WASM instance runs in a single thread.
        unsafe {
            __frnt__rand__rand_bytes(crate::global::INSTANCE_ID, BUF.as_mut_ptr() as _, number);
            BUF.as_slice()
        }
    } else {
        let len = number as usize;
        static mut BUF: std::cell::OnceCell<Vec<u8>> = std::cell::OnceCell::new();
        // Safety: this should be fine to do as long as is called within a single threaded context,
        // which should be the case since the WASM instance runs in a single thread.
        unsafe {
            BUF.get_or_init(|| Vec::with_capacity(len));
            let buf_borrow = BUF.get_mut().unwrap();
            if buf_borrow.len() < len {
                buf_borrow.extend(std::iter::repeat(0).take(len - buf_borrow.len()));
            }
            __frnt__rand__rand_bytes(
                crate::global::INSTANCE_ID,
                buf_borrow.as_mut_ptr() as _,
                number,
            );
            BUF.get().unwrap().as_slice()
        }
    }
}

#[link(wasm_import_module = "freenet_rand")]
extern "C" {
    #[doc(hidden)]
    fn __frnt__rand__rand_bytes(id: i64, ptr: i64, len: u32);
}
