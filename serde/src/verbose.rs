extern crate alloc;

use super::{CommentDe, CommentSer, ValueDe, ValueSer};
use super::{DictFields, EntryFields, FileFields, ItemVariants, ListFields, TextFields};
use alloc::string::String;
use serde::de::VariantAccess as _;
use serde::de::{
    DeserializeSeed, Deserializer, EnumAccess, Error as DeError, MapAccess, SeqAccess, Visitor,
};
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
        out.write_str("a verbose Item (Text, List, or Dict)")
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
        let mut fields = s.serialize_struct("Text", 2)?;
        fields.serialize_field("value", &ValueSer(*this_value))?;
        fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
        fields.end()
    }
}
struct TextDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'a, 'b> TextDe<'a, 'b> {
    const EXPECTING: &'static str = "a verbose Text: string value + epilog comment";
}
impl<'de, 'a, 'b> DeserializeSeed<'de> for TextDe<'a, 'b> {
    type Value = (Value<'a>, Option<Comment<'a>>);
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("Text", &["value", "epilog"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for TextDe<'a, 'b> {
    type Value = (Value<'a>, Option<Comment<'a>>);
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(TextDe::EXPECTING)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let TextDe(build) = self;
        let err = || A::Error::invalid_length(2, &TextDe::EXPECTING);
        Ok((
            seq.next_element_seed(ValueDe(build))?.ok_or_else(err)?,
            seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
        ))
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
        Ok((
            value.ok_or_else(|| A::Error::missing_field("value"))?,
            epilog.ok_or_else(|| A::Error::missing_field("epilog"))?,
        ))
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
        out.write_str("sequence of verbose Items")
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
        let mut fields = s.serialize_struct("List", 3)?;
        fields.serialize_field("prolog", &CommentSer(*this_prolog))?;
        fields.serialize_field("array", &ItemsSer(this_cells))?;
        fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
        fields.end()
    }
}
struct ListDe<'a, 'b>(&'b mut dyn Build<'a>);

impl<'a, 'b> ListDe<'a, 'b> {
    const EXPECTING: &'static str = "a verbose List: prolog + array of items + epilog";
}
impl<'de, 'a, 'b> DeserializeSeed<'de> for ListDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Items<'a>, Option<Comment<'a>>);
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("List", &["prolog", "items", "epilog"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ListDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Items<'a>, Option<Comment<'a>>);
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(ListDe::EXPECTING)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let ListDe(build) = self;
        let err = || A::Error::invalid_length(3, &ListDe::EXPECTING);
        Ok((
            seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
            seq.next_element_seed(ItemsDe(build))?.ok_or_else(err)?,
            seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
        ))
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
            prolog.ok_or_else(|| A::Error::missing_field("prolog"))?,
            array.ok_or_else(|| A::Error::missing_field("array"))?,
            epilog.ok_or_else(|| A::Error::missing_field("epilog"))?,
        ))
    }
}

struct EntrySer<'a>(Entry<'a>);
impl<'a> Serialize for EntrySer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let EntrySer(this) = self;
        let mut fields = s.serialize_struct("Entry", 4)?;
        fields.serialize_field("gap", &this.gap)?;
        fields.serialize_field("before", &CommentSer(this.before))?;
        if let Some(verbatim) = this.key.verbatim(0) {
            fields.serialize_field("key", verbatim)?;
        } else {
            fields.serialize_field("key", &this.key.joined())?;
        }
        fields.serialize_field("item", &ItemSer(this.item))?;
        fields.end()
    }
}
struct EntryDe<'a, 'b>(&'b mut dyn Build<'a>);

