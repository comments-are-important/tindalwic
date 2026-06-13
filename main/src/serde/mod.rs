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

pub mod format {
    //! our serde data format
    extern crate alloc;
    use crate::parse::{Build, Parse};
    use crate::{Entry, File, Item, Value};
    use ::serde::{de, ser};
    use alloc::string::{String, ToString};
    use core::fmt::{self, Display};
    use de::{Deserializer as _, Error as _};

    /// specialized to Err([Error])
    pub type Result<T> = core::result::Result<T, Error>;

    /// payload is just a String message
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

    /// build a tindalwic [`Item`] from any `T: Serialize`
    struct ItemSer<'b, 'a> {
        build: &'b mut dyn Build<'a>,
    }
    impl<'c, 'b, 'a> ser::Serializer for &'c mut ItemSer<'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        type SerializeSeq = SerializeSeq<'c, 'b, 'a>;
        type SerializeTuple = SerializeSeq<'c, 'b, 'a>;
        type SerializeTupleStruct = SerializeSeq<'c, 'b, 'a>;
        type SerializeTupleVariant = SerializeTupleVariant<'c, 'b, 'a>;
        type SerializeMap = SerializeMap<'c, 'b, 'a>;
        type SerializeStruct = SerializeStruct<'c, 'b, 'a>;
        type SerializeStructVariant = SerializeStructVariant<'c, 'b, 'a>;

