//! adapters for serde
#![warn(unused)]

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

use crate::alloc::Arena;
use crate::{Comment, UTF8};
use core::fmt;
use serde::Deserialize;
use serde::de::{DeserializeSeed, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};
use tindalwic_macros::serialize_deserialize_seed_visit;

serialize_deserialize_seed_visit! {
    UTF8("a string value")
    serialize {
        if this.dedent == 0 || this.dedent == usize::MAX {
            s.serialize_str(this.slice)
        } else {
            s.serialize_str(&this.joined())
        }
    }
    deserialize_str
    visit_borrowed_str {
        Ok(UTF8::wrap(v))
    }
    visit_str {
        Ok(UTF8::wrap(arena.intern(v)))
    }
}

impl<'a> UTF8Ser<'a> {
    fn opt(value: Option<Comment<'a>>) -> Option<UTF8Ser<'a>> {
        match value {
            None => None,
            Some(comment) => Some(UTF8Ser(comment.utf8)),
        }
    }
}

#[derive(Deserialize)]
enum ItemVariants {
    Text,
    List,
    Dict,
}
impl ItemVariants {
    const NAME: &'static str = "Item";
    const VARIANTS: &'static [&'static str] = &["Text", "List", "Dict"];
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum TextFields {
    UTF8,
    Epilog,
}
impl TextFields {
    const NAME: &'static str = "Text";
    const FIELDS: &'static [&'static str] = &["utf8", "epilog"];
}