impl<'a, 'b> EntryDe<'a, 'b> {
    const EXPECTING: &'static str = "a verbose entry in a dictionary: gap + before + key + item";
}
impl<'de, 'a, 'b> DeserializeSeed<'de> for EntryDe<'a, 'b> {
    type Value = Entry<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("Entry", &["gap", "before", "key", "item"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for EntryDe<'a, 'b> {
    type Value = Entry<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(EntryDe::EXPECTING)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let EntryDe(build) = self;
        let err = || A::Error::invalid_length(4, &EntryDe::EXPECTING);
        Ok(Entry {
            gap: seq.next_element()?.ok_or_else(err)?,
            before: seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
            key: build
                .intern(&seq.next_element::<String>()?.ok_or_else(err)?)
                .map_err(A::Error::custom)?
                .into(),
            item: seq.next_element_seed(ItemDe(build))?.ok_or_else(err)?,
        })
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
            gap: gap.ok_or_else(|| A::Error::missing_field("gap"))?,
            before: before.ok_or_else(|| A::Error::missing_field("before"))?,
            key: key.ok_or_else(|| A::Error::missing_field("key"))?,
            item: item.ok_or_else(|| A::Error::missing_field("item"))?,
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
        out.write_str("sequence of verbose Entry")
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
        let mut fields = s.serialize_struct("Dict", 3)?;
        fields.serialize_field("prolog", &CommentSer(*this_prolog))?;
        fields.serialize_field("array", &EntriesSer(this_cells))?;
        fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
        fields.end()
    }
}
struct DictDe<'a, 'b>(&'b mut dyn Build<'a>);

impl<'a, 'b> DictDe<'a, 'b> {
    const EXPECTING: &'static str = "a verbose Dict: prolog + array of entries + epilog";
}
impl<'de, 'a, 'b> DeserializeSeed<'de> for DictDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Entries<'a>, Option<Comment<'a>>);
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("Dict", &["prolog", "entries", "epilog"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for DictDe<'a, 'b> {
    type Value = (Option<Comment<'a>>, Entries<'a>, Option<Comment<'a>>);
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(DictDe::EXPECTING)
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
            prolog.ok_or_else(|| A::Error::missing_field("prolog"))?,
            array.ok_or_else(|| A::Error::missing_field("array"))?,
            epilog.ok_or_else(|| A::Error::missing_field("epilog"))?,
        ))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let DictDe(build) = self;
        let err = || A::Error::invalid_length(3, &DictDe::EXPECTING);
        Ok((
            seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
            seq.next_element_seed(EntriesDe(build))?.ok_or_else(err)?,
            seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
        ))
    }
}

struct FileSer<'a>(File<'a>);
impl<'a> Serialize for FileSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let FileSer(this) = self;
        let mut fields = s.serialize_struct("File", 3)?;
        fields.serialize_field("hashbang", &CommentSer(this.hashbang))?;
        fields.serialize_field("prolog", &CommentSer(this.prolog))?;
        fields.serialize_field("array", &EntriesSer(this.cells))?;
        fields.end()
    }
}
struct FileDe<'a, 'b>(&'b mut dyn Build<'a>);

impl<'a, 'b> FileDe<'a, 'b> {
    const EXPECTING: &'static str = "a verbose File: hashbang + prolog + array of entries";
}
impl<'de, 'a, 'b> DeserializeSeed<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_struct("File", &["hashbang", "prolog", "entries"], self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(FileDe::EXPECTING)
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
            hashbang: hashbang.ok_or_else(|| A::Error::missing_field("hashbang"))?,
            prolog: prolog.ok_or_else(|| A::Error::missing_field("prolog"))?,
            cells: array.ok_or_else(|| A::Error::missing_field("array"))?,
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let FileDe(build) = self;
        let err = || A::Error::invalid_length(3, &FileDe::EXPECTING);
        Ok(File {
            hashbang: seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
            prolog: seq.next_element_seed(CommentDe(build))?.ok_or_else(err)?,
            cells: seq.next_element_seed(EntriesDe(build))?.ok_or_else(err)?,
        })
    }
}

/// serialize all fields, avoiding "skip_serializing_if"
pub struct Verbose<'a>(pub File<'a>);
impl<'a> Serialize for Verbose<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Verbose(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Verbose<'a> {
    /// call thusly: `Verbose::seed(&build).deserialize(...)`
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
