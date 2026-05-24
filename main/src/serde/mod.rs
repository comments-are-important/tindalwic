//! implementations of the serde features

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

use crate::alloc::Arena;
use crate::{Comment, File, UTF8};
use serde::Deserialize;
use serde::de::DeserializeSeed;

use tindalwic_macros::serialize_deserialize_seed_visit as seeded;
// normally rustfmt would skip over everything inside the macro invocation,
// so the GNUmakefile rust/fmt target uses `sed` to swap the calls for
// `const _: () = {...};` blocks which are formatted. rustfmt is very picky
// about the opening line - adding a comment marker causes it to be skipped.
// so nobody can use that "const" trick because `sed` will alter it.
// the end curly is marked `// !seeded` because `sed` alters it (semicolon).

seeded! {
    #[expecting = "a string value"]
    #[deserialize_str]
    impl UTF8 {
        fn serialize() {
            if this.dedent == 0 || this.dedent == usize::MAX {
                s.serialize_str(this.slice)
            } else {
                s.serialize_str(&this.joined())
            }
        }
        fn visit_str() {
            Ok(UTF8::wrap(arena.intern(v)))
        }
    }
} // !seeded

seeded! {
    #[expecting = "a comment (or null)"]
    #[deserialize_option]
    impl Comment {
        fn serialize() {
            match this {
                None => s.serialize_none(),
                Some(comment) => s.serialize_some(&UTF8Ser(comment.utf8)),
            }
        }
        fn visit_none() {
            Ok(None)
        }
        fn visit_some() {
            UTF8De(arena)
                .deserialize(d)
                .map(|utf8| Some(Comment { utf8 }))
        }
    }
} // !seeded

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

/// turn a mode into a DeserializeSeed that produces File
pub trait ArenaSeed<'de, 'a: 'de> {
    /// call thusly: `.seed(&arena).deserialize()`
    fn seed(arena: &'de Arena<'a>) -> impl DeserializeSeed<'de, Value = File<'a>>;
}
