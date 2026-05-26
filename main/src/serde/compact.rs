extern crate alloc;

use super::{CommentDe, CommentSer, UTF8De, UTF8Ser, seeded};
use super::{DictFields, EntryFields, FileFields, ItemVariants, ListFields, TextFields};
use crate::{Dict, Entry, File, Item, List, Name, Text, UTF8};
use alloc::string::{String, ToString};
use core::cell::Cell;
use serde::de::{Error, VariantAccess};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

seeded! {
    #[expecting = "a compact Item (Text, List, or Dict)"]
    #[deserialize_enum]
    impl Item {
        fn serialize() {
            match this {
                Item::Text(text) => s.serialize_newtype_variant("Item", 0, "Text", &TextSer(*text)),
                Item::List(list) => s.serialize_newtype_variant("Item", 1, "List", &ListSer(*list)),
                Item::Dict(dict) => s.serialize_newtype_variant("Item", 2, "Dict", &DictSer(*dict)),
            }
        }
        fn visit_enum() {
            let (this, access) = data.variant::<ItemVariants>()?;
            Ok(match this {
                ItemVariants::Text => Item::Text(access.newtype_variant_seed(TextDe::of(arena))?),
                ItemVariants::List => Item::List(access.newtype_variant_seed(ListDe::of(arena))?),
                ItemVariants::Dict => Item::Dict(access.newtype_variant_seed(DictDe::of(arena))?),
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact Text: [string value] + [epilog comment]"]
    #[deserialize_struct]
    impl Text {
        fn serialize() {
            let value = !this.utf8.is_empty() as usize;
            let epilog = this.epilog.is_some() as usize;
            let mut fields = s.serialize_struct("Text", value + epilog)?;
            if value != 0 {
                fields.serialize_field("value", &UTF8Ser(this.utf8))?;
            }
            if epilog != 0 {
                fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            }
            fields.end()
        }
        fn visit_seq() {
            Err(Error::custom("visitor wants seq of fields, use Verbose"))
        }
        fn visit_map() {
            let mut value = None;
            let mut epilog = None;
            while let Some(field) = map.next_key()? {
                match field {
                    TextFields::Value => {
                        if value.is_some() {
                            return Err(Error::duplicate_field("value"));
                        }
                        value = Some(map.next_value_seed(UTF8De::of(arena))?);
                    }
                    TextFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                }
            }
            Ok(Text {
                utf8: value.unwrap_or_else(|| UTF8::wrap("")),
                epilog: epilog.unwrap_or(None),
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "sequence of compact Items"]
    #[deserialize_seq]
    impl Items {
        fn serialize() {
            let mut seq = s.serialize_seq(Some(this.len()))?;
            for cell in this.iter() {
                seq.serialize_element(&ItemSer(cell.get()))?;
            }
            seq.end()
        }
        fn visit_seq() {
            let mut count = 0usize;
            while let Some(item) = seq.next_element_seed(ItemDe::of(arena))? {
                arena
                    .item(item)
                    .map_err(|err| Error::custom(err.to_string()))?;
                count += 1;
            }
            Ok(arena
                .list(count)
                .map_err(|err| Error::custom(err.to_string()))?
                .cells)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact List: [prolog] + [array of items] + [epilog]"]
    #[deserialize_struct]
    impl List {
        fn serialize() {
            let prolog = this.prolog.is_some() as usize;
            let array = !this.cells.is_empty() as usize;
            let epilog = this.epilog.is_some() as usize;
            let mut fields = s.serialize_struct("List", prolog + array + epilog)?;
            if prolog != 0 {
                fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            }
            if array != 0 {
                fields.serialize_field("array", &ItemsSer(this.cells))?;
            }
            if epilog != 0 {
                fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            }
            fields.end()
        }
        fn visit_seq() {
            Err(Error::custom("visitor wants seq of fields, use Verbose"))
        }
        fn visit_map() {
            let mut prolog = None;
            let mut array = None;
            let mut epilog = None;
            while let Some(field) = map.next_key()? {
                match field {
                    ListFields::Prolog => {
                        if prolog.is_some() {
                            return Err(Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                    ListFields::Array => {
                        if array.is_some() {
                            return Err(Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(ItemsDe::of(arena))?);
                    }
                    ListFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                }
            }
            Ok(List {
                prolog: prolog.unwrap_or(None),
                cells: array.unwrap_or(&[]),
                epilog: epilog.unwrap_or(None),
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact entry in a dictionary: [gap] + [before] + [key] + item"]
    #[deserialize_struct]
    impl Entry {
        fn serialize() {
            let gap = this.name.gap as usize;
            let before = this.name.before.is_some() as usize;
            let key = !this.name.key.is_empty() as usize;
            let item = match this.item {
                Item::Text(text) => text.has_content() as usize,
                _ => 1usize,
            };
            let mut fields = s.serialize_struct("Entry", gap + before + key + item)?;
            if gap != 0 {
                fields.serialize_field("gap", &this.name.gap)?;
            }
            if before != 0 {
                fields.serialize_field("before", &CommentSer(this.name.before))?;
            }
            if key != 0 {
                fields.serialize_field("key", this.name.key)?;
            }
            if item != 0 {
                fields.serialize_field("item", &ItemSer(this.item))?;
            }
            fields.end()
        }
        fn visit_seq() {
            Err(Error::custom("visitor wants seq of fields, use Verbose"))
        }
        fn visit_map() {
            let mut gap = None;
            let mut before = None;
            let mut key = None;
            let mut item = None;
            while let Some(field) = map.next_key()? {
                match field {
                    EntryFields::Gap => {
                        if gap.is_some() {
                            return Err(Error::duplicate_field("gap"));
                        }
                        gap = Some(map.next_value()?);
                    }
                    EntryFields::Before => {
                        if before.is_some() {
                            return Err(Error::duplicate_field("before"));
                        }
                        before = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                    EntryFields::Key => {
                        if key.is_some() {
                            return Err(Error::duplicate_field("key"));
                        }
                        key = Some(arena.str(&map.next_value::<String>()?));
                    }
                    EntryFields::Item => {
                        if item.is_some() {
                            return Err(Error::duplicate_field("item"));
                        }
                        item = Some(map.next_value_seed(ItemDe::of(arena))?);
                    }
                }
            }
            Ok(Entry {
                name: Name {
                    gap: gap.unwrap_or(false),
                    before: before.unwrap_or(None),
                    key: key.unwrap_or(""),
                },
                item: item.unwrap_or_else(|| Item::Text(Text::wrap(""))),
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "sequence of compact Entry"]
    #[deserialize_seq]
    impl Entries {
        fn serialize() {
            let mut seq = s.serialize_seq(Some(this.len()))?;
            for cell in this.iter() {
                seq.serialize_element(&EntrySer(cell.get()))?;
            }
            seq.end()
        }
        fn visit_seq() {
            let mut count = 0usize;
            while let Some(entry) = seq.next_element_seed(EntryDe::of(arena))? {
                arena
                    .entry(entry)
                    .map_err(|err| Error::custom(err.to_string()))?;
                count += 1;
            }
            Ok(arena
                .dict(count)
                .map_err(|err| Error::custom(err.to_string()))?
                .cells)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact Dict: [prolog] + [array of entries] + [epilog]"]
    #[deserialize_struct]
    impl Dict {
        fn serialize() {
            let prolog = this.prolog.is_some() as usize;
            let array = !this.cells.is_empty() as usize;
            let epilog = this.epilog.is_some() as usize;
            let mut fields = s.serialize_struct("Dict", prolog + array + epilog)?;
            if prolog != 0 {
                fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            }
            if array != 0 {
                fields.serialize_field("array", &EntriesSer(this.cells))?;
            }
            if epilog != 0 {
                fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            }
            fields.end()
        }
        fn visit_map() {
            let mut prolog = None;
            let mut array = None;
            let mut epilog = None;
            while let Some(field) = map.next_key()? {
                match field {
                    DictFields::Prolog => {
                        if prolog.is_some() {
                            return Err(Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                    DictFields::Array => {
                        if array.is_some() {
                            return Err(Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(EntriesDe::of(arena))?);
                    }
                    DictFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                }
            }
            Ok(Dict {
                prolog: prolog.unwrap_or(None),
                cells: array.unwrap_or(&[]),
                epilog: epilog.unwrap_or(None),
            })
        }
        fn visit_seq() {
            Err(Error::custom("visitor wants seq of fields, use Verbose"))
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact File: [hashbang] + [prolog] + [array of entries]"]
    #[deserialize_struct]
    impl File {
        fn serialize() {
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
        fn visit_map() {
            let mut hashbang = None;
            let mut prolog = None;
            let mut array = None;
            while let Some(field) = map.next_key()? {
                match field {
                    FileFields::Hashbang => {
                        if hashbang.is_some() {
                            return Err(Error::duplicate_field("hashbang"));
                        }
                        hashbang = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                    FileFields::Prolog => {
                        if prolog.is_some() {
                            return Err(Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                    FileFields::Array => {
                        if array.is_some() {
                            return Err(Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(EntriesDe::of(arena))?);
                    }
                }
            }
            Ok(File {
                hashbang: hashbang.unwrap_or(None),
                prolog: prolog.unwrap_or(None),
                cells: array.unwrap_or(&[]),
            })
        }
        fn visit_seq() {
            Err(Error::custom("visitor wants seq of fields, use Verbose"))
        }
    }
} // !seeded

/// serialize only used fields, ala "skip_serializing_if"
pub struct Compact<'a>(pub File<'a>);
impl<'a> Serialize for Compact<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Compact(this) = self;
        FileSer(*this).serialize(s)
    }
}
#[cfg(feature = "bumpalo")]
impl<'a> Compact<'a> {
    /// call thusly: `Compact::bumpalo_seed(&arena).deserialize(...)`
    pub fn bumpalo_seed<'de>(
        arena: &'de crate::bumpalo::Arena<'a>,
    ) -> impl serde::de::DeserializeSeed<'de, Value = File<'a>>
    where
        'a: 'de,
    {
        FileDe::of(&arena.builder)
    }
}
