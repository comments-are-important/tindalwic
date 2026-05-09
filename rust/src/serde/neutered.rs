use super::{UTF8De, UTF8Ser};
use crate::alloc::Arena;
use crate::internals::Builder;
use crate::{Dict, Entry, File, Item, List, Text};
use core::fmt;
use serde::Deserialize;
use serde::de::{
    DeserializeSeed, Deserializer, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

struct TextSer<'a>(Text<'a>);
impl<'a> Serialize for TextSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let TextSer(this) = self;
        UTF8Ser(this.utf8).serialize(s)
    }
}
struct TextDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for TextDe<'de, 'a, 'bump> {
    type Value = Text<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        let TextDe(arena) = self;
        let utf8 = UTF8De(arena).deserialize(d)?;
        Ok(Text {
            utf8: utf8,
            epilog: None,
        })
    }
}

#[derive(Deserialize)]
enum ItemTag {
    Text,
    List,
    Dict,
}
struct ItemSer<'a, 'store>(Item<'a, 'store>);
impl<'a, 'store> Serialize for ItemSer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ItemSer(this) = self;
        match this {
            Item::Text(text) => s.serialize_newtype_variant("Item", 0, "Text", &TextSer(*text)),
            Item::List(list) => s.serialize_newtype_variant("Item", 1, "List", &ListSer(*list)),
            Item::Dict(dict) => s.serialize_newtype_variant("Item", 2, "Dict", &DictSer(*dict)),
        }
    }
}
struct ItemDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> Visitor<'de> for ItemDe<'de, 'a, 'bump> {
    type Value = Item<'a, 'bump>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("an Item")
    }
    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let ItemDe(arena) = self;
        let (this, access) = data.variant::<ItemTag>()?;
        Ok(match this {
            ItemTag::Text => Item::Text(access.newtype_variant_seed(TextDe(arena))?),
            ItemTag::List => Item::List(access.newtype_variant_seed(ListDe(arena))?),
            ItemTag::Dict => Item::Dict(access.newtype_variant_seed(DictDe(arena))?),
        })
    }
}
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for ItemDe<'de, 'a, 'bump> {
    type Value = Item<'a, 'bump>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_enum("Item", &["Text", "List", "Dict"], self)
    }
}

struct ListSer<'a, 'store>(List<'a, 'store>);
impl<'a, 'store> Serialize for ListSer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ListSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.cells.len()))?;
        for cell in this.cells.iter() {
            seq.serialize_element(&ItemSer(cell.get()))?;
        }
        seq.end()
    }
}
struct ListDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> Visitor<'de> for ListDe<'de, 'a, 'bump> {
    type Value = List<'a, 'bump>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a list of items")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let ListDe(arena) = self;
        let mut count = 0usize;
        while let Some(item) = seq.next_element_seed(ItemDe(arena))? {
            arena.item(item);
            count += 1;
        }
        let list = arena.list(count).ok_or(Error::custom("out of memory"))?;
        Ok(list)
    }
}
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for ListDe<'de, 'a, 'bump> {
    type Value = List<'a, 'bump>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}

struct DictSer<'a, 'store>(Dict<'a, 'store>);
impl<'a, 'store> Serialize for DictSer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let DictSer(this) = self;
        let mut map = s.serialize_map(Some(this.cells.len()))?;
        for cell in this.cells.iter() {
            let Entry { name, item } = cell.get();
            map.serialize_entry(name.key, &ItemSer(item))?;
        }
        map.end()
    }
}
struct DictDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> Visitor<'de> for DictDe<'de, 'a, 'bump> {
    type Value = Dict<'a, 'bump>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a map of entries")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let DictDe(arena) = self;
        let mut count = 0usize;
        while let Some((key, item)) = map.next_entry_seed(UTF8De(arena), ItemDe(arena))? {
            assert!(key.dedent == 0 || key.dedent == usize::MAX);
            arena.entry(Entry::wrap(key.slice, item));
            count += 1;
        }
        let dict = arena.dict(count).ok_or(Error::custom("out of memory"))?;
        Ok(dict)
    }
}
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for DictDe<'de, 'a, 'bump> {
    type Value = Dict<'a, 'bump>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}

/// serialize to a format that can't remember comments.
pub struct Neutered<'a, 'store>(pub File<'a, 'store>);
impl<'a, 'store> Serialize for Neutered<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(file) = self;
        DictSer(Dict::wrap(file.cells)).serialize(s)
    }
}
impl<'de: 'a + 'store, 'a, 'store> Neutered<'a, 'store> {
    /// deserialize from a format lacking comments
    pub fn deserialize<D: Deserializer<'de>>(
        arena: &'de Arena<'a, 'store>,
        d: D,
    ) -> Result<File<'a, 'store>, D::Error> {
        let dict = d.deserialize_map(DictDe(arena))?;
        Ok(File::wrap(dict.cells))
    }
}
