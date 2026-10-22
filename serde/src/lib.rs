//! implementations of the serde features

use serde::Deserialize;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::ser::Serialize;
use std::fmt::{self, Display};
use std::marker::PhantomData;
use std::result::Result as StdResult;
use strum::{EnumCount, IntoStaticStr, VariantArray, VariantNames};
use tindalwic::parse::Build;
use tindalwic::text::UTF8;

/// [conventional](https://serde.rs/conventions.html) Deserializer API module
pub mod de;
/// [conventional](https://serde.rs/conventions.html) Serializer API module
pub mod ser;

/// specialized to Err([Error])
pub type Result<T> = StdResult<T, Error>;
pub use de::DataDe as Deserializer;
pub use ser::DataSer as Serializer;

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

struct ValueSer<'a>(UTF8<'a>);
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
    type Value = UTF8<'a>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_str(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ValueDe<'a, 'b> {
    type Value = UTF8<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a string value")
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> StdResult<Self::Value, E> {
        let ValueDe(build) = self;
        Ok(build.intern(v).map_err(E::custom)?.into())
    }
}

struct MaybeSer<'a>(Option<UTF8<'a>>);
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
    type Value = Option<UTF8<'a>>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_option(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for MaybeDe<'a, 'b> {
    type Value = Option<UTF8<'a>>;
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

tindalwic_macros::template! { ? Text, List, Group, Member, File ^
    #[derive(Clone, Copy, Debug, Deserialize, EnumCount, IntoStaticStr, VariantArray, VariantNames)]
    #[serde(field_identifier, rename_all = "lowercase")]
    #[strum(serialize_all = "lowercase")]
    enum Each~Fields { [^ ~Name, ] }
    impl FieldHelp for Each~Fields {
        const KIND: &'static str = @Each;
        fn num(self) -> usize { self as usize }

    }
}

tindalwic_macros::template! { ? Data ^
    #[derive(Clone, Copy, Debug, Deserialize, IntoStaticStr, VariantNames)]
    #[serde(variant_identifier)]
    enum Each~Variants { [ ^ ~Name, ] }
    impl VariantHelp for Each~Variants {
        const KIND: &'static str = @Each;
        fn num(self) -> usize { self as usize }

    }
}

// #[tindalwic_macros::each(Text,List,File)]
// impl FieldHelp for FieldsᛳEach {
//     const KIND: &'static str = "ᛳEach";
//     fn num(self) -> usize { self as usize }
//     fn should<'a>(this: &ᛳEach<ᛲ>) -> Flags<FieldsᛳEach> {
//         FieldsᛳEach::assign(|field| match field { `
//             [ ? UTF8<ᛲ>         ^ FieldsᛳEach::ᛳName => !this.ᛳname.is_empty(), ]
//             [ ? Option<UTF8<ᛲ>> ^ FieldsᛳEach::ᛳName => this.ᛳname.is_some(), ]
//             [ ? bool            ^ FieldsᛳEach::ᛳName => this.ᛳname, ]
//         })
//     }
// }
