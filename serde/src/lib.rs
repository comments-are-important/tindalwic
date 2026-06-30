//! implementations of the serde features

use serde::Deserialize;
use serde::de::{DeserializeSeed, Error as DeError, Visitor};
use serde::ser::Serialize;
use std::fmt::{self, Display};
use std::result::Result as StdResult;
use tindalwic::{Comment, Value, parse::Build};

pub mod de;
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
    fn visit_str<E: DeError>(self, v: &str) -> StdResult<Self::Value, E> {
        let ValueDe(build) = self;
        Ok(build.intern(v).map_err(E::custom)?.into())
    }
}

struct CommentSer<'a>(Option<Comment<'a>>);

impl<'a> Serialize for CommentSer<'a> {
    fn serialize<S>(&self, s: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let CommentSer(this) = self;
        match this {
            None => s.serialize_none(),
            Some(comment) => s.serialize_some(&ValueSer(comment.value)),
        }
    }
}
struct CommentDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for CommentDe<'a, 'b> {
    type Value = Option<Comment<'a>>;
    fn deserialize<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        d.deserialize_option(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for CommentDe<'a, 'b> {
    type Value = Option<Comment<'a>>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a comment (or null)")
    }
    fn visit_none<E: DeError>(self) -> StdResult<Self::Value, E> {
        Ok(None)
    }
    fn visit_some<D>(self, d: D) -> StdResult<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let CommentDe(build) = self;
        ValueDe(build)
            .deserialize(d)
            .map(|value| Some(Comment { value }))
    }
}

#[derive(Deserialize)]
#[serde(variant_identifier)]
enum ItemVariants {
    Text,
    List,
    Dict,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum TextFields {
    Value,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum ListFields {
    Prolog,
    Array,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntryFields {
    Gap,
    Before,
    Key,
    Item,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum DictFields {
    Prolog,
    Array,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum FileFields {
    Hashbang,
    Prolog,
    Array,
}
