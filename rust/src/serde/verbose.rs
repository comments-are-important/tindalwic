use super::{ItemVariants, TextFields, UTF8De, UTF8Ser};
use crate::alloc::Arena;
use crate::internals::Builder;
use crate::{Comment, Dict, Entry, File, Item, List, Text};
use core::cell::Cell;
use core::fmt;
use serde::de::{DeserializeSeed, Deserializer, Error};
use serde::de::{EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

super::serialize_deserialize_seed_visit! {
    Item("a verbose Item (Text, List, or Dict)")
    serialize {
        match this {
            Item::Text(text) => s.serialize_newtype_variant("Item", 0, "Text", &TextSer(*text)),
            Item::List(list) => s.serialize_newtype_variant("Item", 1, "List", &ListSer(*list)),
            Item::Dict(dict) => s.serialize_newtype_variant("Item", 2, "Dict", &DictSer(*dict)),
        }
    }
    deserialize_enum(ItemVariants::NAME, ItemVariants::VARIANTS)
    visit_enum {
        let (this, access) = data.variant::<ItemVariants>()?;
        Ok(match this {
            ItemVariants::Text => Item::Text(access.newtype_variant_seed(TextDe(arena))?),
            ItemVariants::List => Item::List(access.newtype_variant_seed(ListDe(arena))?),
            ItemVariants::Dict => Item::Dict(access.newtype_variant_seed(DictDe(arena))?),
        })
    }
}

super::serialize_deserialize_seed_visit! {
    Text("a verbose Text (string and optional epilog comment")
    serialize {
        let mut fields = s.serialize_struct("Text", 2)?;
        fields.serialize_field("utf8", &UTF8Ser(this.utf8))?;
        fields.serialize_field("epilog", &UTF8Ser::opt(this.epilog))?;
        fields.end()
    }
    deserialize_struct(TextFields::NAME, TextFields::FIELDS)
    visit_map {
        let mut utf8 = None;
        let mut epilog = None;
        while let Some(key) = map.next_key()? {
            match key {
            TextFields::UTF8 => {
                if utf8.is_some() {
                    return Err(Error::duplicate_field("utf8"));
                }
                utf8 = Some(map.next_value_seed(UTF8De(arena))?);
            }
            TextFields::Epilog => {
                if epilog.is_some() {
                    return Err(Error::duplicate_field("epilog"));
                }
                epilog = Some(map.next_value_seed(UTF8De(arena))?);
            }
        }
        }
        let utf8 = utf8.ok_or_else(|| Error::missing_field("utf8"))?;
        let epilog = epilog.map(|utf8|Comment{utf8});
        Ok(Text { utf8, epilog })
    }
    visit_seq {
        let utf8 = seq.next_element_seed(UTF8De(arena))?
            .ok_or_else(|| Error::invalid_length(0, &self))?;
        let epilog = seq.next_element_seed(UTF8De(arena))?
            .ok_or_else(|| Error::invalid_length(1, &self))?;
        Ok(Text { utf8, epilog: Some(Comment{utf8:epilog})})
    }
}

struct ItemsSer<'w, 'a: 'w, 's: 'w>(&'w [Cell<Item<'a, 's>>]);
impl<'w, 'a: 'w, 's: 'w> Serialize for ItemsSer<'w, 'a, 's> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ItemsSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&ItemSer(cell.get()))?;
        }
        seq.end()
    }
}

struct ListSer<'a, 'store>(List<'a, 'store>);
impl<'a, 'store> Serialize for ListSer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ListSer(this) = self;
        let mut fields = s.serialize_struct("List", 3)?;
        fields.serialize_field("prolog", &UTF8Ser::opt(this.prolog))?;
        fields.serialize_field("cells", &ItemsSer(this.cells))?;
        fields.serialize_field("epilog", &UTF8Ser::opt(this.epilog))?;
        fields.end()
    }
}
struct ListDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for ListDe<'de, 'a, 'bump> {
    type Value = List<'a, 'bump>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
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

struct EntrySer<'a, 'store>(Entry<'a, 'store>);
impl<'a, 'store> Serialize for EntrySer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let EntrySer(this) = self;
        let mut fields = s.serialize_struct("Entry", 4)?;
        fields.serialize_field("gap", &this.name.gap)?;
        fields.serialize_field("before", &UTF8Ser::opt(this.name.before))?;
        fields.serialize_field("key", this.name.key)?;
        fields.serialize_field("item", &ItemSer(this.item))?;
        fields.end()
    }
}

struct EntriesSer<'w, 'a: 'w, 's: 'w>(&'w [Cell<Entry<'a, 's>>]);
impl<'w, 'a: 'w, 's: 'w> Serialize for EntriesSer<'w, 'a, 's> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let EntriesSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&EntrySer::<'a, 's>(cell.get()))?;
        }
        seq.end()
    }
}

struct DictSer<'a, 'store>(Dict<'a, 'store>);
impl<'a, 'store> Serialize for DictSer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let DictSer(this) = self;
        let mut fields = s.serialize_struct("Dict", 3)?;
        fields.serialize_field("prolog", &UTF8Ser::opt(this.prolog))?;
        fields.serialize_field("cells", &EntriesSer(this.cells))?;
        fields.serialize_field("epilog", &UTF8Ser::opt(this.epilog))?;
        fields.end()
    }
}
struct DictDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for DictDe<'de, 'a, 'bump> {
    type Value = Dict<'a, 'bump>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
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

struct FileSer<'a, 'store>(File<'a, 'store>);
impl<'a, 'store> Serialize for FileSer<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let FileSer(this) = self;
        let mut fields = s.serialize_struct("File", 3)?;
        fields.serialize_field("hashbang", &UTF8Ser::opt(this.hashbang))?;
        fields.serialize_field("prolog", &UTF8Ser::opt(this.prolog))?;
        fields.serialize_field("cells", &EntriesSer(this.cells))?;
        fields.end()
    }
}
struct FileDe<'de, 'a, 'bump>(&'de Arena<'a, 'bump>);
impl<'de: 'a, 'a, 'bump> DeserializeSeed<'de> for FileDe<'de, 'a, 'bump> {
    type Value = File<'a, 'bump>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de: 'a, 'a, 'bump> Visitor<'de> for FileDe<'de, 'a, 'bump> {
    type Value = File<'a, 'bump>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a map of entries")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let FileDe(arena) = self;
        let mut count = 0usize;
        while let Some((key, item)) = map.next_entry_seed(UTF8De(arena), ItemDe(arena))? {
            assert!(key.dedent == 0 || key.dedent == usize::MAX);
            arena.entry(Entry::wrap(key.slice, item));
            count += 1;
        }
        let cells = arena
            .dict(count)
            .ok_or(Error::custom("out of memory"))?
            .cells;
        Ok(File {
            hashbang: None,
            prolog: None,
            cells,
        })
    }
}

/// serialize all fields, avoiding "skip_serializing_if"
pub struct Verbose<'a, 'store>(pub File<'a, 'store>);
impl<'a, 'store> Serialize for Verbose<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Verbose(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'de: 'a + 'store, 'a, 'store> Verbose<'a, 'store> {
    /// deserialize from a format lacking comments
    pub fn deserialize<D: Deserializer<'de>>(
        arena: &'de Arena<'a, 'store>,
        d: D,
    ) -> Result<File<'a, 'store>, D::Error> {
        let dict = FileDe(arena).deserialize(d)?;
        Ok(File::wrap(dict.cells))
    }
}
