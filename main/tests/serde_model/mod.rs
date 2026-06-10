#![warn(unused)]

//! exercise each of the 29 types in the serde data model.
//!
//! these definitions aren't meant to be realistic. people will write their own data
//! models, then derive or manually impl the conversions to/from the serde data models.
//! the point here is to check that our serde format impl can handle all those possible
//! data models by verifying that all the defined visit fns are correctly called.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Owned {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Char(char),
    String(String),
    Bytes(Vec<u8>),
    Opt(Option<()>),
    Unit,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Borrowed<'a> {
    String(&'a str),
    Bytes(&'a [u8]),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Enum<T1, T2> {
    // Unit and newtype covered above
    Tuple(T1, T2),
    Struct { one: T1, two: T2 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unit;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Newtype<T>(T);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tuple<T1, T2>(T1, T2);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Struct<T1, T2> {
    one: T1,
    two: T2,
}

pub type Map<K, V> = ::std::collections::BTreeMap<K, V>;
/// there's std::hash_map, but it is experimental + unordered
#[macro_export]
macro_rules! map {
    () => {
        ::std::collections::BTreeMap::new()
    };
    ($($key:expr => $value:expr),+ $(,)?) => {
        {
            let mut map = ::std::collections::BTreeMap::new();
            $(
                map.insert($key, $value);
            )+
            map
        }
    };
}

/// vacuous, but provides reminder of what needs to be tested
#[macro_export]
macro_rules! seq {
    ($($item:tt),* $(,)?) => {
        vec!($($item),*)
    };
}

/// vacuous, but provides reminder of what needs to be tested
/// arrays are also "tuple"s per the model, covering them would be redundant
#[macro_export]
macro_rules! tuple {
    ($($item:tt),* $(,)?) => {
        ($($item),*)
    };
}
