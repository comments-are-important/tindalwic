//! implementations of the serde features

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

use crate::{Comment, Value};
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
    impl Value {
        fn serialize() {
            if let Some(slice) = this.verbatim(0) {
                s.serialize_str(slice)
            } else {
                s.serialize_str(&this.joined())
            }
        }
        fn visit_str() {
            Ok(arena.str(v).into())
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
                Some(comment) => s.serialize_some(&ValueSer(comment.value)),
            }
        }
        fn visit_none() {
            Ok(None)
        }
        fn visit_some() {
            ValueDe::of(arena)
                .deserialize(d)
                .map(|value| Some(Comment { value }))
        }
    }
} // !seeded

#[derive(Deserialize)]
#[serde(variant_identifier)]
#[allow(dead_code)]
enum ItemVariants {
    Text,
    List,
    Dict,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
#[allow(dead_code)]
enum TextFields {
    Value,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
#[allow(dead_code)]
enum ListFields {
    Prolog,
    Array,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
#[allow(dead_code)]
enum EntryFields {
    Gap,
    Before,
    Key,
    Item,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
#[allow(dead_code)]
enum DictFields {
    Prolog,
    Array,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
#[allow(dead_code)]
enum FileFields {
    Hashbang,
    Prolog,
    Array,
}
