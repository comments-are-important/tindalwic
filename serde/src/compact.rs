extern crate alloc;

use super::{CommentDe, CommentSer, ValueDe, ValueSer};
use super::{DictFields, EntryFields, FileFields, ItemVariants, ListFields, TextFields};
use alloc::string::String;
use serde::de::{DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, Visitor};
use serde::de::{Error as _, VariantAccess as _};
use serde::ser::{Serialize, Serializer};
use serde::ser::{SerializeSeq as _, SerializeStruct as _};
use std::fmt;
use tindalwic::{
    Comment, Entries, Entry, File, Item, Items, Value,
    parse::{Build, Parse},
};

struct ItemSer<'a>(Item<'a>);
impl<'a> Serialize for ItemSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ItemSer(this) = self;
        match this {
            Item::Text { value, epilog } => {
                s.serialize_newtype_variant("Item", 0, "Text", &TextSer((*value, *epilog)))
            }
            Item::List {
                prolog,
                cells,
                epilog,
            } => {
                s.serialize_newtype_variant("Item", 1, "List", &ListSer((*prolog, cells, *epilog)))
            }
            Item::Dict {
                prolog,
                cells,
                epilog,
            } => {
                s.serialize_newtype_variant("Item", 2, "Dict", &DictSer((*prolog, cells, *epilog)))
            }
        }
    }
}
struct ItemDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ItemDe<'a, 'b> {
    type Value = Item<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_enum("Item", &["Text", "List", "Dict"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ItemDe<'a, 'b> {
    type Value = Item<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a compact Item (Text, List, or Dict)")
    }
    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let ItemDe(build) = self;
        let (this, access) = data.variant::<ItemVariants>()?;
        Ok(match this {
            ItemVariants::Text => {
                let (value, epilog) = access.newtype_variant_seed(TextDe(build))?;
                Item::Text { value, epilog }
            }
            ItemVariants::List => {
                let (prolog, cells, epilog) = access.newtype_variant_seed(ListDe(build))?;
                Item::List {
                    prolog,
                    cells,
                    epilog,
                }
            }
            ItemVariants::Dict => {
                let (prolog, cells, epilog) = access.newtype_variant_seed(DictDe(build))?;
                Item::Dict {
                    prolog,
                    cells,
                    epilog,
                }
            }
        })
    }
}

struct TextSer<'a>((Value<'a>, Option<Comment<'a>>));
impl<'a> Serialize for TextSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let TextSer(this) = self;
        let (this_value, this_epilog) = this;
        let value = !this_value.is_empty() as usize;
        let epilog = this_epilog.is_some() as usize;
        let mut fields = s.serialize_struct("Text", value + epilog)?;
        if value != 0 {
            fields.serialize_field("value", &ValueSer(*this_value))?;
        }
        if epilog != 0 {
            fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
        }
        fields.end()
    }
}
struct TextDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for TextDe<'a, 'b> {
    type Value = (Value<'a>, Option<Comment<'a>>);
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("Text", &["value", "epilog"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for TextDe<'a, 'b> {
    type Value = (Value<'a>, Option<Comment<'a>>);
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a compact Text: [string value] + [epilog comment]")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let TextDe(build) = self;
        let mut value = None;
        let mut epilog = None;
        while let Some(field) = map.next_key()? {
            match field {
                TextFields::Value => {
                    if value.is_some() {
                        return Err(A::Error::duplicate_field("value"));
                    }
                    value = Some(map.next_value_seed(ValueDe(build))?);
                }
                TextFields::Epilog => {
                    if epilog.is_some() {
                        return Err(A::Error::duplicate_field("epilog"));
                    }
                    epilog = Some(map.next_value_seed(CommentDe(build))?);
                }
            }
        }
        Ok((value.unwrap_or_else(Value::default), epilog.unwrap_or(None)))
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
        out.write_str("sequence of compact Items")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let ItemsDe(build) = self;
        let mut count = 0usize;
        while let Some(item) = seq.next_element_seed(ItemDe(build))? {
            build.push_item(item).map_err(A::Error::custom)?;
            count += 1;
        }
        Ok(build.finish_items(count).map_err(A::Error::custom)?)
    }
}

struct ListSer<'a>((Option<Comment<'a>>, Items<'a>, Option<Comment<'a>>));
impl<'a> Serialize for ListSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ListSer(this) = self;
        let (this_prolog, this_cells, this_epilog) = this;
        let prolog = this_prolog.is_some() as usize;
        let array = !this_cells.is_empty() as usize;
        let epilog = this_epilog.is_some() as usize;
        let mut fields = s.serialize_struct("List", prolog + array + epilog)?;
        if prolog != 0 {
            fields.serialize_field("prolog", &CommentSer(*this_prolog))?;
        }
        if array != 0 {
            fields.serialize_field("array", &ItemsSer(this_cells))?;
        }
        if epilog != 0 {
            fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
        }
        fields.end()
    }
}
struct ListDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ListDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Items<'a>, Option<Comment<'a>>);
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("List", &["prolog", "items", "epilog"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ListDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Items<'a>, Option<Comment<'a>>);
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a compact List: [prolog] + [array of items] + [epilog]")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let ListDe(build) = self;
        let mut prolog = None;
        let mut array = None;
        let mut epilog = None;
        while let Some(field) = map.next_key()? {
            match field {
                ListFields::Prolog => {
                    if prolog.is_some() {
                        return Err(A::Error::duplicate_field("prolog"));
                    }
                    prolog = Some(map.next_value_seed(CommentDe(build))?);
                }
                ListFields::Array => {
                    if array.is_some() {
                        return Err(A::Error::duplicate_field("array"));
                    }
                    array = Some(map.next_value_seed(ItemsDe(build))?);
                }
                ListFields::Epilog => {
                    if epilog.is_some() {
                        return Err(A::Error::duplicate_field("epilog"));
                    }
                    epilog = Some(map.next_value_seed(CommentDe(build))?);
                }
            }
        }
        Ok((
            prolog.unwrap_or(None),
            array.unwrap_or(&[]),
            epilog.unwrap_or(None),
        ))
    }
}

