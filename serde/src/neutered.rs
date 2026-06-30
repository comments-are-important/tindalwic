extern crate alloc;

use super::{ValueDe, ValueSer};
use alloc::string::{String, ToString};
use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, Serializer};
use serde::ser::{SerializeMap as _, SerializeSeq as _};
use std::fmt;
use tindalwic::{
    Entries, Entry, File, Item, Items,
    parse::{Build, Parse},
};

struct ItemSer<'a>(Item<'a>);
impl<'a> Serialize for ItemSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ItemSer(this) = self;
        match this {
            Item::Text { value, .. } => ValueSer(*value).serialize(s),
            Item::List { cells, .. } => ItemsSer(cells).serialize(s),
            Item::Dict { cells, .. } => EntriesSer(cells).serialize(s),
        }
    }
}
struct ItemDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ItemDe<'a, 'b> {
    type Value = Item<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ItemDe<'a, 'b> {
    type Value = Item<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a neutered item (simple value, list, or dictionary)")
    }
    fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Item::text(if v { "true" } else { "false" }))
    }
    fn visit_i8<E: Error>(self, v: i8) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_i16<E: Error>(self, v: i16) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_i32<E: Error>(self, v: i32) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_i128<E: Error>(self, v: i128) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_u16<E: Error>(self, v: u16) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_u32<E: Error>(self, v: u32) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_u128<E: Error>(self, v: u128) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_f32<E: Error>(self, v: f32) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        let mut buffer = ryu::Buffer::new();
        Ok(Item::text(
            build.intern(buffer.format(v)).map_err(E::custom)?,
        ))
    }
    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        let mut buffer = ryu::Buffer::new();
        Ok(Item::text(
            build.intern(buffer.format(v)).map_err(E::custom)?,
        ))
    }
    fn visit_char<E: Error>(self, v: char) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        let value = ValueDe(build).visit_str(v)?;
        Ok(Item::Text {
            value,
            epilog: None,
        })
    }
    fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        let ItemDe(build) = self;
        if v.is_ascii() {
            let value = unsafe { std::str::from_utf8_unchecked(v) };
            Ok(Item::text(build.intern(value).map_err(E::custom)?))
        } else {
            let value: String = v.iter().map(|&b| char::from(b)).collect();
            Ok(Item::text(build.intern(&value).map_err(E::custom)?))
        }
    }
    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Item::default())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let ItemDe(build) = self;
        Ok(Item::list(ItemsDe(build).visit_seq(seq)?))
    }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let ItemDe(build) = self;
        Ok(Item::dict(EntriesDe(build).visit_map(map)?))
    }
}

struct ItemsSer<'a>(Items<'a>);
impl<'a> Serialize for ItemsSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ItemsSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&ItemSer(cell.get()))?;
        }
        seq.end()
    }
}
struct ItemsDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ItemsDe<'a, 'b> {
    type Value = Items<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ItemsDe<'a, 'b> {
    type Value = Items<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a list of neutered items")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let ItemsDe(build) = self;
        let mut count = 0usize;
        while let Some(item) = seq.next_element_seed(ItemDe(build))? {
            build.push_item(item).map_err(A::Error::custom)?;
            count += 1;
        }
        build.finish_items(count).map_err(A::Error::custom)
    }
}

struct EntriesSer<'a>(Entries<'a>);
impl<'a> Serialize for EntriesSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let EntriesSer(this) = self;
        let mut map = s.serialize_map(Some(this.len()))?;
        for cell in this.iter() {
            let Entry { key, item, .. } = cell.get();
            if let Some(verbatim) = key.verbatim(0) {
                map.serialize_entry(verbatim, &ItemSer(item))?;
            } else {
                map.serialize_entry(&key.joined(), &ItemSer(item))?;
            }
        }
        map.end()
    }
}
struct EntriesDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for EntriesDe<'a, 'b> {
    type Value = Entries<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for EntriesDe<'a, 'b> {
    type Value = Entries<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a dictionary (string keys, neutered item values)")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let EntriesDe(build) = self;
        let mut count = 0usize;
        while let Some(key) = map.next_key_seed(ValueDe(build))? {
            let item = map.next_value_seed(ItemDe(build))?;
            let entry = Entry {
                key: if let Some(slice) = key.verbatim(0) {
                    slice
                } else {
                    build.intern(&key.joined()).map_err(A::Error::custom)?
                }
                .into(),
                item,
                ..Default::default()
            };
            build.push_entry(entry).map_err(A::Error::custom)?;
            count += 1;
        }
        build.finish_entries(count).map_err(A::Error::custom)
    }
}

struct FileSer<'a>(File<'a>);
impl<'a> Serialize for FileSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let FileSer(this) = self;
        EntriesSer(this.cells).serialize(s)
    }
}
struct FileDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a file (string keys, neutered item values)")
    }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let FileDe(build) = self;
        let cells = EntriesDe(build).visit_map(map)?;
        Ok(File {
            hashbang: None,
            prolog: None,
            cells,
        })
    }
}

/// serialize to a format that can't remember comments.
pub struct Neutered<'a>(pub File<'a>);
impl<'a> Serialize for Neutered<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Neutered<'a> {
    /// call thusly: `Neutered::seed(&mut parse).deserialize(...)`
    /// the deserialize will likely fail unless parse.builder() supports intern
    pub fn seed<'de, 'b, P: Parse<'a>>(
        parse: &'b mut P,
    ) -> impl DeserializeSeed<'de, Value = File<'a>> + 'b
    where
        'a: 'b,
    {
        FileDe(parse.builder())
    }
}
