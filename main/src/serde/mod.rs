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

#[cfg(feature = "bumpalo")]
pub mod format {
    //! our serde data format
    extern crate alloc;
    use crate::{Entry, Item};
    use ::serde::Deserializer;
    use ::serde::de::value::{MapDeserializer, SeqDeserializer};
    use ::serde::de::{DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor};
    use ::serde::de::{Error as _, Unexpected};
    use alloc::string::{String, ToString};
    use core::fmt::{self, Display};

    /// specialized to Err([Error])
    pub type Result<T> = core::result::Result<T, Error>;

    /// payload is just a String message
    #[derive(Debug)]
    pub struct Error(String);
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

    #[derive(Copy, Clone)]
    struct ItemDe<'de, 'a> {
        encoded: &'de str,
        item: Item<'a>,
    }
    impl<'de, 'a> ItemDe<'de, 'a> {
        fn with(&self, item: Item<'a>) -> Self {
            ItemDe {
                encoded: self.encoded,
                item,
            }
        }
        fn parse<T: core::str::FromStr>(&self) -> Option<T> {
            if let Item::Text { value, .. } = self.item {
                if let Some(verbatim) = value.verbatim(0) {
                    if let Ok(value) = verbatim.trim().parse::<T>() {
                        return Some(value);
                    }
                } else if let Ok(value) = value.joined().trim().parse::<T>() {
                    return Some(value);
                }
            }
            None
        }
        fn outlive(&self, value: crate::Value<'a>) -> Option<&'de str> {
            if let Some(verbatim) = value.verbatim(0) {
                let base = self.encoded.as_ptr() as usize;
                let mut start = verbatim.as_ptr() as usize;
                if base <= start && start < base + self.encoded.len() {
                    start -= base;
                    return Some(&self.encoded[start..start + verbatim.len()]);
                }
            }
            None
        }
    }
    impl<'de, 'a> IntoDeserializer<'de, Error> for ItemDe<'de, 'a> {
        type Deserializer = Self;
        fn into_deserializer(self) -> Self::Deserializer {
            self
        }
    }
    impl<'de, 'a> Deserializer<'de> for ItemDe<'de, 'a> {
        type Error = Error;

        fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            match self.item {
                Item::Text { value, .. } => {
                    if let Some(verbatim) = self.outlive(value) {
                        v.visit_borrowed_str(verbatim)
                    } else {
                        v.visit_string(value.joined())
                    }
                }
                Item::List { cells, .. } => v.visit_seq(SeqDeserializer::new(
                    cells.iter().map(|cell| self.with(cell.get())),
                )),
                Item::Dict { cells, .. } => {
                    v.visit_map(MapDeserializer::new(cells.iter().map(|cell| {
                        let Entry { key, item, .. } = cell.get();
                        let text = Item::Text {
                            value: key,
                            epilog: None,
                        };
                        (self.with(text), self.with(item))
                    })))
                }
            }
        }

        fn deserialize_bool<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<bool>() {
                v.visit_bool(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i8<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i8>() {
                v.visit_i8(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i16<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i16>() {
                v.visit_i16(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i32<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i32>() {
                v.visit_i32(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i64<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i64>() {
                v.visit_i64(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u8<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u8>() {
                v.visit_u8(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u16<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u16>() {
                v.visit_u16(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u32<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u32>() {
                v.visit_u32(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u64<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u64>() {
                v.visit_u64(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_f32<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<f32>() {
                v.visit_f32(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_f64<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<f64>() {
                v.visit_f64(value)
            } else {
                self.deserialize_any(v)
            }
        }

        fn deserialize_char<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            fn only_char(of: &str) -> Option<char> {
                let mut iter = of.chars();
                if let Some(first) = iter.next() {
                    if iter.next() == None {
                        return Some(first);
                    }
                }
                None
            }
            if let Item::Text { value, .. } = self.item {
                if let Some(verbatim) = value.verbatim(0) {
                    if let Some(only) = only_char(verbatim) {
                        return v.visit_char(only);
                    } else if let Some(trimmed) = only_char(verbatim.trim()) {
                        return v.visit_char(trimmed);
                    }
                } else {
                    let joined = value.joined();
                    if let Some(only) = only_char(&joined) {
                        return v.visit_char(only);
                    } else if let Some(trimmed) = only_char(joined.trim()) {
                        return v.visit_char(trimmed);
                    }
                }
            }
            self.deserialize_any(v)
        }

        fn deserialize_str<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }
        fn deserialize_string<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_bytes<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Item::Text { value, .. } = self.item {
                if let Some(verbatim) = self.outlive(value) {
                    return v.visit_borrowed_bytes(verbatim.as_bytes());
                } else {
                    return v.visit_byte_buf(value.joined().as_bytes().to_vec());
                }
            }
            self.deserialize_any(v)
        }
        fn deserialize_byte_buf<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_bytes(v)
        }

        fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            match self.item {
                Item::Text { value, .. } => {
                    if value.is_empty() {
                        v.visit_none()
                    } else {
                        v.visit_some(self)
                    }
                }
                Item::List { cells, .. } => {
                    if cells.is_empty() {
                        v.visit_none()
                    } else {
                        v.visit_some(self)
                    }
                }
                Item::Dict { cells, .. } => {
                    if cells.is_empty() {
                        v.visit_none()
                    } else {
                        v.visit_some(self)
                    }
                }
            }
        }

        fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Item::Text { value, .. } = self.item {
                if value.is_empty() {
                    return v.visit_unit();
                }
            }
            self.deserialize_any(v)
        }

        fn deserialize_unit_struct<V: Visitor<'de>>(
            self,
            _name: &'static str,
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_newtype_struct<V: Visitor<'de>>(
            self,
            _name: &'static str,
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_tuple_struct<V: Visitor<'de>>(
            self,
            _name: &'static str,
            _len: usize,
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_map<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _name: &'static str,
            _fields: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_enum<V: Visitor<'de>>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            match self.item {
                Item::Text { .. } => v.visit_enum(EnumUnit(self)),
                Item::List { .. } => Err(Error::custom("want enum, have list")),
                Item::Dict { .. } => v.visit_enum(EnumOther(self)),
            }
        }

        fn deserialize_identifier<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }
    }
    struct EnumUnit<'de, 'a>(ItemDe<'de, 'a>);
    impl<'de, 'a> EnumAccess<'de> for EnumUnit<'de, 'a> {
        type Error = Error;
        type Variant = Self;

        fn variant_seed<V: DeserializeSeed<'de>>(
            self,
            seed: V,
        ) -> Result<(V::Value, Self::Variant)> {
            let EnumUnit(de) = self;
            let variant = seed.deserialize(de);
            Ok((variant?, self))
        }
    }
    impl<'de, 'a> VariantAccess<'de> for EnumUnit<'de, 'a> {
        #![allow(unused_variables)]
        type Error = Error;

        fn unit_variant(self) -> Result<()> {
            Ok(())
        }

        fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            ))
        }

        fn tuple_variant<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            ))
        }

        fn struct_variant<V: Visitor<'de>>(
            self,
            fields: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            ))
        }
    }
    struct EnumOther<'de, 'a>(ItemDe<'de, 'a>);
    impl<'de, 'a> EnumAccess<'de> for EnumOther<'de, 'a> {
        type Error = Error;
        type Variant = Self;

        fn variant_seed<V: DeserializeSeed<'de>>(
            self,
            seed: V,
        ) -> Result<(V::Value, Self::Variant)> {
            let EnumOther(de) = self;
            let variant = seed.deserialize(de);
            Ok((variant?, self))
        }
    }
    impl<'de, 'a> VariantAccess<'de> for EnumOther<'de, 'a> {
        type Error = Error;

        fn unit_variant(self) -> Result<()> {
            ::serde::de::Deserialize::deserialize(self.0)
        }

        fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
            seed.deserialize(self.0)
        }

        fn tuple_variant<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
            ::serde::de::Deserializer::deserialize_seq(self.0, v)
        }

        fn struct_variant<V: Visitor<'de>>(
            self,
            fields: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            ::serde::de::Deserializer::deserialize_struct(self.0, "", fields, v)
        }
    }
    /// unpack tindalwic data into any type that can visit {map,seq,str}
    pub fn from_str<'de, T: ::serde::Deserialize<'de>>(encoded: &'de str) -> Result<T> {
        let bump = bumpalo::Bump::new();
        let mut arena = crate::bumpalo::Arena::new(&bump);
        let item = arena
            .describe_errors(encoded, usize::MAX)
            .map_err(Error::custom)?
            .embed_without_hashbang();
        let value = T::deserialize(ItemDe { encoded, item })?;
        Ok(value)
    }
}
