#![feature(prelude_import)]
#![allow(dead_code, unused)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::error::Error;
mod parent {
    use std::error::Error;
    use super::children::{self, *};
    use freenet_stdlib::{composers::*, prelude::*};
    use serde::{Deserialize, Serialize};
    use freenet_stdlib::memory::wasm_interface::inner_validate_state;
    mod low_level_ffi_impl {
        use super::*;
        use freenet_stdlib::prelude::ContractInterface;
        impl SerializationAdapter for ParentContract {
            type Parameters = ParentContractParams;
            type Delta = ParentContractDelta;
            type Summary = ParentContractSummary;
        }
        impl BincodeEncoder for ParentContract {}
        impl BincodeEncoder for ParentContractParams {}
        impl BincodeEncoder for ParentContractDelta {}
        impl BincodeEncoder for ParentContractSummary {}
    }
    pub struct ParentContract {
        contract_b_0: ChildContract,
        contract_b_1: ChildContract,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ParentContract {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "ParentContract",
                    false as usize + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "contract_b_0",
                    &self.contract_b_0,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "contract_b_1",
                    &self.contract_b_1,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ParentContract {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "contract_b_0" => _serde::__private::Ok(__Field::__field0),
                            "contract_b_1" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"contract_b_0" => _serde::__private::Ok(__Field::__field0),
                            b"contract_b_1" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ParentContract>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ParentContract;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct ParentContract",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            ChildContract,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct ParentContract with 2 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            ChildContract,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct ParentContract with 2 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(ParentContract {
                            contract_b_0: __field0,
                            contract_b_1: __field1,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<ChildContract> = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<ChildContract> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "contract_b_0",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            ChildContract,
                                        >(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "contract_b_1",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            ChildContract,
                                        >(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("contract_b_0")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("contract_b_1")?
                            }
                        };
                        _serde::__private::Ok(ParentContract {
                            contract_b_0: __field0,
                            contract_b_1: __field1,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &[
                    "contract_b_0",
                    "contract_b_1",
                ];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "ParentContract",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ParentContract>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    impl<'a> From<&'a Parameters<'static>> for ParentContractParams {
        fn from(value: &'a Parameters<'static>) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    pub struct ParentContractParams {
        contract_b_0_params: ChildContractParams,
        contract_b_1_params: ChildContractParams,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ParentContractParams {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "ParentContractParams",
                    false as usize + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "contract_b_0_params",
                    &self.contract_b_0_params,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "contract_b_1_params",
                    &self.contract_b_1_params,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ParentContractParams {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "contract_b_0_params" => {
                                _serde::__private::Ok(__Field::__field0)
                            }
                            "contract_b_1_params" => {
                                _serde::__private::Ok(__Field::__field1)
                            }
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"contract_b_0_params" => {
                                _serde::__private::Ok(__Field::__field0)
                            }
                            b"contract_b_1_params" => {
                                _serde::__private::Ok(__Field::__field1)
                            }
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ParentContractParams>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ParentContractParams;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct ParentContractParams",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            ChildContractParams,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct ParentContractParams with 2 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            ChildContractParams,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct ParentContractParams with 2 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(ParentContractParams {
                            contract_b_0_params: __field0,
                            contract_b_1_params: __field1,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<
                            ChildContractParams,
                        > = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<
                            ChildContractParams,
                        > = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "contract_b_0_params",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            ChildContractParams,
                                        >(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "contract_b_1_params",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            ChildContractParams,
                                        >(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("contract_b_0_params")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("contract_b_1_params")?
                            }
                        };
                        _serde::__private::Ok(ParentContractParams {
                            contract_b_0_params: __field0,
                            contract_b_1_params: __field1,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &[
                    "contract_b_0_params",
                    "contract_b_1_params",
                ];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "ParentContractParams",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ParentContractParams>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    impl ComposableParameters for ParentContractParams {
        fn contract_id(&self) -> Option<ContractInstanceId> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct ParentContractSummary;
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ParentContractSummary {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serializer::serialize_unit_struct(
                    __serializer,
                    "ParentContractSummary",
                )
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ParentContractSummary {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ParentContractSummary>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ParentContractSummary;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "unit struct ParentContractSummary",
                        )
                    }
                    #[inline]
                    fn visit_unit<__E>(
                        self,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        _serde::__private::Ok(ParentContractSummary)
                    }
                }
                _serde::Deserializer::deserialize_unit_struct(
                    __deserializer,
                    "ParentContractSummary",
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ParentContractSummary>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    impl<'a> From<&'a ParentContract> for ParentContractSummary {
        fn from(value: &'a ParentContract) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl<'a> From<&'a ParentContractSummary> for ChildContractSummary {
        fn from(value: &'a ParentContractSummary) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl ComposableSummary<ChildContractSummary> for ParentContractSummary {
        fn merge(&mut self, _value: ChildContractSummary) {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl ComposableSummary<ParentContractSummary> for ParentContractSummary {
        fn merge(&mut self, _value: ParentContractSummary) {
            ::core::panicking::panic("not yet implemented")
        }
    }
    pub struct ParentContractDelta {
        contract_b_0: ChildContractDelta,
        contract_b_1: ChildContractDelta,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ParentContractDelta {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "ParentContractDelta",
                    false as usize + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "contract_b_0",
                    &self.contract_b_0,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "contract_b_1",
                    &self.contract_b_1,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ParentContractDelta {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "contract_b_0" => _serde::__private::Ok(__Field::__field0),
                            "contract_b_1" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"contract_b_0" => _serde::__private::Ok(__Field::__field0),
                            b"contract_b_1" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ParentContractDelta>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ParentContractDelta;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct ParentContractDelta",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            ChildContractDelta,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct ParentContractDelta with 2 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            ChildContractDelta,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct ParentContractDelta with 2 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(ParentContractDelta {
                            contract_b_0: __field0,
                            contract_b_1: __field1,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<
                            ChildContractDelta,
                        > = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<
                            ChildContractDelta,
                        > = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "contract_b_0",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            ChildContractDelta,
                                        >(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "contract_b_1",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            ChildContractDelta,
                                        >(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("contract_b_0")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("contract_b_1")?
                            }
                        };
                        _serde::__private::Ok(ParentContractDelta {
                            contract_b_0: __field0,
                            contract_b_1: __field1,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &[
                    "contract_b_0",
                    "contract_b_1",
                ];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "ParentContractDelta",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ParentContractDelta>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    impl<'a> From<&'a State<'static>> for ParentContract {
        fn from(value: &'a State<'static>) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl<'a> From<&'a ParentContract> for ChildContract {
        fn from(value: &'a ParentContract) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl<'a> From<&'a ParentContractDelta> for ChildContractDelta {
        fn from(value: &'a ParentContractDelta) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl<'a> From<&'a ParentContractParams> for ChildContractParams {
        fn from(value: &'a ParentContractParams) -> Self {
            ::core::panicking::panic("not yet implemented")
        }
    }
    impl ComposableContract for ParentContract {
        type Context = NoContext;
        type Parameters = ParentContractParams;
        type Delta = ParentContractDelta;
        type Summary = ParentContractSummary;
        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<ValidateResult, ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            <ChildContract as ComposableContract>::verify::<
                ChildContract,
                Self,
            >(
                &self.contract_b_0,
                &<ChildContract as ComposableContract>::Parameters::from(parameters),
                self,
                related,
            )?;
            Ok(ValidateResult::Valid)
        }
        fn verify_delta<Child>(
            parameters: &Self::Parameters,
            delta: &Self::Delta,
        ) -> Result<bool, ContractError>
        where
            Child: ComposableContract,
        {
            <ChildContract as ComposableContract>::verify_delta::<
                ChildContract,
            >(&parameters.into(), &delta.into())?;
            Ok(true)
        }
        fn merge(
            &mut self,
            parameters: &Self::Parameters,
            update_data: &TypedUpdateData<Self>,
            related: &RelatedContractsContainer,
        ) -> MergeResult {
            {
                let sub_update: TypedUpdateData<ChildContract> = TypedUpdateData::from_other(
                    update_data,
                );
                match freenet_stdlib::composers::ComposableContract::merge(
                    &mut self.contract_b_0,
                    &parameters.into(),
                    &sub_update,
                    related,
                ) {
                    MergeResult::Success => {}
                    MergeResult::RequestRelated(req) => {
                        return MergeResult::RequestRelated(req);
                    }
                    MergeResult::Error(e) => return MergeResult::Error(e),
                }
            }
            {
                let sub_update: TypedUpdateData<ChildContract> = TypedUpdateData::from_other(
                    update_data,
                );
                match freenet_stdlib::composers::ComposableContract::merge(
                    &mut self.contract_b_1,
                    &parameters.into(),
                    &sub_update,
                    related,
                ) {
                    MergeResult::Success => {}
                    MergeResult::RequestRelated(req) => {
                        return MergeResult::RequestRelated(req);
                    }
                    MergeResult::Error(e) => return MergeResult::Error(e),
                }
            }
            MergeResult::Success
        }
        fn summarize<ParentSummary>(
            &self,
            parameters: &Self::Parameters,
            summary: &mut ParentSummary,
        ) -> Result<(), ContractError>
        where
            ParentSummary: ComposableSummary<<Self as ComposableContract>::Summary>,
        {
            self.contract_b_0.summarize(&parameters.into(), &mut ParentContractSummary)?;
            self.contract_b_1.summarize(&parameters.into(), &mut ParentContractSummary)?;
            Ok(())
        }
        fn delta(
            &self,
            parameters: &Self::Parameters,
            summary: &Self::Summary,
        ) -> Result<Self::Delta, ContractError> {
            let contract_b_0 = self
                .contract_b_0
                .delta(&parameters.into(), &summary.into())?;
            let contract_b_1 = self
                .contract_b_0
                .delta(&parameters.into(), &summary.into())?;
            Ok(ParentContractDelta {
                contract_b_0,
                contract_b_1,
            })
        }
    }
    impl ::freenet_stdlib::prelude::ContractInterface for ParentContract {
        fn validate_state(
            parameters: ::freenet_stdlib::prelude::Parameters<'static>,
            state: ::freenet_stdlib::prelude::State<'static>,
            related: ::freenet_stdlib::prelude::RelatedContracts<'static>,
        ) -> ::core::result::Result<
            ::freenet_stdlib::prelude::ValidateResult,
            ::freenet_stdlib::prelude::ContractError,
        > {
            match ::freenet_stdlib::composers::from_bytes::inner_validate_state::<
                ParentContract,
                ChildContract,
                <ParentContract as ::freenet_stdlib::composers::ComposableContract>::Context,
            >(parameters.clone(), state.clone(), related.clone())? {
                ::freenet_stdlib::prelude::ValidateResult::Valid => {}
                ::freenet_stdlib::prelude::ValidateResult::Invalid => {
                    return ::core::result::Result::Ok(
                        ::freenet_stdlib::prelude::ValidateResult::Invalid,
                    );
                }
                ::freenet_stdlib::prelude::ValidateResult::RequestRelated(req) => {
                    return ::core::result::Result::Ok(
                        ::freenet_stdlib::prelude::ValidateResult::RequestRelated(req),
                    );
                }
            }
            match ::freenet_stdlib::composers::from_bytes::inner_validate_state::<
                ParentContract,
                ChildContract,
                <ParentContract as ::freenet_stdlib::composers::ComposableContract>::Context,
            >(parameters.clone(), state.clone(), related.clone())? {
                ::freenet_stdlib::prelude::ValidateResult::Valid => {}
                ::freenet_stdlib::prelude::ValidateResult::Invalid => {
                    return ::core::result::Result::Ok(
                        ::freenet_stdlib::prelude::ValidateResult::Invalid,
                    );
                }
                ::freenet_stdlib::prelude::ValidateResult::RequestRelated(req) => {
                    return ::core::result::Result::Ok(
                        ::freenet_stdlib::prelude::ValidateResult::RequestRelated(req),
                    );
                }
            }
            ::core::result::Result::Ok(::freenet_stdlib::prelude::ValidateResult::Valid)
        }
        fn validate_delta(
            parameters: ::freenet_stdlib::prelude::Parameters<'static>,
            delta: ::freenet_stdlib::prelude::StateDelta<'static>,
        ) -> ::core::result::Result<bool, ::freenet_stdlib::prelude::ContractError> {
            if !::freenet_stdlib::composers::from_bytes::inner_validate_delta::<
                ParentContract,
                ChildContract,
            >(parameters.clone(), delta.clone())? {
                return ::core::result::Result::Ok(false);
            }
            if !::freenet_stdlib::composers::from_bytes::inner_validate_delta::<
                ParentContract,
                ChildContract,
            >(parameters.clone(), delta.clone())? {
                return ::core::result::Result::Ok(false);
            }
            ::core::result::Result::Ok(true)
        }
        fn update_state(
            parameters: ::freenet_stdlib::prelude::Parameters<'static>,
            state: ::freenet_stdlib::prelude::State<'static>,
            data: Vec<freenet_stdlib::prelude::UpdateData<'static>>,
        ) -> ::core::result::Result<
            ::freenet_stdlib::prelude::UpdateModification<'static>,
            ::freenet_stdlib::prelude::ContractError,
        > {
            let mut final_update = state;
            {
                let modification = ::freenet_stdlib::composers::from_bytes::inner_update_state::<
                    ParentContract,
                    ChildContract,
                >(parameters.clone(), final_update.clone(), data.clone())?;
                if modification.requires_dependencies() {
                    return ::core::result::Result::Ok(modification);
                } else {
                    final_update = modification.unwrap_valid();
                }
            }
            {
                let modification = ::freenet_stdlib::composers::from_bytes::inner_update_state::<
                    ParentContract,
                    ChildContract,
                >(parameters.clone(), final_update.clone(), data.clone())?;
                if modification.requires_dependencies() {
                    return ::core::result::Result::Ok(modification);
                } else {
                    final_update = modification.unwrap_valid();
                }
            }
            Ok(::freenet_stdlib::prelude::UpdateModification::valid(final_update))
        }
        fn summarize_state(
            parameters: ::freenet_stdlib::prelude::Parameters<'static>,
            state: ::freenet_stdlib::prelude::State<'static>,
        ) -> ::core::result::Result<
            ::freenet_stdlib::prelude::StateSummary<'static>,
            ::freenet_stdlib::prelude::ContractError,
        > {
            let mut summary: ::core::option::Option<
                <ParentContract as ::freenet_stdlib::composers::ComposableContract>::Summary,
            > = ::core::option::Option::None;
            let summary = ::freenet_stdlib::composers::from_bytes::inner_summarize_state::<
                ParentContract,
            >(parameters.clone(), state.clone())?;
            let serializable_summary = <ParentContract as ::freenet_stdlib::prelude::SerializationAdapter>::Summary::from(
                summary,
            );
            let encoded_summary = ::freenet_stdlib::prelude::Encoder::serialize(
                &serializable_summary,
            )?;
            Ok(encoded_summary.into())
        }
        fn get_state_delta(
            parameters: ::freenet_stdlib::prelude::Parameters<'static>,
            state: ::freenet_stdlib::prelude::State<'static>,
            summary: ::freenet_stdlib::prelude::StateSummary<'static>,
        ) -> ::core::result::Result<
            ::freenet_stdlib::prelude::StateDelta<'static>,
            ::freenet_stdlib::prelude::ContractError,
        > {
            let delta = ::freenet_stdlib::composers::from_bytes::inner_state_delta::<
                ParentContract,
            >(parameters.clone(), state.clone(), summary.clone())?;
            let serializable_delta = <ParentContract as SerializationAdapter>::Delta::from(
                delta,
            );
            let encoded_delta = ::freenet_stdlib::prelude::Encoder::serialize(
                &serializable_delta,
            )?;
            Ok(encoded_delta.into())
        }
    }
    #[no_mangle]
    #[cfg(feature = "freenet-main-contract")]
    pub extern "C" fn validate_state(parameters: i64, state: i64, related: i64) -> i64 {
        ::freenet_stdlib::memory::wasm_interface::inner_validate_state::<
            ParentContract,
        >(parameters, state, related)
    }
    #[no_mangle]
    #[cfg(feature = "freenet-main-contract")]
    pub extern "C" fn validate_delta(parameters: i64, delta: i64) -> i64 {
        ::freenet_stdlib::memory::wasm_interface::inner_validate_delta::<
            ParentContract,
        >(parameters, delta)
    }
    #[no_mangle]
    #[cfg(feature = "freenet-main-contract")]
    pub extern "C" fn update_state(parameters: i64, state: i64, delta: i64) -> i64 {
        ::freenet_stdlib::memory::wasm_interface::inner_update_state::<
            ParentContract,
        >(parameters, state, delta)
    }
    #[no_mangle]
    #[cfg(feature = "freenet-main-contract")]
    pub extern "C" fn summarize_state(parameters: i64, state: i64) -> i64 {
        ::freenet_stdlib::memory::wasm_interface::inner_summarize_state::<
            ParentContract,
        >(parameters, state)
    }
    #[no_mangle]
    #[cfg(feature = "freenet-main-contract")]
    pub extern "C" fn get_state_delta(parameters: i64, state: i64, summary: i64) -> i64 {
        ::freenet_stdlib::memory::wasm_interface::inner_get_state_delta::<
            ParentContract,
        >(parameters, state, summary)
    }
    impl<'x> From<&'x ParentContract> for children::PubKey {
        fn from(value: &'x ParentContract) -> Self {
            children::PubKey
        }
    }
}
mod children {
    use std::error::Error;
    use freenet_stdlib::{composers::*, prelude::*};
    use serde::{Deserialize, Serialize};
    pub struct ChildContract {}
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ChildContract {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "ChildContract",
                    false as usize,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ChildContract {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ChildContract>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ChildContract;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct ChildContract",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        _: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        _serde::__private::Ok(ChildContract {})
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        _serde::__private::Ok(ChildContract {})
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &[];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "ChildContract",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ChildContract>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    pub struct ChildContractParams;
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ChildContractParams {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serializer::serialize_unit_struct(
                    __serializer,
                    "ChildContractParams",
                )
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ChildContractParams {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ChildContractParams>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ChildContractParams;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "unit struct ChildContractParams",
                        )
                    }
                    #[inline]
                    fn visit_unit<__E>(
                        self,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        _serde::__private::Ok(ChildContractParams)
                    }
                }
                _serde::Deserializer::deserialize_unit_struct(
                    __deserializer,
                    "ChildContractParams",
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ChildContractParams>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::clone::Clone for ChildContractParams {
        #[inline]
        fn clone(&self) -> ChildContractParams {
            ChildContractParams
        }
    }
    impl ComposableParameters for ChildContractParams {
        fn contract_id(&self) -> Option<ContractInstanceId> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct ChildContractSummary;
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ChildContractSummary {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serializer::serialize_unit_struct(
                    __serializer,
                    "ChildContractSummary",
                )
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ChildContractSummary {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ChildContractSummary>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ChildContractSummary;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "unit struct ChildContractSummary",
                        )
                    }
                    #[inline]
                    fn visit_unit<__E>(
                        self,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        _serde::__private::Ok(ChildContractSummary)
                    }
                }
                _serde::Deserializer::deserialize_unit_struct(
                    __deserializer,
                    "ChildContractSummary",
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ChildContractSummary>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    pub struct ChildContractDelta;
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ChildContractDelta {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serializer::serialize_unit_struct(
                    __serializer,
                    "ChildContractDelta",
                )
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ChildContractDelta {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ChildContractDelta>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ChildContractDelta;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "unit struct ChildContractDelta",
                        )
                    }
                    #[inline]
                    fn visit_unit<__E>(
                        self,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        _serde::__private::Ok(ChildContractDelta)
                    }
                }
                _serde::Deserializer::deserialize_unit_struct(
                    __deserializer,
                    "ChildContractDelta",
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ChildContractDelta>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    pub struct PubKey;
    impl From<ChildContractParams> for PubKey {
        fn from(value: ChildContractParams) -> Self {
            PubKey
        }
    }
    impl ComposableContract for ChildContract {
        type Context = PubKey;
        type Summary = ChildContractSummary;
        type Parameters = ChildContractParams;
        type Delta = ChildContractDelta;
        fn verify<Child, Ctx>(
            &self,
            parameters: &Self::Parameters,
            context: &Ctx,
            related: &RelatedContractsContainer,
        ) -> Result<ValidateResult, ContractError>
        where
            Child: ComposableContract,
            Self::Context: for<'x> From<&'x Ctx>,
        {
            let pub_key = PubKey::from(context);
            Ok(ValidateResult::Valid)
        }
        fn verify_delta<Child>(
            parameters: &Self::Parameters,
            delta: &Self::Delta,
        ) -> Result<bool, ContractError>
        where
            Child: ComposableContract,
        {
            let pub_key = PubKey::from(parameters.clone());
            Ok(true)
        }
        fn merge(
            &mut self,
            parameters: &Self::Parameters,
            _delta: &TypedUpdateData<Self>,
            related: &RelatedContractsContainer,
        ) -> MergeResult {
            let contract_id = parameters.contract_id().unwrap();
            let Related::Found { state: mut contract_b, .. } = related
                .get::<ChildContract>(&contract_id) else {
                let mut req = RelatedContractsContainer::default();
                req.request::<ChildContract>(contract_id);
                return MergeResult::RequestRelated(req);
            };
            MergeResult::Success
        }
        fn delta(
            &self,
            _parameters: &Self::Parameters,
            _summary: &Self::Summary,
        ) -> Result<Self::Delta, ContractError> {
            ::core::panicking::panic("not yet implemented")
        }
        fn summarize<ParentSummary>(
            &self,
            _parameters: &Self::Parameters,
            summary: &mut ParentSummary,
        ) -> Result<(), ContractError>
        where
            ParentSummary: ComposableSummary<<Self as ComposableContract>::Summary>,
        {
            summary.merge(ChildContractSummary);
            ::core::panicking::panic("not yet implemented")
        }
    }
}
#[rustc_main]
#[no_coverage]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
