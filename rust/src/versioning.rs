use std::fmt;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Arc;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    client_api::{TryFromFbs, WsApiError},
    common_generated::common::{ContractContainer as FbsContractContainer, ContractType},
    contract_interface::{ContractInstanceId, ContractKey},
    generated::client_request::{DelegateContainer as FbsDelegateContainer, DelegateType},
    parameters::Parameters,
    prelude::{
        CodeHash, ContractCode, ContractWasmAPIVersion::V1, Delegate, DelegateCode, DelegateKey,
        WrappedContract,
    },
};

/// Contains the different versions available for WASM delegates.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DelegateWasmAPIVersion {
    V1(#[serde(deserialize_with = "Delegate::deserialize_delegate")] Delegate<'static>),
}

impl DelegateWasmAPIVersion {}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DelegateContainer {
    Wasm(DelegateWasmAPIVersion),
}

impl From<DelegateContainer> for APIVersion {
    fn from(delegate: DelegateContainer) -> APIVersion {
        match delegate {
            DelegateContainer::Wasm(DelegateWasmAPIVersion::V1(_)) => APIVersion::Version0_0_1,
        }
    }
}

impl DelegateContainer {
    pub fn key(&self) -> &DelegateKey {
        match self {
            Self::Wasm(DelegateWasmAPIVersion::V1(delegate_v1)) => delegate_v1.key(),
        }
    }

    pub fn code(&self) -> &DelegateCode {
        match self {
            Self::Wasm(DelegateWasmAPIVersion::V1(delegate_v1)) => delegate_v1.code(),
        }
    }

    pub fn code_hash(&self) -> &CodeHash {
        match self {
            Self::Wasm(DelegateWasmAPIVersion::V1(delegate_v1)) => delegate_v1.code_hash(),
        }
    }
}

impl<'a> TryFrom<(&'a Path, Parameters<'static>)> for DelegateContainer {
    type Error = std::io::Error;

    fn try_from((path, params): (&'a Path, Parameters<'static>)) -> Result<Self, Self::Error> {
        let (contract_code, version) = DelegateCode::load_versioned_from_path(path)?;

        match version {
            APIVersion::Version0_0_1 => {
                let delegate = Delegate::from((&contract_code, &params));
                Ok(DelegateContainer::Wasm(DelegateWasmAPIVersion::V1(
                    delegate,
                )))
            }
        }
    }
}

impl<'a, P> TryFrom<(Vec<u8>, P)> for DelegateContainer
where
    P: std::ops::Deref<Target = Parameters<'a>>,
{
    type Error = std::io::Error;

    fn try_from((versioned_contract_bytes, params): (Vec<u8>, P)) -> Result<Self, Self::Error> {
        let params = params.deref().clone().into_owned();

        let (contract_code, version) =
            DelegateCode::load_versioned_from_bytes(versioned_contract_bytes)?;

        match version {
            APIVersion::Version0_0_1 => {
                let delegate = Delegate::from((&contract_code, &params));
                Ok(DelegateContainer::Wasm(DelegateWasmAPIVersion::V1(
                    delegate,
                )))
            }
        }
    }
}

impl<'a> TryFromFbs<&FbsDelegateContainer<'a>> for DelegateContainer {
    fn try_decode_fbs(container: &FbsDelegateContainer<'a>) -> Result<Self, WsApiError> {
        match container.delegate_type() {
            DelegateType::WasmDelegateV1 => {
                let delegate = container.delegate_as_wasm_delegate_v1().unwrap();
                let data = DelegateCode::from(delegate.data().data().bytes().to_vec());
                let params = Parameters::from(delegate.parameters().bytes().to_vec());
                Ok(DelegateContainer::Wasm(DelegateWasmAPIVersion::V1(
                    Delegate::from((&data, &params)),
                )))
            }
            _ => unreachable!(),
        }
    }
}

