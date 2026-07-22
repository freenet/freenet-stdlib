//! Contract key types and identifiers.
//!
//! This module provides the core types for identifying contracts:
//! - `ContractInstanceId`: The hash of contract code and parameters (use for routing/lookup)
//! - `ContractKey`: A complete key specification with code hash (use for storage/execution)

use std::borrow::Borrow;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

use blake3::{traits::digest::Digest, Hasher as Blake3};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::client_api::{TryFromFbs, WsApiError};
use crate::code_hash::CodeHash;
use crate::common_generated::common::ContractKey as FbsContractKey;
use crate::parameters::Parameters;

use super::code::ContractCode;
use super::CONTRACT_KEY_SIZE;

/// The key representing the hash of the contract executable code hash and a set of `parameters`.
#[serde_as]
#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
#[cfg_attr(
    any(feature = "testing", all(test, any(unix, windows))),
    derive(arbitrary::Arbitrary)
)]
#[repr(transparent)]
pub struct ContractInstanceId(#[serde_as(as = "[_; CONTRACT_KEY_SIZE]")] [u8; CONTRACT_KEY_SIZE]);

impl ContractInstanceId {
    pub fn from_params_and_code<'a>(
        params: impl Borrow<Parameters<'a>>,
        code: impl Borrow<ContractCode<'a>>,
    ) -> Self {
        generate_id(params.borrow(), code.borrow())
    }

    pub const fn new(key: [u8; CONTRACT_KEY_SIZE]) -> Self {
        Self(key)
    }

    /// `Base58` string representation of the `contract id`.
    pub fn encode(&self) -> String {
        bs58::encode(self.0)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Build `ContractId` from the binary representation.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, bs58::decode::Error> {
        let mut spec = [0; CONTRACT_KEY_SIZE];
        bs58::decode(bytes)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .onto(&mut spec)?;
        Ok(Self(spec))
    }
}

impl Deref for ContractInstanceId {
    type Target = [u8; CONTRACT_KEY_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ContractInstanceId {
    type Err = bs58::decode::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ContractInstanceId::from_bytes(s)
    }
}

impl TryFrom<String> for ContractInstanceId {
    type Error = bs58::decode::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        ContractInstanceId::from_bytes(s)
    }
}

impl Display for ContractInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl std::fmt::Debug for ContractInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ContractInstanceId")
            .field(&self.encode())
            .finish()
    }
}

/// A complete key specification, that represents a cryptographic hash that identifies the contract.
///
/// This type always contains both the instance ID and the code hash.
/// Use `ContractInstanceId` for operations that only need to identify the contract
/// (routing, client requests), and `ContractKey` for operations that need the full
/// specification (storage, execution).
#[serde_as]
#[derive(Debug, Eq, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(
    any(feature = "testing", all(test, any(unix, windows))),
    derive(arbitrary::Arbitrary)
)]
pub struct ContractKey {
    instance: ContractInstanceId,
    code: CodeHash,
}

impl ContractKey {
    pub fn from_params_and_code<'a>(
        params: impl Borrow<Parameters<'a>>,
        wasm_code: impl Borrow<ContractCode<'a>>,
    ) -> Self {
        let code = wasm_code.borrow();
        let id = generate_id(params.borrow(), code);
        let code_hash = *code.hash();
        Self {
            instance: id,
            code: code_hash,
        }
    }

    /// Gets the whole spec key hash.
    pub fn as_bytes(&self) -> &[u8] {
        self.instance.0.as_ref()
    }

    /// Returns the hash of the contract code.
    pub fn code_hash(&self) -> &CodeHash {
        &self.code
    }

    /// Returns the encoded hash of the contract code.
    pub fn encoded_code_hash(&self) -> String {
        bs58::encode(self.code.0)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string()
    }

    /// Returns the contract key from the encoded hash of the contract code and the given
    /// parameters.
    pub fn from_params(
        code_hash: impl Into<String>,
        parameters: Parameters,
    ) -> Result<Self, bs58::decode::Error> {
        let mut code_key = [0; CONTRACT_KEY_SIZE];
        bs58::decode(code_hash.into())
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .onto(&mut code_key)?;

        let mut hasher = Blake3::new();
        hasher.update(code_key.as_slice());
        hasher.update(parameters.as_ref());
        let full_key_arr = hasher.finalize();

        let mut spec = [0; CONTRACT_KEY_SIZE];
        spec.copy_from_slice(&full_key_arr);
        Ok(Self {
            instance: ContractInstanceId(spec),
            code: CodeHash(code_key),
        })
    }

    /// Returns the `Base58` encoded string of the [`ContractInstanceId`](ContractInstanceId).
    pub fn encoded_contract_id(&self) -> String {
        self.instance.encode()
    }

    pub fn id(&self) -> &ContractInstanceId {
        &self.instance
    }

    /// Constructs a ContractKey from a pre-computed instance ID and code hash.
    ///
    /// This is useful when the node needs to reconstruct a key from stored index data.
    /// Callers must ensure the instance_id was correctly derived from the code_hash
    /// and parameters, as this constructor does not verify consistency.
    pub fn from_id_and_code(instance_id: ContractInstanceId, code_hash: CodeHash) -> Self {
        Self {
            instance: instance_id,
            code: code_hash,
        }
    }
}