struct EntrySer<'a>(Entry<'a>);
impl<'a> Serialize for EntrySer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let EntrySer(this) = self;
        let gap = this.gap as usize;
        let before = this.before.is_some() as usize;
        let key = !this.key.is_empty() as usize;
        let item = match this.item {
            Item::Text { value, epilog } => (epilog.is_some() || !value.is_empty()) as usize,
            _ => 1usize,
        };
        let mut fields = s.serialize_struct("Entry", gap + before + key + item)?;
        if gap != 0 {
            fields.serialize_field("gap", &this.gap)?;
        }
        if before != 0 {
            fields.serialize_field("before", &CommentSer(this.before))?;
        }
        if key != 0 {
            if let Some(verbatim) = this.key.verbatim(0) {
                fields.serialize_field("key", verbatim)?;
            } else {
                fields.serialize_field("key", &this.key.joined())?;
            }
        }
        if item != 0 {
            fields.serialize_field("item", &ItemSer(this.item))?;
        }
        fields.end()
    }
}
struct EntryDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for EntryDe<'a, 'b> {
    type Value = Entry<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("Entry", &["gap", "before", "key", "item"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for EntryDe<'a, 'b> {
    type Value = Entry<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a compact entry in a dictionary: [gap] + [before] + [key] + [item]")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let EntryDe(build) = self;
        let mut gap = None;
        let mut before = None;
        let mut key = None;
        let mut item = None;
        while let Some(field) = map.next_key()? {
            match field {
                EntryFields::Gap => {
                    if gap.is_some() {
                        return Err(A::Error::duplicate_field("gap"));
                    }
                    gap = Some(map.next_value()?);
                }
                EntryFields::Before => {
                    if before.is_some() {
                        return Err(A::Error::duplicate_field("before"));
                    }
                    before = Some(map.next_value_seed(CommentDe(build))?);
                }
                EntryFields::Key => {
                    if key.is_some() {
                        return Err(A::Error::duplicate_field("key"));
                    }
                    key = Some(
                        build
                            .intern(&map.next_value::<String>()?)
                            .map_err(A::Error::custom)?
                            .into(),
                    );
                }
                EntryFields::Item => {
                    if item.is_some() {
                        return Err(A::Error::duplicate_field("item"));
                    }
                    item = Some(map.next_value_seed(ItemDe(build))?);
                }
            }
        }
        Ok(Entry {
            gap: gap.unwrap_or(false),
            before: before.unwrap_or(None),
            key: key.unwrap_or_else(Value::default),
            item: item.unwrap_or_else(Item::default),
        })
    }
}

struct EntriesSer<'a>(Entries<'a>);
impl<'a> Serialize for EntriesSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let EntriesSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&EntrySer(cell.get()))?;
        }
        seq.end()
    }
}
struct EntriesDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for EntriesDe<'a, 'b> {
    type Value = Entries<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for EntriesDe<'a, 'b> {
    type Value = Entries<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("sequence of compact Entry")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let EntriesDe(build) = self;
        let mut count = 0usize;
        while let Some(entry) = seq.next_element_seed(EntryDe(build))? {
            build.push_entry(entry).map_err(A::Error::custom)?;
            count += 1;
        }
        Ok(build.finish_entries(count).map_err(A::Error::custom)?)
    }
}

