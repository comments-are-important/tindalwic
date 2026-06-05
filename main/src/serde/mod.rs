//! implementations of the serde features

mod compact;
mod neutered;
mod verbose;

pub use compact::Compact;
pub use neutered::Neutered;
pub use verbose::Verbose;

use crate::Comment;
use serde::de::DeserializeSeed as _;

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
        fn offer() {
            if let Some(slice) = this.verbatim(0) {
                let base = input.as_ptr() as usize;
                let start = slice.as_ptr() as usize;
                if base <= start && start < base + input.len() {
                    let offset = start - base;
                    return v.visit_borrowed_str(&input[offset..offset + slice.len()]);
                }
            }
            v.visit_string(this.joined())
        }
        fn serialize() {
            if let Some(slice) = this.verbatim(0) {
                s.serialize_str(slice)
            } else {
                s.serialize_str(&this.joined())
            }
        }
        fn visit_str() {
            Ok(build.intern(v).map_err(E::custom)?.into())
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
            ValueDe::of(build)
                .deserialize(d)
                .map(|value| Some(Comment { value }))
        }
    }
} // !seeded

#[derive(serde::Deserialize)]
#[serde(variant_identifier)]
enum ItemVariants {
    Text,
    List,
    Dict,
}

#[derive(serde::Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum TextFields {
    Value,
    Epilog,
}

#[derive(serde::Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum ListFields {
    Prolog,
    Array,
    Epilog,
}

#[derive(serde::Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntryFields {
    Gap,
    Before,
    Key,
    Item,
}

#[derive(serde::Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum DictFields {
    Prolog,
    Array,
    Epilog,
}

#[derive(serde::Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum FileFields {
    Hashbang,
    Prolog,
    Array,
}

pub mod err {
    //! the concrete Error use by our serde data format
    extern crate alloc;
    use alloc::string::{String, ToString};
    use core::fmt::{self, Display};
    /// payload is just a String message
    #[derive(Debug)]
    pub struct Error(String);
    /// specialized to Err([Error])
    pub type Result<T> = core::result::Result<T, Error>;

    impl Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl core::error::Error for Error {}
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
}
