extern crate alloc;

use super::{CommentDe, CommentSer, ValueDe, ValueSer, seeded};
use super::{DictFields, EntryFields, FileFields, ItemVariants, ListFields, TextFields};
use crate::{Comment, Entry, File, Item, Value};
use alloc::string::{String, ToString};
use core::cell::Cell;
use serde::de::{Error, VariantAccess};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

seeded! {
    #[expecting = "a verbose Item (Text, List, or Dict)"]
    #[deserialize_enum]
    impl Item {
        fn serialize() {
            match this {
                Item::Text { value, epilog } => {
                    s.serialize_newtype_variant("Item", 0, "Text", &TextSer((*value, *epilog)))
                }
                Item::List {
                    prolog,
                    cells,
                    epilog,
                } => s.serialize_newtype_variant(
                    "Item",
                    1,
                    "List",
                    &ListSer((*prolog, cells, *epilog)),
                ),
                Item::Dict {
                    prolog,
                    cells,
                    epilog,
                } => s.serialize_newtype_variant(
                    "Item",
                    2,
                    "Dict",
                    &DictSer((*prolog, cells, *epilog)),
                ),
            }
        }
        fn visit_enum() {
            let (this, access) = data.variant::<ItemVariants>()?;
            Ok(match this {
                ItemVariants::Text => {
                    let (value, epilog) = access.newtype_variant_seed(TextDe::of(arena))?;
                    Item::Text { value, epilog }
                }
                ItemVariants::List => {
                    let (prolog, cells, epilog) = access.newtype_variant_seed(ListDe::of(arena))?;
                    Item::List {
                        prolog,
                        cells,
                        epilog,
                    }
                }
                ItemVariants::Dict => {
                    let (prolog, cells, epilog) = access.newtype_variant_seed(DictDe::of(arena))?;
                    Item::Dict {
                        prolog,
                        cells,
                        epilog,
                    }
                }
            })
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose Text: string value + epilog comment"]
    #[deserialize_struct]
    impl Text {
        fn serialize() {
            let (this_value, this_epilog) = this;
            let mut fields = s.serialize_struct("Text", 2)?;
            fields.serialize_field("value", &ValueSer(*this_value))?;
            fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let err = || Error::invalid_length(2, &TextSer::EXPECTING);
            Ok((
                seq.next_element_seed(ValueDe::of(arena))?.ok_or_else(err)?,
                seq.next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
            ))
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
                        value = Some(map.next_value_seed(ValueDe::of(arena))?);
                    }
                    TextFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                }
            }
            Ok((
                value.ok_or_else(|| Error::missing_field("value"))?,
                epilog.ok_or_else(|| Error::missing_field("epilog"))?,
            ))
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
            while let Some(item) = seq.next_element_seed(ItemDe::of(arena))? {
                arena
                    .push_item(item)
                    .map_err(|err| Error::custom(err.to_string()))?;
                count += 1;
            }
            Ok(arena
                .take_items(count)
                .map_err(|err| Error::custom(err.to_string()))?)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose List: prolog + array of items + epilog"]
    #[deserialize_struct]
    impl List {
        fn serialize() {
            let (this_prolog, this_cells, this_epilog) = this;
            let mut fields = s.serialize_struct("List", 3)?;
            fields.serialize_field("prolog", &CommentSer(*this_prolog))?;
            fields.serialize_field("array", &ItemsSer(this_cells))?;
            fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let err = || Error::invalid_length(3, &ListSer::EXPECTING);
            Ok((
                seq.next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
                seq.next_element_seed(ItemsDe::of(arena))?.ok_or_else(err)?,
                seq.next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
            ))
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
            Ok((
                prolog.ok_or_else(|| Error::missing_field("prolog"))?,
                array.ok_or_else(|| Error::missing_field("array"))?,
                epilog.ok_or_else(|| Error::missing_field("epilog"))?,
            ))
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose entry in a dictionary: gap + before + key + item"]
    #[deserialize_struct]
    impl Entry {
        fn serialize() {
            let first = this.key.lines().next().unwrap_or(""); // TODO key.one_liner
            let mut fields = s.serialize_struct("Entry", 4)?;
            fields.serialize_field("gap", &this.gap)?;
            fields.serialize_field("before", &CommentSer(this.before))?;
            fields.serialize_field("key", first)?;
            fields.serialize_field("item", &ItemSer(this.item))?;
            fields.end()
        }
        fn visit_seq() {
            let err = || Error::invalid_length(4, &EntrySer::EXPECTING);
            Ok(Entry {
                gap: seq.next_element()?.ok_or_else(err)?,
                before: seq
                    .next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
                key: arena
                    .str(&seq.next_element::<String>()?.ok_or_else(err)?)
                    .into(),
                item: seq.next_element_seed(ItemDe::of(arena))?.ok_or_else(err)?,
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
                        before = Some(map.next_value_seed(CommentDe::of(arena))?);
                    }
                    EntryFields::Key => {
                        if key.is_some() {
                            return Err(Error::duplicate_field("key"));
                        }
                        key = Some(arena.str(&map.next_value::<String>()?).into());
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
                gap: gap.ok_or_else(|| Error::missing_field("gap"))?,
                before: before.ok_or_else(|| Error::missing_field("before"))?,
                key: key.ok_or_else(|| Error::missing_field("key"))?,
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
            while let Some(entry) = seq.next_element_seed(EntryDe::of(arena))? {
                arena
                    .push_entry(entry)
                    .map_err(|err| Error::custom(err.to_string()))?;
                count += 1;
            }
            Ok(arena
                .take_entries(count)
                .map_err(|err| Error::custom(err.to_string()))?)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a verbose Dict: prolog + array of entries + epilog"]
    #[deserialize_struct]
    impl Dict {
        fn serialize() {
            let (this_prolog, this_cells, this_epilog) = this;
            let mut fields = s.serialize_struct("Dict", 3)?;
            fields.serialize_field("prolog", &CommentSer(*this_prolog))?;
            fields.serialize_field("array", &EntriesSer(this_cells))?;
            fields.serialize_field("epilog", &CommentSer(*this_epilog))?;
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
            Ok((
                prolog.ok_or_else(|| Error::missing_field("prolog"))?,
                array.ok_or_else(|| Error::missing_field("array"))?,
                epilog.ok_or_else(|| Error::missing_field("epilog"))?,
            ))
        }
        fn visit_seq() {
            let err = || Error::invalid_length(3, &DictSer::EXPECTING);
            Ok((
                seq.next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
                seq.next_element_seed(EntriesDe::of(arena))?
                    .ok_or_else(err)?,
                seq.next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
            ))
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
                hashbang: hashbang.ok_or_else(|| Error::missing_field("hashbang"))?,
                prolog: prolog.ok_or_else(|| Error::missing_field("prolog"))?,
                cells: array.ok_or_else(|| Error::missing_field("array"))?,
            })
        }
        fn visit_seq() {
            let err = || Error::invalid_length(3, &FileSer::EXPECTING);
            Ok(File {
                hashbang: seq
                    .next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
                prolog: seq
                    .next_element_seed(CommentDe::of(arena))?
                    .ok_or_else(err)?,
                cells: seq
                    .next_element_seed(EntriesDe::of(arena))?
                    .ok_or_else(err)?,
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
#[cfg(feature = "bumpalo")]
mod bumpalo {
    use super::{File, FileDe, Verbose};
    use crate::alloc::Intern;
    use crate::parse::Builder;
    impl<'a> Verbose<'a> {
        /// call thusly: `Verbose::bumpalo_seed(&arena).deserialize(...)`
        pub fn bumpalo_seed<'de, 'ib, IB: Intern<'a> + Builder<'a>>(
            arena: &'ib mut IB,
        ) -> impl serde::de::DeserializeSeed<'de, Value = File<'a>> + use<'de, 'a, 'ib, IB>
        {
            FileDe::of(arena)
        }
    }
}