        fn serialize_bool(self, v: bool) -> Result<Item<'a>> {
            Ok(Item::text(if v { "true" } else { "false" }))
        }
        fn serialize_i8(self, v: i8) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_i16(self, v: i16) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_i32(self, v: i32) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_i64(self, v: i64) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_i128(self, v: i128) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_u8(self, v: u8) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_u16(self, v: u16) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_u32(self, v: u32) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_u64(self, v: u64) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_u128(self, v: u128) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_f32(self, v: f32) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_f64(self, v: f64) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }

        fn serialize_char(self, v: char) -> Result<Item<'a>> {
            Ok(Item::text(
                self.build.intern(&v.to_string()).map_err(Error::new)?,
            ))
        }
        fn serialize_str(self, v: &str) -> Result<Item<'a>> {
            Ok(Item::text(self.build.intern(v).map_err(Error::new)?))
        }
        fn serialize_bytes(self, v: &[u8]) -> Result<Item<'a>> {
            // differs from Neutered on purpose: no lossy Latin-1 round-trip.
            // ItemDe::deserialize_bytes hands back the text's UTF-8 bytes, so only
            // valid UTF-8 can round-trip; reject the rest rather than lie.
            match core::str::from_utf8(v) {
                Ok(s) => Ok(Item::text(self.build.intern(s).map_err(Error::new)?)),
                Err(_) => Err(Error::new(
                    "non-UTF-8 bytes cannot be encoded as tindalwic text",
                )),
            }
        }

        // empty Text -> None / unit on the read side
        fn serialize_none(self) -> Result<Item<'a>> {
            Ok(Item::default())
        }
        fn serialize_unit(self) -> Result<Item<'a>> {
            Ok(Item::default())
        }
        fn serialize_unit_struct(self, _name: &'static str) -> Result<Item<'a>> {
            // NB: ItemDe::deserialize_unit_struct routes to deserialize_any, so this
            // won't round-trip until that's fixed to hit the unit path.
            Ok(Item::default())
        }

        // Some(x) and newtype(x) are encoded as bare x
        fn serialize_some<T: ?Sized + ser::Serialize>(self, value: &T) -> Result<Item<'a>> {
            value.serialize(self)
        }
        fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Item<'a>> {
            value.serialize(self)
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _idx: u32,
            variant: &'static str,
        ) -> Result<Item<'a>> {
            Ok(Item::text(self.build.intern(variant).map_err(Error::new)?))
        }
        fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
            self,
            _name: &'static str,
            _idx: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Item<'a>> {
            let inner = value.serialize(&mut *self)?;
            let key = self.build.intern(variant).map_err(Error::new)?;
            self.build
                .push_entry(Entry {
                    key: key.into(),
                    item: inner,
                    ..Default::default()
                })
                .map_err(Error::new)?;
            let cells = self.build.finish_entries(1).map_err(Error::new)?;
            Ok(Item::dict(cells))
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
            Ok(SerializeSeq {
                ser: self,
                count: 0,
            })
        }
        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
            Ok(SerializeSeq {
                ser: self,
                count: 0,
            })
        }
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct> {
            Ok(SerializeSeq {
                ser: self,
                count: 0,
            })
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _idx: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant> {
            let variant = self.build.intern(variant).map_err(Error::new)?;
            Ok(SerializeTupleVariant {
                ser: self,
                variant,
                count: 0,
            })
        }
        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
            Ok(SerializeMap {
                ser: self,
                key: None,
                count: 0,
            })
        }
        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct> {
            Ok(SerializeStruct {
                ser: self,
                count: 0,
            })
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _idx: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant> {
            let variant = self.build.intern(variant).map_err(Error::new)?;
            Ok(SerializeStructVariant {
                ser: self,
                variant,
                count: 0,
            })
        }
    }

    // every element is built and pushed eagerly, before the next one, so the shared
    // CellVec stacks stay strictly LIFO. don't batch.
    struct SerializeSeq<'c, 'b, 'a> {
        ser: &'c mut ItemSer<'b, 'a>,
        count: usize,
    }
    impl<'c, 'b, 'a> SerializeSeq<'c, 'b, 'a> {
        fn push<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            let item = value.serialize(&mut *self.ser)?;
            self.ser.build.push_item(item).map_err(Error::new)?;
            self.count += 1;
            Ok(())
        }
        fn list(self) -> Result<Item<'a>> {
            Ok(Item::list(
                self.ser
                    .build
                    .finish_items(self.count)
                    .map_err(Error::new)?,
            ))
        }
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeSeq for SerializeSeq<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, v: &T) -> Result<()> {
            self.push(v)
        }
        fn end(self) -> Result<Item<'a>> {
            self.list()
        }
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeTuple for SerializeSeq<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, v: &T) -> Result<()> {
            self.push(v)
        }
        fn end(self) -> Result<Item<'a>> {
            self.list()
        }
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeTupleStruct for SerializeSeq<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, v: &T) -> Result<()> {
            self.push(v)
        }
        fn end(self) -> Result<Item<'a>> {
            self.list()
        }
    }

    // {variant: [..]}  — the double-close case; inner finish_items pops before the
    // outer finish_entries(1), so LIFO holds.
    struct SerializeTupleVariant<'c, 'b, 'a> {
        ser: &'c mut ItemSer<'b, 'a>,
        variant: &'a str,
        count: usize,
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeTupleVariant for SerializeTupleVariant<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            let item = value.serialize(&mut *self.ser)?;
            self.ser.build.push_item(item).map_err(Error::new)?;
            self.count += 1;
            Ok(())
        }
        fn end(self) -> Result<Item<'a>> {
            let list = Item::list(
                self.ser
                    .build
                    .finish_items(self.count)
                    .map_err(Error::new)?,
            );
            self.ser
                .build
                .push_entry(Entry {
                    key: self.variant.into(),
                    item: list,
                    ..Default::default()
                })
                .map_err(Error::new)?;
            let cells = self.ser.build.finish_entries(1).map_err(Error::new)?;
            Ok(Item::dict(cells))
        }
    }

    struct SerializeMap<'c, 'b, 'a> {
        ser: &'c mut ItemSer<'b, 'a>,
        key: Option<Value<'a>>,
        count: usize,
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeMap for SerializeMap<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_key<T: ?Sized + ser::Serialize>(&mut self, key: &T) -> Result<()> {
            match key.serialize(&mut *self.ser)? {
                Item::Text { value, .. } => {
                    self.key = Some(value);
                    Ok(())
                }
                _ => Err(Error::new("map key must serialize to a string")),
            }
        }
        fn serialize_value<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            let item = value.serialize(&mut *self.ser)?;
            let key = self
                .key
                .take()
                .ok_or_else(|| Error::new("value before key"))?;
            self.ser
                .build
                .push_entry(Entry {
                    key,
                    item,
                    ..Default::default()
                })
                .map_err(Error::new)?;
            self.count += 1;
            Ok(())
        }
        fn end(self) -> Result<Item<'a>> {
            Ok(Item::dict(
                self.ser
                    .build
                    .finish_entries(self.count)
                    .map_err(Error::new)?,
            ))
        }
    }

    struct SerializeStruct<'c, 'b, 'a> {
        ser: &'c mut ItemSer<'b, 'a>,
        count: usize,
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeStruct for SerializeStruct<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_field<T: ?Sized + ser::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()> {
            let item = value.serialize(&mut *self.ser)?;
            let key = self.ser.build.intern(key).map_err(Error::new)?;
            self.ser
                .build
                .push_entry(Entry {
                    key: key.into(),
                    item,
                    ..Default::default()
                })
                .map_err(Error::new)?;
            self.count += 1;
            Ok(())
        }
        fn end(self) -> Result<Item<'a>> {
            let cells = self
                .ser
                .build
                .finish_entries(self.count)
                .map_err(Error::new)?;
            Ok(Item::dict(cells))
        }
    }

    // {variant: {..}}
    struct SerializeStructVariant<'c, 'b, 'a> {
        ser: &'c mut ItemSer<'b, 'a>,
        variant: &'a str,
        count: usize,
    }
    impl<'c, 'b, 'a> ::serde::ser::SerializeStructVariant for SerializeStructVariant<'c, 'b, 'a> {
        type Ok = Item<'a>;
        type Error = Error;
        fn serialize_field<T: ?Sized + ser::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()> {
            let item = value.serialize(&mut *self.ser)?;
            let key = self.ser.build.intern(key).map_err(Error::new)?;
            self.ser
                .build
                .push_entry(Entry {
                    key: key.into(),
                    item,
                    ..Default::default()
                })
                .map_err(Error::new)?;
            self.count += 1;
            Ok(())
        }
        fn end(self) -> Result<Item<'a>> {
            let dict = Item::dict(
                self.ser
                    .build
                    .finish_entries(self.count)
                    .map_err(Error::new)?,
            );
            self.ser
                .build
                .push_entry(Entry {
                    key: self.variant.into(),
                    item: dict,
                    ..Default::default()
                })
                .map_err(Error::new)?;
            let cells = self.ser.build.finish_entries(1).map_err(Error::new)?;
            Ok(Item::dict(cells))
        }
    }

    /// encode a type that is compatible with dictionary into a tindalwic data file.
    pub fn to_tindalwic<'a, T: ?Sized + ser::Serialize>(
        build: &mut dyn Build<'a>,
        value: &T,
    ) -> Result<String> {
        let item = {
            let mut ser = ItemSer { build };
            value.serialize(&mut ser)?
        };
        let file = File::try_from_dict_without_epilog(&item)
            .ok_or_else(|| Error::new("top-level value must serialize to a map or struct"))?;
        Ok(file.to_string())
    }

    #[derive(Copy, Clone)]
    struct ItemDe<'de, 'a> {
        encoded: &'de str,
        item: Item<'a>,
    }
    impl<'de, 'a> ItemDe<'de, 'a> {
        fn with_item(&self, item: Item<'a>) -> Self {
            ItemDe {
                encoded: self.encoded,
                item,
            }
        }
        fn with_text(&self, value: Value<'a>) -> Self {
            self.with_item(Item::Text {
                value,
                epilog: None,
            })
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
        fn outlive(&self, value: Value<'a>) -> Option<&'de str> {
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
    impl<'de, 'a> de::IntoDeserializer<'de, Error> for ItemDe<'de, 'a> {
        type Deserializer = Self;
        fn into_deserializer(self) -> Self::Deserializer {
            self
        }
    }
    impl<'de, 'a> de::Deserializer<'de> for ItemDe<'de, 'a> {
        type Error = Error;

        fn deserialize_any<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            match self.item {
                Item::Text { value, .. } => {
                    if let Some(verbatim) = self.outlive(value) {
                        v.visit_borrowed_str(verbatim)
                    } else {
                        v.visit_string(value.joined())
                    }
                }
                Item::List { cells, .. } => v.visit_seq(de::value::SeqDeserializer::new(
                    cells.iter().map(|cell| self.with_item(cell.get())),
                )),
                Item::Dict { cells, .. } => {
                    v.visit_map(de::value::MapDeserializer::new(cells.iter().map(|cell| {
                        let Entry { key, item, .. } = cell.get();
                        (self.with_text(key), self.with_item(item))
                    })))
                }
            }
        }

        fn deserialize_bool<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<bool>() {
                v.visit_bool(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i8<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i8>() {
                v.visit_i8(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i16<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i16>() {
                v.visit_i16(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i32<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i32>() {
                v.visit_i32(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i64<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i64>() {
                v.visit_i64(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_i128<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<i128>() {
                v.visit_i128(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u8<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u8>() {
                v.visit_u8(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u16<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u16>() {
                v.visit_u16(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u32<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u32>() {
                v.visit_u32(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u64<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u64>() {
                v.visit_u64(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_u128<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<u128>() {
                v.visit_u128(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_f32<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<f32>() {
                v.visit_f32(value)
            } else {
                self.deserialize_any(v)
            }
        }
        fn deserialize_f64<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Some(value) = self.parse::<f64>() {
                v.visit_f64(value)
            } else {
                self.deserialize_any(v)
            }
        }

        fn deserialize_char<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
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

        fn deserialize_str<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }
        fn deserialize_string<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_bytes<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Item::Text { value, .. } = self.item {
                if let Some(verbatim) = self.outlive(value) {
                    return v.visit_borrowed_bytes(verbatim.as_bytes());
                } else {
                    return v.visit_byte_buf(value.joined().as_bytes().to_vec());
                }
            }
            self.deserialize_any(v)
        }
        fn deserialize_byte_buf<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_bytes(v)
        }

        fn deserialize_option<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
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

        fn deserialize_unit<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            if let Item::Text { value, .. } = self.item {
                if value.is_empty() {
                    return v.visit_unit();
                }
            }
            self.deserialize_any(v)
        }

        fn deserialize_unit_struct<V: de::Visitor<'de>>(
            self,
            _name: &'static str,
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_unit(v)
        }

        fn deserialize_newtype_struct<V: de::Visitor<'de>>(
            self,
            _name: &'static str,
            v: V,
        ) -> Result<V::Value> {
            v.visit_newtype_struct(self)
        }

        fn deserialize_seq<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_tuple<V: de::Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_tuple_struct<V: de::Visitor<'de>>(
            self,
            _name: &'static str,
            _len: usize,
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_map<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_struct<V: de::Visitor<'de>>(
            self,
            _name: &'static str,
            _fields: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_enum<V: de::Visitor<'de>>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            match self.item {
                Item::Text { value, .. } => v.visit_enum(EnumAccess {
                    de: &self,
                    name: value,
                    payload: None,
                }),
                Item::Dict { cells: [entry], .. } => {
                    let Entry { key, item, .. } = entry.get();
                    v.visit_enum(EnumAccess {
                        de: &self,
                        name: key,
                        payload: Some(item),
                    })
                }
                _ => self.deserialize_any(v),
            }
        }

        fn deserialize_identifier<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }

        fn deserialize_ignored_any<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value> {
            self.deserialize_any(v)
        }
    }
    struct EnumAccess<'de, 'a, 'i> {
        de: &'i ItemDe<'de, 'a>, // the container of this encoded enum
        name: Value<'a>,
        payload: Option<Item<'a>>,
    }
    impl<'de, 'a, 'i> ::serde::de::EnumAccess<'de> for EnumAccess<'de, 'a, 'i> {
        type Error = Error;
        type Variant = VariantAccess<'de, 'a, 'i>;

        fn variant_seed<V: de::DeserializeSeed<'de>>(
            self,
            seed: V,
        ) -> Result<(V::Value, Self::Variant)> {
            let EnumAccess { de, name, payload } = self;
            Ok((
                seed.deserialize(self.de.with_text(name))?,
                VariantAccess { de, payload },
            ))
        }
    }
    struct VariantAccess<'de, 'a, 'i> {
        de: &'i ItemDe<'de, 'a>, // the container of this encoded enum
        payload: Option<Item<'a>>,
    }
    impl<'de, 'a, 'i> ::serde::de::VariantAccess<'de> for VariantAccess<'de, 'a, 'i> {
        type Error = Error;

        fn unit_variant(self) -> Result<()> {
            if let Some(item) = self.payload {
                match item {
                    Item::Text { value, .. } => {
                        if value.is_empty() {
                            Ok(())
                        } else {
                            // could Unexpected::Str but that needs borrowed slice
                            // TODO try out this generic message and see if it is good enough
                            Err(Error::invalid_type(
                                de::Unexpected::Other("text"),
                                &"unit variant",
                            ))
                        }
                    }
                    Item::List { .. } => {
                        Err(Error::invalid_type(de::Unexpected::Seq, &"unit variant"))
                    }
                    Item::Dict { .. } => {
                        Err(Error::invalid_type(de::Unexpected::Map, &"unit variant"))
                    }
                }
            } else {
                Ok(())
            }
        }

        fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
            if let Some(item) = self.payload {
                seed.deserialize(self.de.with_item(item))
            } else {
                Err(Error::invalid_type(
                    de::Unexpected::UnitVariant,
                    &"newtype variant",
                ))
            }
        }

        fn tuple_variant<V: de::Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
            if let Some(item) = self.payload {
                self.de.with_item(item).deserialize_seq(v)
            } else {
                Err(Error::invalid_type(
                    de::Unexpected::UnitVariant,
                    &"tuple variant",
                ))
            }
        }

        fn struct_variant<V: de::Visitor<'de>>(
            self,
            fields: &'static [&'static str],
            v: V,
        ) -> Result<V::Value> {
            if let Some(item) = self.payload {
                self.de.with_item(item).deserialize_struct("", fields, v)
            } else {
                Err(Error::invalid_type(
                    de::Unexpected::UnitVariant,
                    &"struct variant",
                ))
            }
        }
    }
    /// decode tindalwic data file into a type that is compatible with dictionary
    pub fn from_tindalwic<'de, T: ::serde::Deserialize<'de>>(
        parse: &mut (dyn Parse<'de> + 'de),
        encoded: &'de str,
    ) -> Result<T> {
        let item = parse
            .first_error(encoded)
            .map_err(Error::custom)?
            .embed_without_hashbang();
        let value = T::deserialize(ItemDe { encoded, item })?;
        Ok(value)
    }
}