struct DictSer<'a>((Option<Comment<'a>>, Entries<'a>, Option<Comment<'a>>));
impl<'a> Serialize for DictSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let DictSer(this) = self;
        let (this_prolog, this_cells, this_epilog) = this;
        let prolog = this_prolog.is_some() as usize;
        let array = !this_cells.is_empty() as usize;
        let epilog = this_epilog.is_some() as usize;
        let mut fields = s.serialize_struct("Dict", prolog + array + epilog)?;
        if prolog != 0 {
            fields.serialize_field("prolog", &CommentSer(*this_prolog))?;
        }
        if array != 0 {
            fields.serialize_field("array", &EntriesSer(this_cells))?;
        }
        if epilog != 0 {
            fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
        }
        fields.end()
    }
}
struct DictDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for DictDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Entries<'a>, Option<Comment<'a>>);
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("Dict", &["prolog", "entries", "epilog"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for DictDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Entries<'a>, Option<Comment<'a>>);
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a compact Dict: [prolog] + [array of entries] + [epilog]")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let DictDe(build) = self;
        let mut prolog = None;
        let mut array = None;
        let mut epilog = None;
        while let Some(field) = map.next_key()? {
            match field {
                DictFields::Prolog => {
                    if prolog.is_some() {
                        return Err(A::Error::duplicate_field("prolog"));
                    }
                    prolog = Some(map.next_value_seed(CommentDe(build))?);
                }
                DictFields::Array => {
                    if array.is_some() {
                        return Err(A::Error::duplicate_field("array"));
                    }
                    array = Some(map.next_value_seed(EntriesDe(build))?);
                }
                DictFields::Epilog => {
                    if epilog.is_some() {
                        return Err(A::Error::duplicate_field("epilog"));
                    }
                    epilog = Some(map.next_value_seed(CommentDe(build))?);
                }
            }
        }
        Ok((
            prolog.unwrap_or(None),
            array.unwrap_or(&[]),
            epilog.unwrap_or(None),
        ))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct FileSer<'a>(File<'a>);
impl<'a> Serialize for FileSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let FileSer(this) = self;
        let hashbang = this.hashbang.is_some() as usize;
        let prolog = this.prolog.is_some() as usize;
        let array = !this.cells.is_empty() as usize;
        let mut fields = s.serialize_struct("File", hashbang + prolog + array)?;
        if hashbang != 0 {
            fields.serialize_field("hashbang", &CommentSer(this.hashbang))?;
        }
        if prolog != 0 {
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
        }
        if array != 0 {
            fields.serialize_field("array", &EntriesSer(this.cells))?;
        }
        fields.end()
    }
}
struct FileDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("File", &["hashbang", "prolog", "entries"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a compact File: [hashbang] + [prolog] + [array of entries]")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let FileDe(build) = self;
        let mut hashbang = None;
        let mut prolog = None;
        let mut array = None;
        while let Some(field) = map.next_key()? {
            match field {
                FileFields::Hashbang => {
                    if hashbang.is_some() {
                        return Err(A::Error::duplicate_field("hashbang"));
                    }
                    hashbang = Some(map.next_value_seed(CommentDe(build))?);
                }
                FileFields::Prolog => {
                    if prolog.is_some() {
                        return Err(A::Error::duplicate_field("prolog"));
                    }
                    prolog = Some(map.next_value_seed(CommentDe(build))?);
                }
                FileFields::Array => {
                    if array.is_some() {
                        return Err(A::Error::duplicate_field("array"));
                    }
                    array = Some(map.next_value_seed(EntriesDe(build))?);
                }
            }
        }
        Ok(File {
            hashbang: hashbang.unwrap_or(None),
            prolog: prolog.unwrap_or(None),
            cells: array.unwrap_or(&[]),
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

/// serialize only used fields, ala "skip_serializing_if"
pub struct Compact<'a>(pub File<'a>);
impl<'a> Serialize for Compact<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Compact(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Compact<'a> {
    /// call thusly: `Compact::seed(&build).deserialize(...)`
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