impl DelegateCode<'static> {
    fn load_versioned(
        mut contract_data: Cursor<Vec<u8>>,
    ) -> Result<(Self, APIVersion), std::io::Error> {
        // Get contract version
        let version = contract_data.read_u64::<BigEndian>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to read version")
        })?;
        let version = APIVersion::from_u64(version).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Version error: {}", e),
            )
        })?;

        if version == APIVersion::Version0_0_1 {
            let mut code_hash = [0u8; 32];
            contract_data.read_exact(&mut code_hash)?;
        }

        // Get contract code
        let mut code_data: Vec<u8> = vec![];
        contract_data
            .read_to_end(&mut code_data)
            .map_err(|_| std::io::ErrorKind::InvalidData)?;
        Ok((DelegateCode::from(code_data), version))
    }

    /// Loads contract code which has been versioned from the fs.
    pub fn load_versioned_from_path(path: &Path) -> Result<(Self, APIVersion), std::io::Error> {
        let contract_data = Cursor::new(Self::load_bytes(path)?);
        Self::load_versioned(contract_data)
    }

    /// Loads contract code which has been versioned from the fs.
    pub fn load_versioned_from_bytes(
        versioned_code: Vec<u8>,
    ) -> Result<(Self, APIVersion), std::io::Error> {
        let contract_data = Cursor::new(versioned_code);
        Self::load_versioned(contract_data)
    }
}

impl DelegateCode<'_> {
    pub fn to_bytes_versioned(
        &self,
        version: APIVersion,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match version {
            APIVersion::Version0_0_1 => {
                let output_size =
                    std::mem::size_of::<u64>() + self.data().len() + self.hash().0.len();
                let mut output: Vec<u8> = Vec::with_capacity(output_size);
                output.write_u64::<BigEndian>(APIVersion::Version0_0_1.into_u64())?;
                output.extend(self.hash().0.iter());
                output.extend(self.data());
                Ok(output)
            }
        }
    }
}

/// Contains the different versions available for WASM contracts.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]

pub enum ContractWasmAPIVersion {
    V1(WrappedContract),
}

impl From<ContractWasmAPIVersion> for ContractContainer {
    fn from(value: ContractWasmAPIVersion) -> Self {
        ContractContainer::Wasm(value)
    }
}

impl Display for ContractWasmAPIVersion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ContractWasmAPIVersion::V1(contract_v1) => {
                write!(f, "[api=0.0.1]({contract_v1})")
            }
        }
    }
}

/// Wrapper that allows contract versioning. This enum maintains the types of contracts that are
/// allowed and their corresponding version.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContractContainer {
    Wasm(ContractWasmAPIVersion),
}

impl From<ContractContainer> for APIVersion {
    fn from(contract: ContractContainer) -> APIVersion {
        match contract {
            ContractContainer::Wasm(ContractWasmAPIVersion::V1(_)) => APIVersion::Version0_0_1,
        }
    }
}

impl ContractContainer {
    /// Return the `ContractKey` from the specific contract version.
    pub fn key(&self) -> ContractKey {
        match self {
            Self::Wasm(ContractWasmAPIVersion::V1(contract_v1)) => *contract_v1.key(),
        }
    }

    /// Return the `ContractInstanceId` from the specific contract version.
    pub fn id(&self) -> &ContractInstanceId {
        match self {
            Self::Wasm(ContractWasmAPIVersion::V1(contract_v1)) => contract_v1.key().id(),
        }
    }

    /// Return the `Parameters` from the specific contract version.
    pub fn params(&self) -> Parameters<'static> {
        match self {
            Self::Wasm(ContractWasmAPIVersion::V1(contract_v1)) => contract_v1.params().clone(),
        }
    }

    /// Return the contract code from the specific contract version as `Vec<u8>`.
    pub fn data(&self) -> &[u8] {
        match self {
            Self::Wasm(ContractWasmAPIVersion::V1(contract_v1)) => contract_v1.data.data(),
        }
    }

    pub fn unwrap_v1(self) -> WrappedContract {
        match self {
            Self::Wasm(ContractWasmAPIVersion::V1(contract_v1)) => contract_v1,
        }
    }
}

impl Display for ContractContainer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ContractContainer::Wasm(wasm_version) => {
                write!(f, "WasmContainer({wasm_version})")
            }
        }
    }
}

impl<'a> TryFrom<(&'a Path, Parameters<'static>)> for ContractContainer {
    type Error = std::io::Error;

    fn try_from((path, params): (&'a Path, Parameters<'static>)) -> Result<Self, Self::Error> {
        let (contract_code, version) = ContractCode::load_versioned_from_path(path)?;

        match version {
            APIVersion::Version0_0_1 => Ok(ContractContainer::Wasm(ContractWasmAPIVersion::V1(
                WrappedContract::new(Arc::new(contract_code), params),
            ))),
        }
    }
}

