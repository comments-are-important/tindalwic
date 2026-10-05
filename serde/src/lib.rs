//! implementations of the serde features

use serde::Deserialize;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::ser::Serialize;
use std::fmt::{self, Display};
use std::marker::PhantomData;
use std::result::Result as StdResult;
use strum::{EnumCount, IntoStaticStr, VariantArray, VariantNames};
use tindalwic::parse::Build;
use tindalwic::*;

/// [conventional](https://serde.rs/conventions.html) Deserializer API module
pub mod de;
/// [conventional](https://serde.rs/conventions.html) Serializer API module
pub mod ser;

/// specialized to Err([Error])
pub type Result<T> = StdResult<T, Error>;
pub use de::ItemDe as Deserializer;
pub use ser::ItemSer as Serializer;

/// payload is just an English message
#[derive(Debug)]
pub struct Error(String);
impl Error {
    /// construct from slice
    pub fn new(message: &str) -> Self {
        Error(String::from(message))
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl serde::ser::Error for Error {
    fn custom<T: Display>(m: T) -> Self {
        Error(m.to_string())
    }
}
impl serde::de::Error for Error {
    fn custom<T: Display>(m: T) -> Self {
        Error(m.to_string())
    }
}

// ==================================================================================

// the serde derive macros can't predict what seed might be used,
// so even if the core crate had serde and could use the macros,
// the de impl would still need to be written out.

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

struct ValueSer<'a>(Value<'a>);
impl<'a> Serialize for ValueSer<'a> {
    fn serialize<S>(&self, s: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let ValueSer(this) = self;
        if let Some(slice) = this.verbatim(0) {
            s.serialize_str(slice)
        } else {
            s.serialize_str(&this.joined())
        }
    }
}
struct ValueDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ValueDe<'a, 'b> {
    type Value = Value<'a>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_str(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ValueDe<'a, 'b> {
    type Value = Value<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a string value")
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> StdResult<Self::Value, E> {
        let ValueDe(build) = self;
        Ok(build.intern(v).map_err(E::custom)?.into())
    }
}

struct MaybeSer<'a>(Option<Value<'a>>);
impl<'a> Serialize for MaybeSer<'a> {
    fn serialize<S>(&self, s: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let MaybeSer(this) = self;
        match this {
            None => s.serialize_none(),
            Some(value) => s.serialize_some(&ValueSer(*value)),
        }
    }
}
struct MaybeDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for MaybeDe<'a, 'b> {
    type Value = Option<Value<'a>>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_option(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for MaybeDe<'a, 'b> {
    type Value = Option<Value<'a>>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("an optional value (comments)")
    }
    fn visit_none<E: serde::de::Error>(self) -> StdResult<Self::Value, E> {
        Ok(None)
    }
    fn visit_some<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let MaybeDe(build) = self;
        ValueDe(build).deserialize(d).map(|value| Some(value))
    }
}

// ==================================================================================

trait VariantHelp: VariantNames + Into<&'static str> + Copy {
    const KIND: &'static str;
    fn num(self) -> usize;

    const NAMES: &'static [&'static str] = <Self as VariantNames>::VARIANTS;
    fn serialize<S, W>(&self, s: S, value: &W) -> StdResult<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
        W: ?Sized + Serialize,
    {
        s.serialize_newtype_variant(Self::KIND, self.num() as u32, (*self).into(), value)
    }
}
macro_rules! variantHelp {
    ($helper:ident<$type:ident>) => {
        impl VariantHelp for $helper {
            const KIND: &'static str = stringify!($type);
            fn num(self) -> usize {
                self as usize
            }
        }
    };
}

#[derive(Clone, Copy, Deserialize, IntoStaticStr, VariantNames)]
#[serde(variant_identifier)]
enum ItemVariants {
    Text,
    List,
    Dict,
}
variantHelp! { ItemVariants<Item> }

struct Flags<H: FieldHelp> {
    flags: Box<[bool]>,
    phantom: PhantomData<H>,
}
impl<H: FieldHelp> Flags<H> {
    fn count(&self) -> usize {
        self.flags.iter().map(|flag| (*flag) as usize).sum()
    }
    fn selected(&self) -> impl std::iter::Iterator<Item = &H> {
        H::ARRAY.iter().filter(|variant| self.flags[variant.num()])
    }
    fn once<'de, A, F>(&mut self, variant: H, f: F) -> StdResult<H, A::Error>
    where
        A: MapAccess<'de>,
        F: Fn(&'static str) -> A::Error,
    {
        if self.flags[variant.num()] {
            return Err(f(variant.into()));
        }
        self.flags[variant.num()] = true;
        Ok(variant)
    }
}
trait FieldHelp: EnumCount + VariantArray + VariantNames + Into<&'static str> + Copy {
    const KIND: &'static str;
    fn num(self) -> usize;

    const NAMES: &'static [&'static str] = <Self as VariantNames>::VARIANTS;
    const ARRAY: &'static [Self] = <Self as VariantArray>::VARIANTS;
    fn each<'a>(value: bool) -> Flags<Self> {
        Flags {
            flags: vec![value; Self::COUNT].into_boxed_slice(),
            phantom: PhantomData,
        }
    }
    fn assign<F: Fn(Self) -> bool>(f: F) -> Flags<Self> {
        let mut flags = Self::each(false);
        for variant in Self::ARRAY {
            flags.flags[variant.num()] = (f)(*variant)
        }
        flags
    }
}
macro_rules! fieldHelp {
    ($helper:ident<$type:ident>) => {
        impl FieldHelp for $helper {
            const KIND: &'static str = stringify!($type);
            fn num(self) -> usize {
                self as usize
            }
        }
    };
}

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum CommentFields {
    Gap,
    Value,
}
fieldHelp!(CommentFields<Comment>);

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum TextFields {
    Value,
    Epilog,
}
fieldHelp!(TextFields<Text>);

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum ListFields {
    Prolog,
    Items,
    Epilog,
}
fieldHelp!(ListFields<List>);

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum NameFields {
    Comment,
    Key,
}
fieldHelp!(NameFields<Name>);

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum EntryFields {
    Name,
    Item,
}
fieldHelp!(EntryFields<Entry>);

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum DictFields {
    Prolog,
    Entries,
    Epilog,
}
fieldHelp!(DictFields<Dict>);

#[derive(Clone, Copy, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
#[serde(field_identifier, rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum FileFields {
    Hashbang,
    Prolog,
    Entries,
}
fieldHelp!(FileFields<File>);
