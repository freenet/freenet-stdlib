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
//!         _params: Parameters<'static>,
//!         _attested: Option<&'static [u8]>,
//!         message: InboundDelegateMsg,
//!     ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
//!         // Read/write temporary context
//!         let data = ctx.read();
//!         ctx.write(b"new state");
//!
//!         // Access persistent secrets
//!         if let Some(key) = ctx.get_secret(b"private_key") {
//!             // use key...
//!         }
//!         ctx.set_secret(b"new_secret", b"value");
//!
//!         Ok(vec![])
//!     }
//! }
//! ```
//!
//! # Context vs Secrets
//!
//! - **Context** (`read`/`write`): Temporary state within a single message batch.
//!   Reset between separate runtime calls. Use for intermediate processing state.
//!
//! - **Secrets** (`get_secret`/`set_secret`): Persistent encrypted storage.
//!   Survives across all delegate invocations. Use for private keys, tokens, etc.
//!
//! # Error Codes
//!
//! Host functions return negative values to indicate errors:
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | Success |
//! | -1   | Called outside process() context |
//! | -2   | Secret not found |
//! | -3   | Storage operation failed |
//! | -4   | Invalid parameter (e.g., negative length) |
//! | -5   | Context too large (exceeds i32::MAX) |
//! | -6   | Buffer too small |
//!
//! The wrapper methods in [`DelegateCtx`] handle these error codes and present
//! a more ergonomic API.

/// Error codes returned by host functions.
///
/// Negative values indicate errors, non-negative values indicate success
/// (usually the number of bytes read/written).
pub mod error_codes {
    /// Operation succeeded.
    pub const SUCCESS: i32 = 0;
    /// Called outside of a process() context.
    pub const ERR_NOT_IN_PROCESS: i32 = -1;
    /// Secret not found.
    pub const ERR_SECRET_NOT_FOUND: i32 = -2;
    /// Storage operation failed.
    pub const ERR_STORAGE_FAILED: i32 = -3;
    /// Invalid parameter (e.g., negative length).
    pub const ERR_INVALID_PARAM: i32 = -4;
    /// Context too large (exceeds i32::MAX).
    pub const ERR_CONTEXT_TOO_LARGE: i32 = -5;
    /// Buffer too small to hold the data.
    pub const ERR_BUFFER_TOO_SMALL: i32 = -6;
}

// ============================================================================
// Host function declarations (WASM only)
// ============================================================================

#[cfg(target_family = "wasm")]
#[link(wasm_import_module = "freenet_delegate_ctx")]
extern "C" {
    /// Returns the current context length in bytes, or negative error code.
    fn __frnt__delegate__ctx_len() -> i32;
    /// Reads context into the buffer at `ptr` (max `len` bytes). Returns bytes written, or negative error code.
    fn __frnt__delegate__ctx_read(ptr: i64, len: i32) -> i32;
    /// Writes `len` bytes from `ptr` into the context, replacing existing content. Returns 0 on success, or negative error code.
    fn __frnt__delegate__ctx_write(ptr: i64, len: i32) -> i32;
}

#[cfg(target_family = "wasm")]
#[link(wasm_import_module = "freenet_delegate_secrets")]
extern "C" {
    /// Get a secret. Returns bytes written to `out_ptr`, or negative error code.
    fn __frnt__delegate__get_secret(key_ptr: i64, key_len: i32, out_ptr: i64, out_len: i32) -> i32;
    /// Get secret length without fetching value. Returns length, or negative error code.
    fn __frnt__delegate__get_secret_len(key_ptr: i64, key_len: i32) -> i32;
    /// Store a secret. Returns 0 on success, or negative error code.
    fn __frnt__delegate__set_secret(key_ptr: i64, key_len: i32, val_ptr: i64, val_len: i32) -> i32;
    /// Check if a secret exists. Returns 1 if yes, 0 if no, or negative error code.
    fn __frnt__delegate__has_secret(key_ptr: i64, key_len: i32) -> i32;
    /// Remove a secret. Returns 0 on success, or negative error code.
    fn __frnt__delegate__remove_secret(key_ptr: i64, key_len: i32) -> i32;
}

// ============================================================================
// DelegateCtx - Unified handle to context and secrets
// ============================================================================

/// Opaque handle to the delegate's execution environment.
///
/// Provides access to both:
/// - **Temporary context**: State shared within a single message batch (reset between calls)
/// - **Persistent secrets**: Encrypted storage that survives across all invocations
///
/// # Context Methods
/// - [`read`](Self::read), [`write`](Self::write), [`len`](Self::len), [`clear`](Self::clear)
///
/// # Secret Methods
/// - [`get_secret`](Self::get_secret), [`set_secret`](Self::set_secret),
///   [`has_secret`](Self::has_secret), [`remove_secret`](Self::remove_secret)
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

    // ========================================================================
    // Context methods (temporary state within a batch)
    // ========================================================================

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
    ///
    /// Returns `true` on success, `false` on error.
    pub fn write(&mut self, data: &[u8]) -> bool {
        #[cfg(target_family = "wasm")]
        {
            let result =
                unsafe { __frnt__delegate__ctx_write(data.as_ptr() as i64, data.len() as i32) };
            result == 0
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = data;
            false
        }
    }

    /// Clear the context.
    #[inline]
    pub fn clear(&mut self) {
        self.write(&[]);
    }

    // ========================================================================
    // Secret methods (persistent encrypted storage)
    // ========================================================================

    /// Get the length of a secret without retrieving its value.
    ///
    /// Returns `None` if the secret does not exist.
    pub fn get_secret_len(&self, key: &[u8]) -> Option<usize> {
        #[cfg(target_family = "wasm")]
        {
            let result =
                unsafe { __frnt__delegate__get_secret_len(key.as_ptr() as i64, key.len() as i32) };
            if result < 0 {
                None
            } else {
                Some(result as usize)
            }
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let _ = key;
            None
        }
    }

    /// Get a secret by key.
    ///
    /// Returns `None` if the secret does not exist.
    pub fn get_secret(&self, key: &[u8]) -> Option<Vec<u8>> {
        #[cfg(target_family = "wasm")]
        {
            // First get the length to allocate the right buffer size
            let len = self.get_secret_len(key)?;

            if len == 0 {
                return Some(Vec::new());
            }

            let mut out = vec![0u8; len];
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
    pub fn set_secret(&mut self, key: &[u8], value: &[u8]) -> bool {
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
    pub fn has_secret(&self, key: &[u8]) -> bool {
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
    pub fn remove_secret(&mut self, key: &[u8]) -> bool {
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

impl Default for DelegateCtx {
    fn default() -> Self {
        Self { _private: () }
    }
}

impl std::fmt::Debug for DelegateCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelegateCtx")
            .field("context_len", &self.len())
            .finish_non_exhaustive()
    }
}