impl<'a, P> TryFrom<(Vec<u8>, P)> for ContractContainer
where
    P: std::ops::Deref<Target = Parameters<'a>>,
{
    type Error = std::io::Error;

    fn try_from((versioned_contract_bytes, params): (Vec<u8>, P)) -> Result<Self, Self::Error> {
        let params = params.deref().clone().into_owned();

        let (contract_code, version) =
            ContractCode::load_versioned_from_bytes(versioned_contract_bytes)?;

        match version {
            APIVersion::Version0_0_1 => Ok(ContractContainer::Wasm(ContractWasmAPIVersion::V1(
                WrappedContract::new(Arc::new(contract_code), params),
            ))),
        }
    }
}

impl ContractCode<'static> {
    fn load_versioned(
        mut contract_data: Cursor<Vec<u8>>,
    ) -> Result<(Self, APIVersion), std::io::Error> {
        // Get contract version
        let version = contract_data.read_u64::<BigEndian>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to read version")
        })?;
        let version = APIVersion::from_u64(version).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Version error: {}", e),
            )
        })?;

        if version == APIVersion::Version0_0_1 {
            let mut code_hash = [0u8; 32];
            contract_data.read_exact(&mut code_hash)?;
        }

        // Get contract code
        let mut code_data: Vec<u8> = vec![];
        contract_data
            .read_to_end(&mut code_data)
            .map_err(|_| std::io::ErrorKind::InvalidData)?;
        Ok((ContractCode::from(code_data), version))
    }

    /// Loads contract code which has been versioned from the fs.
    pub fn load_versioned_from_path(path: &Path) -> Result<(Self, APIVersion), std::io::Error> {
        let contract_data = Cursor::new(Self::load_bytes(path)?);
        Self::load_versioned(contract_data)
    }

    /// Loads contract code which has been versioned from the fs.
    pub fn load_versioned_from_bytes(
        versioned_code: Vec<u8>,
    ) -> Result<(Self, APIVersion), std::io::Error> {
        let contract_data = Cursor::new(versioned_code);
        Self::load_versioned(contract_data)
    }
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("unsupported incremental API version: {0}")]
    UnsupportedVersion(u64),
    #[error("failed to read version: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum APIVersion {
    Version0_0_1,
}

impl APIVersion {
    fn from_u64(version: u64) -> Result<Self, VersionError> {
        match version {
            0 => Ok(Self::Version0_0_1),
            v => Err(VersionError::UnsupportedVersion(v)),
        }
    }

    fn into_u64(self) -> u64 {
        match self {
            Self::Version0_0_1 => 0,
        }
    }
}

impl Display for APIVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            APIVersion::Version0_0_1 => write!(f, "0.0.1"),
        }
    }
}

impl<'a> TryFrom<&'a semver::Version> for APIVersion {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(value: &'a semver::Version) -> Result<Self, Self::Error> {
        match value {
            ver if ver == &semver::Version::new(0, 0, 1) => Ok(APIVersion::Version0_0_1),
            other => Err(format!("{other} version not supported").into()),
        }
    }
}

impl ContractCode<'_> {
    pub fn to_bytes_versioned(
        &self,
        version: APIVersion,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match version {
            APIVersion::Version0_0_1 => {
                let output_size =
                    std::mem::size_of::<u64>() + self.data().len() + self.hash().0.len();
                let mut output: Vec<u8> = Vec::with_capacity(output_size);
                output.write_u64::<BigEndian>(APIVersion::Version0_0_1.into_u64())?;
                output.extend(self.hash().0.iter());
                output.extend(self.data());
                Ok(output)
            }
        }
    }
}

impl<'a> TryFromFbs<&FbsContractContainer<'a>> for ContractContainer {
    fn try_decode_fbs(value: &FbsContractContainer<'a>) -> Result<Self, WsApiError> {
        match value.contract_type() {
            ContractType::WasmContractV1 => {
                let contract = value.contract_as_wasm_contract_v1().unwrap();
                let data = Arc::new(ContractCode::from(contract.data().data().bytes().to_vec()));
                let params = Parameters::from(contract.parameters().bytes().to_vec());
                let key = ContractKey::from_params_and_code(&params, &*data);
                Ok(ContractContainer::from(V1(WrappedContract {
                    data,
                    params,
                    key,
                })))
            }
            _ => unreachable!(),
        }
    }
}
