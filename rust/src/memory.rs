//! Internally used functionality to interact between WASM and the host environment.
//! Most of the usage of types is unsafe and requires knowledge on how
//! the WASM runtime is set and used. Use with caution.
//!
//! End users should be using higher levels of abstraction to write contracts
//! and shouldn't need to manipulate functions and types in this module directly.
//! Use with caution.

pub mod buf;

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct WasmLinearMem {
    pub start_ptr: *const u8,
    pub size: u64,
}

impl WasmLinearMem {
    /// # Safety
    /// Ensure that the passed pointer is a valid pointer to the start of
    /// the WASM linear memory
    pub unsafe fn new(start_ptr: *const u8, size: u64) -> Self {
        Self { start_ptr, size }
    }
}

#[cfg(feature = "contract")]
pub mod wasm_interface {
    use crate::prelude::*;

    fn set_logger() -> Result<(), ContractInterfaceResult> {
        #[cfg(feature = "trace")]
        {
            use crate::prelude::*;
            use tracing_subscriber as tra;
            if let Err(err) = tra::fmt()
                .with_env_filter("warn,freenet_stdlib=trace")
                .try_init()
            {
                return Err(ContractInterfaceResult::from(Err::<ValidateResult, _>(
                    ContractError::Other(format!("{}", err)),
                )));
            }
        }
        Ok(())
    }

    use std::io::Read;

    /// Read all bytes from a streaming buffer into a Vec.
    fn read_streaming_bytes(ptr: i64) -> Result<Vec<u8>, ContractInterfaceResult> {
        let mut reader = unsafe { super::buf::StreamingBuffer::from_ptr(ptr) };
        let mut bytes = Vec::with_capacity(reader.total_remaining());
        reader.read_to_end(&mut bytes).map_err(|e| {
            ContractInterfaceResult::from(Err::<ValidateResult, _>(ContractError::Other(format!(
                "streaming read failed: {e}"
            ))))
        })?;
        Ok(bytes)
    }

    pub fn inner_validate_state<T: ContractInterface>(
        parameters: i64,
        state: i64,
        related: i64,
    ) -> i64 {
        if let Err(e) = set_logger().map_err(|e| e.into_raw()) {
            return e;
        }
        let parameters = match read_streaming_bytes(parameters) {
            Ok(bytes) => Parameters::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let state = match read_streaming_bytes(state) {
            Ok(bytes) => State::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let related_bytes = match read_streaming_bytes(related) {
            Ok(bytes) => bytes,
            Err(e) => return e.into_raw(),
        };
        let related: RelatedContracts<'static> =
            match bincode::deserialize::<RelatedContracts>(&related_bytes) {
                Ok(v) => v.into_owned(),
                Err(err) => {
                    return ContractInterfaceResult::from(Err::<::core::primitive::bool, _>(
                        ContractError::Deser(format!("{}", err)),
                    ))
                    .into_raw()
                }
            };
        let result = <T as ContractInterface>::validate_state(parameters, state, related);
        ContractInterfaceResult::from(result).into_raw()
    }

    pub fn inner_update_state<T: ContractInterface>(
        parameters: i64,
        state: i64,
        updates: i64,
    ) -> i64 {
        if let Err(e) = set_logger().map_err(|e| e.into_raw()) {
            return e;
        }
        let parameters = match read_streaming_bytes(parameters) {
            Ok(bytes) => Parameters::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let state = match read_streaming_bytes(state) {
            Ok(bytes) => State::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let updates_bytes = match read_streaming_bytes(updates) {
            Ok(bytes) => bytes,
            Err(e) => return e.into_raw(),
        };
        let updates: Vec<UpdateData<'static>> =
            match bincode::deserialize::<Vec<UpdateData>>(&updates_bytes) {
                Ok(v) => v.into_iter().map(|u| u.into_owned()).collect(),
                Err(err) => {
                    return ContractInterfaceResult::from(Err::<ValidateResult, _>(
                        ContractError::Deser(format!("{}", err)),
                    ))
                    .into_raw()
                }
            };
        let result = <T as ContractInterface>::update_state(parameters, state, updates);
        ContractInterfaceResult::from(result).into_raw()
    }

    pub fn inner_summarize_state<T: ContractInterface>(parameters: i64, state: i64) -> i64 {
        if let Err(e) = set_logger().map_err(|e| e.into_raw()) {
            return e;
        }
        let parameters = match read_streaming_bytes(parameters) {
            Ok(bytes) => Parameters::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let state = match read_streaming_bytes(state) {
            Ok(bytes) => State::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let summary = <T as ContractInterface>::summarize_state(parameters, state);
        ContractInterfaceResult::from(summary).into_raw()
    }

    pub fn inner_get_state_delta<T: ContractInterface>(
        parameters: i64,
        state: i64,
        summary: i64,
    ) -> i64 {
        if let Err(e) = set_logger().map_err(|e| e.into_raw()) {
            return e;
        }
        let parameters = match read_streaming_bytes(parameters) {
            Ok(bytes) => Parameters::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let state = match read_streaming_bytes(state) {
            Ok(bytes) => State::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let summary = match read_streaming_bytes(summary) {
            Ok(bytes) => StateSummary::from(bytes),
            Err(e) => return e.into_raw(),
        };
        let new_delta = <T as ContractInterface>::get_state_delta(parameters, state, summary);
        ContractInterfaceResult::from(new_delta).into_raw()
    }
}