impl PartialEq for ContractKey {
    fn eq(&self, other: &Self) -> bool {
        self.instance == other.instance
    }
}

impl std::hash::Hash for ContractKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.instance.0.hash(state);
    }
}

impl From<ContractKey> for ContractInstanceId {
    fn from(key: ContractKey) -> Self {
        key.instance
    }
}

impl Deref for ContractKey {
    type Target = [u8; CONTRACT_KEY_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.instance.0
    }
}

impl std::fmt::Display for ContractKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.instance.fmt(f)
    }
}

impl<'a> TryFromFbs<&FbsContractKey<'a>> for ContractKey {
    fn try_decode_fbs(key: &FbsContractKey<'a>) -> Result<Self, WsApiError> {
        let key_bytes: [u8; CONTRACT_KEY_SIZE] = key.instance().data().bytes().try_into().unwrap();
        let instance = ContractInstanceId::new(key_bytes);
        // The `code` field carries the already-computed 32-byte code hash
        // (BLAKE3 of the wasm), so pass those bytes straight through. Calling
        // `CodeHash::from_code` here would hash the hash again —
        // BLAKE3(BLAKE3(wasm)) — yielding a key that never matches the store and
        // breaking every FlatBuffers UpdateRequest ("Contract not in store and
        // no code provided"). GET/SUBSCRIBE dodged this because they decode only
        // the instance id; UPDATE decodes the full key. The delegate decoder
        // already does the pass-through correctly (see
        // `DelegateKey::try_decode_fbs`). Regression test below.
        let code = key
            .code()
            .map(|code_hash| CodeHash::try_from(code_hash.bytes()))
            .transpose()
            .map_err(|e| WsApiError::deserialization(e.to_string()))?
            .ok_or_else(|| WsApiError::deserialization("ContractKey missing code hash".into()))?;
        Ok(ContractKey { instance, code })
    }
}

fn generate_id<'a>(
    parameters: &Parameters<'a>,
    code_data: &ContractCode<'a>,
) -> ContractInstanceId {
    let contract_hash = code_data.hash();

    let mut hasher = Blake3::new();
    hasher.update(contract_hash.0.as_slice());
    hasher.update(parameters.as_ref());
    let full_key_arr = hasher.finalize();

    debug_assert_eq!(full_key_arr[..].len(), CONTRACT_KEY_SIZE);
    let mut spec = [0; CONTRACT_KEY_SIZE];
    spec.copy_from_slice(&full_key_arr);
    ContractInstanceId(spec)
}

#[inline]
pub(super) fn internal_fmt_key(
    key: &[u8; CONTRACT_KEY_SIZE],
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let r = bs58::encode(key)
        .with_alphabet(bs58::Alphabet::BITCOIN)
        .into_string();
    write!(f, "{}", &r[..8])
}

#[cfg(test)]
mod fbs_tests {
    use super::*;
    use crate::common_generated::common::{
        ContractInstanceId as FbsContractInstanceId, ContractInstanceIdArgs, ContractKeyArgs,
    };

    /// The wire `code` field carries the raw 32-byte code hash, and the decoder
    /// must return those exact bytes. Regression for the double-hash bug where
    /// `try_decode_fbs` re-hashed the hash (BLAKE3(BLAKE3(wasm))), producing a
    /// key that never matched the store and failing every FlatBuffers
    /// UpdateRequest with "Contract not in store and no code provided".
    #[test]
    fn contract_key_code_hash_passes_through_fbs_decode() {
        let instance_bytes = [7u8; CONTRACT_KEY_SIZE];
        // A distinct, arbitrary code hash. The decoder must reproduce it
        // verbatim; if it re-hashes, the assertion below fails.
        let code_bytes = [42u8; CONTRACT_KEY_SIZE];

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let instance_data = builder.create_vector(&instance_bytes);
        let instance_offset = FbsContractInstanceId::create(
            &mut builder,
            &ContractInstanceIdArgs {
                data: Some(instance_data),
            },
        );
        let code = Some(builder.create_vector(&code_bytes));
        let key_offset = FbsContractKey::create(
            &mut builder,
            &ContractKeyArgs {
                instance: Some(instance_offset),
                code,
            },
        );
        builder.finish_minimal(key_offset);

        let fbs_key = flatbuffers::root::<FbsContractKey>(builder.finished_data())
            .expect("valid ContractKey flatbuffer");
        let decoded = ContractKey::try_decode_fbs(&fbs_key).expect("decode ContractKey");

        assert_eq!(
            decoded.code_hash().as_ref(),
            &code_bytes,
            "decoder must pass the code hash through unchanged, not re-hash it"
        );
        assert_eq!(decoded.id().as_bytes(), &instance_bytes);
    }
}
