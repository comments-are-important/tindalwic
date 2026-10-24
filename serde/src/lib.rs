//! implementations of the serde features

use serde::Deserialize;
use serde::de::{DeserializeSeed, Visitor};
use serde::ser::Serialize;
use std::fmt::{self, Display};
use std::result::Result as StdResult;
use tindalwic::parse::Build;
use tindalwic::text::Lines;

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

struct LinesSer<'a>(Lines<'a>);
impl<'a> Serialize for LinesSer<'a> {
    fn serialize<S>(&self, s: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let LinesSer(this) = self;
        if let Some(slice) = this.verbatim(0) {
            s.serialize_str(slice)
        } else {
            s.serialize_str(&this.joined())
        }
    }
}
struct LinesDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for LinesDe<'a, 'b> {
    type Value = Lines<'a>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_str(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for LinesDe<'a, 'b> {
    type Value = Lines<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a string value")
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> StdResult<Self::Value, E> {
        let LinesDe(build) = self;
        Ok(build.intern(v).map_err(E::custom)?.into())
    }
}

struct CommentSer<'a>(Option<Lines<'a>>);
impl<'a> Serialize for CommentSer<'a> {
    fn serialize<S>(&self, s: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let CommentSer(this) = self;
        match this {
            None => s.serialize_none(),
            Some(value) => s.serialize_some(&LinesSer(*value)),
        }
    }
}
struct CommentDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for CommentDe<'a, 'b> {
    type Value = Option<Lines<'a>>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_option(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for CommentDe<'a, 'b> {
    type Value = Option<Lines<'a>>;
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
        let CommentDe(build) = self;
        LinesDe(build).deserialize(d).map(|value| Some(value))
    }
}

// ==================================================================================

tindalwic_macros::template! { ? struct ^

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(field_identifier, rename_all = "lowercase")]
    enum Entity@Fields { [^ $Name, ] }
    impl Entity@Fields {
        const NAMES: &'static [&'static str] = &[[^ ~Name, ]];
    }


}
tindalwic_macros::template! { ? enum ^

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(variant_identifier)]
    enum Entity@Variants { [ ^ $Name, ] }
    impl Entity@Variants {
        const NAMES: &'static [&'static str] = &[[^ ~Name, ]];
        fn name(&self) -> &'static str { Self::NAMES[(*self) as usize] }

        fn serialize<S, W>(&self, s: S, value: &W) -> StdResult<S::Ok, S::Error>
        where
            S: serde::ser::Serializer,
            W: ?Sized + Serialize,
        {
            s.serialize_newtype_variant(~Entity, (*self) as u32, self.name(), value)
        }
    }
}
