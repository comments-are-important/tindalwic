//! adapters for serde

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

use crate::alloc::Arena;
use crate::{Comment, UTF8};
use core::fmt;
use serde::de::{DeserializeSeed, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};

struct UTF8Ser<'a>(UTF8<'a>);
impl<'a> UTF8Ser<'a> {
    fn opt(value: Option<Comment<'a>>) -> Option<UTF8Ser<'a>> {
        match value {
            None => None,
            Some(comment) => Some(UTF8Ser(comment.utf8)),
        }
    }
}
impl<'a> Serialize for UTF8Ser<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let UTF8Ser(this) = self;
        if this.dedent == 0 || this.dedent == usize::MAX {
            s.serialize_str(this.slice)
        } else {
            s.serialize_str(&this.joined())
        }
    }
}

struct UTF8De<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a + 'bump, 'a, 'bump> Visitor<'de> for UTF8De<'de, 'a, 'bump> {
    type Value = UTF8<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a string")
    }
    fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Self::Value, E> {
        Ok(UTF8::wrap(v))
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let UTF8De(arena) = self;
        Ok(UTF8::wrap(arena.intern(v)))
    }
}
impl<'de: 'a + 'bump, 'a, 'bump> DeserializeSeed<'de> for UTF8De<'de, 'a, 'bump> {
    type Value = UTF8<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_str(self)
    }
}
