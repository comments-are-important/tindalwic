extern crate alloc;

use super::{CommentDe, CommentSer, ValueDe, ValueSer, seeded};
use super::{DictFields, EntryFields, FileFields, ItemVariants, ListFields, TextFields};
use crate::{Comment, Entry, File, Item, Value};
use ::serde::de::{Error as _, VariantAccess as _};
use ::serde::ser::{SerializeSeq as _, SerializeStruct as _};
use alloc::string::String;
use core::cell::Cell;

seeded! {
    #[expecting = "a compact Item (Text, List, or Dict)"]
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
                    let (value, epilog) = access.newtype_variant_seed(TextDe::of(build))?;
                    Item::Text { value, epilog }
                }
                ItemVariants::List => {
                    let (prolog, cells, epilog) = access.newtype_variant_seed(ListDe::of(build))?;
                    Item::List {
                        prolog,
                        cells,
                        epilog,
                    }
                }
                ItemVariants::Dict => {
                    let (prolog, cells, epilog) = access.newtype_variant_seed(DictDe::of(build))?;
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
    #[expecting = "a compact Text: [string value] + [epilog comment]"]
    #[deserialize_struct]
    impl Text {
        fn serialize() {
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
        fn visit_seq() {
            Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
        }
        fn visit_map() {
            let mut value = None;
            let mut epilog = None;
            while let Some(field) = map.next_key()? {
                match field {
                    TextFields::Value => {
                        if value.is_some() {
                            return Err(A::Error::duplicate_field("value"));
                        }
                        value = Some(map.next_value_seed(ValueDe::of(build))?);
                    }
                    TextFields::Epilog => {
                        if epilog.is_some() {
                            return Err(A::Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(build))?);
                    }
                }
            }
            Ok((value.unwrap_or_else(Value::default), epilog.unwrap_or(None)))
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
            while let Some(item) = seq.next_element_seed(ItemDe::of(build))? {
                build.push_item(item).map_err(A::Error::custom)?;
                count += 1;
            }
            Ok(build.finish_items(count).map_err(A::Error::custom)?)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact List: [prolog] + [array of items] + [epilog]"]
    #[deserialize_struct]
    impl List {
        fn serialize() {
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
        fn visit_seq() {
            Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
        }
        fn visit_map() {
            let mut prolog = None;
            let mut array = None;
            let mut epilog = None;
            while let Some(field) = map.next_key()? {
                match field {
                    ListFields::Prolog => {
                        if prolog.is_some() {
                            return Err(A::Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe::of(build))?);
                    }
                    ListFields::Array => {
                        if array.is_some() {
                            return Err(A::Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(ItemsDe::of(build))?);
                    }
                    ListFields::Epilog => {
                        if epilog.is_some() {
                            return Err(A::Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(build))?);
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
} // !seeded

seeded! {
    #[expecting = "a compact entry in a dictionary: [gap] + [before] + [key] + [item]"]
    #[deserialize_struct]
    impl Entry {
        fn serialize() {
            let gap = this.gap as usize;
            let before = this.before.is_some() as usize;
            let key = !this.key.is_empty() as usize;
            let item = match this.item {
                // aggressive, maybe confusing, but appropriate for this mode.
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
                let first = this.key.lines().next().unwrap_or(""); // TODO key.one_liner
                fields.serialize_field("key", first)?;
            }
            if item != 0 {
                fields.serialize_field("item", &ItemSer(this.item))?;
            }
            fields.end()
        }
        fn visit_seq() {
            Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
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
                            return Err(A::Error::duplicate_field("gap"));
                        }
                        gap = Some(map.next_value()?);
                    }
                    EntryFields::Before => {
                        if before.is_some() {
                            return Err(A::Error::duplicate_field("before"));
                        }
                        before = Some(map.next_value_seed(CommentDe::of(build))?);
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
                        item = Some(map.next_value_seed(ItemDe::of(build))?);
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
            while let Some(entry) = seq.next_element_seed(EntryDe::of(build))? {
                build.push_entry(entry).map_err(A::Error::custom)?;
                count += 1;
            }
            Ok(build.finish_entries(count).map_err(A::Error::custom)?)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a compact Dict: [prolog] + [array of entries] + [epilog]"]
    #[deserialize_struct]
    impl Dict {
        fn serialize() {
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
        fn visit_map() {
            let mut prolog = None;
            let mut array = None;
            let mut epilog = None;
            while let Some(field) = map.next_key()? {
                match field {
                    DictFields::Prolog => {
                        if prolog.is_some() {
                            return Err(A::Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe::of(build))?);
                    }
                    DictFields::Array => {
                        if array.is_some() {
                            return Err(A::Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(EntriesDe::of(build))?);
                    }
                    DictFields::Epilog => {
                        if epilog.is_some() {
                            return Err(A::Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe::of(build))?);
                    }
                }
            }
            Ok((
                prolog.unwrap_or(None),
                array.unwrap_or(&[]),
                epilog.unwrap_or(None),
            ))
        }
        fn visit_seq() {
            Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
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
                            return Err(A::Error::duplicate_field("hashbang"));
                        }
                        hashbang = Some(map.next_value_seed(CommentDe::of(build))?);
                    }
                    FileFields::Prolog => {
                        if prolog.is_some() {
                            return Err(A::Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe::of(build))?);
                    }
                    FileFields::Array => {
                        if array.is_some() {
                            return Err(A::Error::duplicate_field("array"));
                        }
                        array = Some(map.next_value_seed(EntriesDe::of(build))?);
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
            Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
        }
    }
} // !seeded

/// serialize only used fields, ala "skip_serializing_if"
pub struct Compact<'a>(pub File<'a>);
impl<'a> ::serde::ser::Serialize for Compact<'a> {
    fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Compact(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Compact<'a> {
    /// call thusly: `Compact::seed(&build).deserialize(...)`
    /// the deserialize will likely fail unless parse.builder() supports intern
    pub fn seed<'de, 'b, P: crate::parse::Parse<'a>>(
        parse: &'b mut P,
    ) -> impl serde::de::DeserializeSeed<'de, Value = File<'a>> + 'b
    where
        'a: 'b,
    {
        FileDe::of(parse.builder())
    }
}
