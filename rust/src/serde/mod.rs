//! implementations of the serde features

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

use crate::alloc::Arena;
use crate::{Comment, UTF8};
use serde::Deserialize;
use tindalwic_macros::serialize_deserialize_seed_visit;

serialize_deserialize_seed_visit! {
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
        fn visit_borrowed_str() {
            Ok(UTF8::wrap(v))
        }
        fn visit_str() {
            Ok(UTF8::wrap(arena.intern(v)))
        }
    }
}

serialize_deserialize_seed_visit! {
    #[expecting = "a comment (or null)"]
    #[deserialize_option]
    impl Comment {
        fn serialize() {
            match this {
                None => s.serialize_none(),
                Some(comment) => UTF8Ser(comment.utf8).serialize(s),
            }
        }
        fn visit_none() {
            Ok(None)
        }
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
    UTF8,
    Epilog,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum ListFields {
    Prolog,
    Items,
    Epilog,
}

// #[derive(Deserialize)]
// #[serde(field_identifier, rename_all = "lowercase")]
// enum DictFields {
//     Prolog,
//     Items,
//     Epilog,
// }
