extern crate alloc;

use super::{ArenaSeed, CommentDe, CommentSer, UTF8De, UTF8Ser, seeded};
use super::{DictFields, EntryFields, FileFields, ItemVariants, ListFields, TextFields};
use crate::alloc::Arena;
use crate::{Dict, Entry, File, Item, List, Name, Text};
use alloc::string::{String, ToString};
use core::cell::Cell;
use serde::de::{DeserializeSeed, Error, VariantAccess};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

seeded! {
    #[expecting = "a verbose Item (Text, List, or Dict)"]
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
                ItemVariants::Text => Item::Text(access.newtype_variant_seed(TextDe(arena))?),
                ItemVariants::List => Item::List(access.newtype_variant_seed(ListDe(arena))?),
                ItemVariants::Dict => Item::Dict(access.newtype_variant_seed(DictDe(arena))?),
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose Text: string value + epilog comment"]
    #[deserialize_struct]
    impl Text {
        fn serialize() {
            let mut fields = s.serialize_struct("Text", 2)?;
            fields.serialize_field("value", &UTF8Ser(this.utf8))?;
            fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let err = || Error::invalid_length(2, &self);
            Ok(Text {
                utf8: seq.next_element_seed(UTF8De(arena))?.ok_or_else(err)?,
                epilog: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
            })
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
                        value = Some(map.next_value_seed(UTF8De(arena))?);
                    }
                    TextFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                }
            }
            Ok(Text {
                utf8: value.ok_or_else(|| Error::missing_field("value"))?,
                epilog: epilog.ok_or_else(|| Error::missing_field("epilog"))?,
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "sequence of verbose Items"]
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
            while let Some(item) = seq.next_element_seed(ItemDe(arena))? {
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
    #[expecting = "a verbose List: prolog + array of items + epilog"]
    #[deserialize_struct]
    impl List {
        fn serialize() {
            let mut fields = s.serialize_struct("List", 3)?;
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            fields.serialize_field("array", &ItemsSer(this.cells))?;
            fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let err = || Error::invalid_length(3, &self);
            Ok(List {
                prolog: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
                cells: seq.next_element_seed(ItemsDe(arena))?.ok_or_else(err)?,
                epilog: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
            })
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
                        prolog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    ListFields::Array => {
                        if array.is_some() {
                            return Err(Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(ItemsDe(arena))?);
                    }
                    ListFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                }
            }
            Ok(List {
                prolog: prolog.ok_or_else(|| Error::missing_field("prolog"))?,
                cells: array.ok_or_else(|| Error::missing_field("array"))?,
                epilog: epilog.ok_or_else(|| Error::missing_field("epilog"))?,
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose entry in a dictionary: gap + before + key + item"]
    #[deserialize_struct]
    impl Entry {
        fn serialize() {
            let mut fields = s.serialize_struct("Entry", 4)?;
            fields.serialize_field("gap", &this.name.gap)?;
            fields.serialize_field("before", &CommentSer(this.name.before))?;
            fields.serialize_field("key", this.name.key)?;
            fields.serialize_field("item", &ItemSer(this.item))?;
            fields.end()
        }
        fn visit_seq() {
            let err = || Error::invalid_length(4, &self);
            Ok(Entry {
                name: Name {
                    gap: seq.next_element()?.ok_or_else(err)?,
                    before: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
                    key: arena.intern(&seq.next_element::<String>()?.ok_or_else(err)?),
                },
                item: seq.next_element_seed(ItemDe(arena))?.ok_or_else(err)?,
            })
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
                        before = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    EntryFields::Key => {
                        if key.is_some() {
                            return Err(Error::duplicate_field("key"));
                        }
                        key = Some(arena.intern(&map.next_value::<String>()?));
                    }
                    EntryFields::Item => {
                        if item.is_some() {
                            return Err(Error::duplicate_field("item"));
                        }
                        item = Some(map.next_value_seed(ItemDe(arena))?);
                    }
                }
            }
            Ok(Entry {
                name: Name {
                    gap: gap.ok_or_else(|| Error::missing_field("gap"))?,
                    before: before.ok_or_else(|| Error::missing_field("before"))?,
                    key: key.ok_or_else(|| Error::missing_field("key"))?,
                },
                item: item.ok_or_else(|| Error::missing_field("item"))?,
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "sequence of verbose Entry"]
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
            while let Some(entry) = seq.next_element_seed(EntryDe(arena))? {
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
    #[expecting = "a verbose Dict: prolog + array of entries + epilog"]
    #[deserialize_struct]
    impl Dict {
        fn serialize() {
            let mut fields = s.serialize_struct("Dict", 3)?;
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            fields.serialize_field("array", &EntriesSer(this.cells))?;
            fields.serialize_field("epilog", &CommentSer(this.epilog))?;
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
                        prolog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    DictFields::Array => {
                        if array.is_some() {
                            return Err(Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(EntriesDe(arena))?);
                    }
                    DictFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                }
            }
            Ok(Dict {
                prolog: prolog.ok_or_else(|| Error::missing_field("prolog"))?,
                cells: array.ok_or_else(|| Error::missing_field("array"))?,
                epilog: epilog.ok_or_else(|| Error::missing_field("epilog"))?,
            })
        }
        fn visit_seq() {
            let err = || Error::invalid_length(3, &self);
            Ok(Dict {
                prolog: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
                cells: seq.next_element_seed(EntriesDe(arena))?.ok_or_else(err)?,
                epilog: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose File: hashbang + prolog + array of entries"]
    #[deserialize_struct]
    impl File {
        fn serialize() {
            let mut fields = s.serialize_struct("File", 3)?;
            fields.serialize_field("hashbang", &CommentSer(this.hashbang))?;
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            fields.serialize_field("array", &EntriesSer(this.cells))?;
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
                        hashbang = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    FileFields::Prolog => {
                        if prolog.is_some() {
                            return Err(Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    FileFields::Array => {
                        if array.is_some() {
                            return Err(Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(EntriesDe(arena))?);
                    }
                }
            }
            Ok(File {
                hashbang: hashbang.ok_or_else(|| Error::missing_field("hashbang"))?,
                prolog: prolog.ok_or_else(|| Error::missing_field("prolog"))?,
                cells: array.ok_or_else(|| Error::missing_field("array"))?,
            })
        }
        fn visit_seq() {
            let err = || Error::invalid_length(3, &self);
            Ok(File {
                hashbang: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
                prolog: seq.next_element_seed(CommentDe(arena))?.ok_or_else(err)?,
                cells: seq.next_element_seed(EntriesDe(arena))?.ok_or_else(err)?,
            })
        }
    }
} // !seeded

/// serialize all fields, avoiding "skip_serializing_if"
pub struct Verbose<'a>(pub File<'a>);
impl<'a> Serialize for Verbose<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Verbose(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'de, 'a: 'de> ArenaSeed<'de, 'a> for Verbose<'a> {
    fn seed(arena: &'de Arena<'a>) -> impl DeserializeSeed<'de, Value = File<'a>> {
        FileDe(arena)
    }
}
