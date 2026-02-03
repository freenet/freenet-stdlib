//! Host function API for delegates.
//!
//! This module provides synchronous access to delegate context and secrets
//! via host functions, eliminating the need for message round-trips.
//!
//! # Example
//!
//! ```ignore
//! use freenet_stdlib::prelude::*;
//!
//! #[delegate]
//! impl DelegateInterface for MyDelegate {
//!     fn process(
//!         ctx: &mut DelegateCtx,
//!         secrets: &mut SecretsStore,
//!         _params: Parameters<'static>,
//!         _attested: Option<&'static [u8]>,
//!         message: InboundDelegateMsg,
//!     ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
//!         // Read/write context directly
//!         let data = ctx.read();
//!         ctx.write(b"new state");
//!
//!         // Access secrets synchronously
//!         if let Some(key) = secrets.get(b"private_key") {
//!             // use key...
//!         }
//!         secrets.set(b"new_secret", b"value");
//!
//!         Ok(vec![])
//!     }
//! }
//! ```

// ============================================================================
// Host function declarations (WASM only)
// ============================================================================

#[cfg(target_family = "wasm")]
#[link(wasm_import_module = "freenet_delegate_ctx")]
extern "C" {
    /// Returns the current context length in bytes.
    fn __frnt__delegate__ctx_len() -> i32;
    /// Reads context into the buffer at `ptr` (max `len` bytes). Returns bytes written.
    fn __frnt__delegate__ctx_read(ptr: i64, len: i32) -> i32;
    /// Writes `len` bytes from `ptr` into the context, replacing existing content.
    fn __frnt__delegate__ctx_write(ptr: i64, len: i32);
}

#[cfg(target_family = "wasm")]
#[link(wasm_import_module = "freenet_delegate_secrets")]
extern "C" {
    /// Get a secret. Returns bytes written to `out_ptr`, or -1 if not found.
    fn __frnt__delegate__get_secret(key_ptr: i64, key_len: i32, out_ptr: i64, out_len: i32) -> i32;
    /// Store a secret. Returns 0 on success, -1 on error.
    fn __frnt__delegate__set_secret(key_ptr: i64, key_len: i32, val_ptr: i64, val_len: i32) -> i32;
    /// Check if a secret exists. Returns 1 if yes, 0 if no.
    fn __frnt__delegate__has_secret(key_ptr: i64, key_len: i32) -> i32;
    /// Remove a secret. Returns 0 on success, -1 if not found.
    fn __frnt__delegate__remove_secret(key_ptr: i64, key_len: i32) -> i32;
}

// ============================================================================
// DelegateCtx - Opaque handle to mutable context
// ============================================================================

/// Opaque handle to the delegate's mutable context.
///
/// Context persists across messages within a single `inbound_app_message` batch,
/// but is reset between separate runtime calls. Use this for temporary state
/// that needs to be shared across multiple messages in one batch.
///
/// For persistent state, use [`SecretsStore`] instead.
#[repr(transparent)]
pub struct DelegateCtx {
    _private: (),
}

impl DelegateCtx {
    /// Creates the context handle.
    ///
    /// # Safety
    ///
    /// This should only be called by macro-generated code when the runtime
    /// has set up the delegate execution environment.
    #[doc(hidden)]
    pub unsafe fn __new() -> Self {
        Self { _private: () }
    }

    /// Returns the current context length in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        #[cfg(target_family = "wasm")]
        {
            let len = unsafe { __frnt__delegate__ctx_len() };
            if len < 0 {
                0
            } else {
                len as usize
            }
        }
        #[cfg(not(target_family = "wasm"))]
        {
            0
        }
    }

    /// Returns `true` if the context is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read the current context bytes.
    ///
    /// Returns an empty `Vec` if no context has been written.
    pub fn read(&self) -> Vec<u8> {
        #[cfg(target_family = "wasm")]
        {
            let len = unsafe { __frnt__delegate__ctx_len() };
            if len <= 0 {
                return Vec::new();
            }
            let mut buf = vec![0u8; len as usize];
            let read = unsafe { __frnt__delegate__ctx_read(buf.as_mut_ptr() as i64, len) };
            buf.truncate(read.max(0) as usize);
            buf
        }
        #[cfg(not(target_family = "wasm"))]
        {
            Vec::new()
        }
    }

    /// Read context into a provided buffer.
    ///
    /// Returns the number of bytes actually read.
    pub fn read_into(&self, buf: &mut [u8]) -> usize {
        #[cfg(target_family = "wasm")]
        {
            let read =
                unsafe { __frnt__delegate__ctx_read(buf.as_mut_ptr() as i64, buf.len() as i32) };
            read.max(0) as usize
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = buf;
            0
        }
    }

    /// Write new context bytes, replacing any existing content.
    pub fn write(&mut self, data: &[u8]) {
        #[cfg(target_family = "wasm")]
        {
            unsafe {
                __frnt__delegate__ctx_write(data.as_ptr() as i64, data.len() as i32);
            }
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = data;
        }
    }

    /// Clear the context.
    #[inline]
    pub fn clear(&mut self) {
        self.write(&[]);
    }
}

impl std::fmt::Debug for DelegateCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelegateCtx")
            .field("len", &self.len())
            .finish()
    }
}

// ============================================================================
// SecretsStore - Opaque handle to secret storage
// ============================================================================

/// Maximum buffer size for reading secrets.
const SECRET_MAX_SIZE: usize = 64 * 1024; // 64 KB

/// Opaque handle to the delegate's secret store.
///
/// Secrets are persistent across all delegate invocations and are stored
/// securely by the runtime. Use this for sensitive data like private keys,
/// tokens, or other credentials.
///
/// Each delegate has its own isolated secret namespace - secrets from one
/// delegate cannot be accessed by another.
#[repr(transparent)]
pub struct SecretsStore {
    _private: (),
}

impl SecretsStore {
    /// Creates the secrets store handle.
    ///
    /// # Safety
    ///
    /// This should only be called by macro-generated code when the runtime
    /// has set up the delegate execution environment.
    #[doc(hidden)]
    pub unsafe fn __new() -> Self {
        Self { _private: () }
    }

    /// Get a secret by key.
    ///
    /// Returns `None` if the secret does not exist.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        #[cfg(target_family = "wasm")]
        {
            let mut out = vec![0u8; SECRET_MAX_SIZE];
            let result = unsafe {
                __frnt__delegate__get_secret(
                    key.as_ptr() as i64,
                    key.len() as i32,
                    out.as_mut_ptr() as i64,
                    out.len() as i32,
                )
            };
            if result < 0 {
                None
            } else {
                out.truncate(result as usize);
                Some(out)
            }
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = key;
            None
        }
    }

    /// Store a secret.
    ///
    /// Returns `true` on success, `false` on error.
    pub fn set(&mut self, key: &[u8], value: &[u8]) -> bool {
        #[cfg(target_family = "wasm")]
        {
            let result = unsafe {
                __frnt__delegate__set_secret(
                    key.as_ptr() as i64,
                    key.len() as i32,
                    value.as_ptr() as i64,
                    value.len() as i32,
                )
            };
            result == 0
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = (key, value);
            false
        }
    }

    /// Check if a secret exists.
    pub fn has(&self, key: &[u8]) -> bool {
        #[cfg(target_family = "wasm")]
        {
            let result =
                unsafe { __frnt__delegate__has_secret(key.as_ptr() as i64, key.len() as i32) };
            result == 1
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = key;
            false
        }
    }

    /// Remove a secret.
    ///
    /// Returns `true` if the secret was removed, `false` if it didn't exist.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        #[cfg(target_family = "wasm")]
        {
            let result =
                unsafe { __frnt__delegate__remove_secret(key.as_ptr() as i64, key.len() as i32) };
            result == 0
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = key;
            false
        }
    }
}

impl std::fmt::Debug for SecretsStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretsStore").finish_non_exhaustive()
    }
}
