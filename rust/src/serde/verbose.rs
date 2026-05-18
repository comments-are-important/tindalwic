use super::{CommentDe, CommentSer, UTF8De, UTF8Ser};
use super::{DictFields, ItemVariants, ListFields, TextFields};
use crate::alloc::Arena;
use crate::internals::Builder;
use crate::{Comment, Dict, Entry, File, Item, List, Text};
use core::cell::Cell;
use serde::de::{DeserializeSeed, Deserializer, Error, VariantAccess};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

super::serialize_deserialize_seed_visit! {
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
}

super::serialize_deserialize_seed_visit! {
    #[expecting = "sequence of verbose Item"]
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
                arena.item(item);
                count += 1;
            }
            Ok(arena.list(count).unwrap().cells)
        }
    }
}

super::serialize_deserialize_seed_visit! {
    #[expecting = "a verbose Text (string + epilog comment)"]
    #[deserialize_struct]
    impl Text {
        fn serialize() {
            let mut fields = s.serialize_struct("Text", 2)?;
            fields.serialize_field("value", &UTF8Ser(this.utf8))?;
            fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let utf8 = seq
                .next_element_seed(UTF8De(arena))?
                .ok_or_else(|| Error::invalid_length(0, &self))?;
            let epilog = seq
                .next_element_seed(CommentDe(arena))?
                .ok_or_else(|| Error::invalid_length(1, &self))?;
            Ok(Text { utf8, epilog })
        }
        fn visit_map() {
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
            let epilog = epilog.map(|utf8| Comment { utf8 });
            Ok(Text { utf8, epilog })
        }
    }
}

super::serialize_deserialize_seed_visit! {
    #[expecting = "a verbose List (prolog, items, epilog)"]
    #[deserialize_struct]
    impl List {
        fn serialize() {
            let mut fields = s.serialize_struct("List", 3)?;
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            fields.serialize_field("items", &ItemsSer(this.cells))?;
            fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let prolog = seq
                .next_element_seed(CommentDe(arena))?
                .ok_or_else(|| Error::invalid_length(1, &self))?;
            let cells = seq
                .next_element_seed(ItemsDe(arena))?
                .ok_or_else(|| Error::invalid_length(0, &self))?;
            let epilog = seq
                .next_element_seed(CommentDe(arena))?
                .ok_or_else(|| Error::invalid_length(1, &self))?;
            Ok(List {
                prolog,
                cells,
                epilog,
            })
        }
        fn visit_map() {
            let mut prolog = None;
            let mut items = None;
            let mut epilog = None;
            while let Some(key) = map.next_key()? {
                match key {
                    ListFields::Prolog => {
                        if prolog.is_some() {
                            return Err(Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    ListFields::Items => {
                        if items.is_some() {
                            return Err(Error::duplicate_field("value"));
                        }
                        items = Some(map.next_value_seed(ItemsDe(arena))?);
                    }
                    ListFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                }
            }
            let prolog = prolog.ok_or_else(|| Error::missing_field("prolog"))?;
            let cells = items.ok_or_else(|| Error::missing_field("items"))?;
            let epilog = epilog.ok_or_else(|| Error::missing_field("epilog"))?;
            Ok(List {
                prolog,
                cells,
                epilog,
            })
        }
    }
}

super::serialize_deserialize_seed_visit! {
    #[expecting="a verbose entry in a dictionary"]
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
    }
}

super::serialize_deserialize_seed_visit! {
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
                arena.entry(entry);
                count += 1;
            }
            Ok(arena.dict(count).unwrap().cells)
        }
    }
}

super::serialize_deserialize_seed_visit! {
    #[expecting = "a verbose Dict (prolog, entries, epilog)"]
    #[deserialize_struct]
    impl Dict {
        fn serialize() {
            let mut fields = s.serialize_struct("Dict", 3)?;
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            fields.serialize_field("entries", &EntriesSer(this.cells))?;
            fields.serialize_field("epilog", &CommentSer(this.epilog))?;
            fields.end()
        }
        fn visit_seq() {
            let prolog = seq
                .next_element_seed(CommentDe(arena))?
                .ok_or_else(|| Error::invalid_length(1, &self))?;
            let cells = seq
                .next_element_seed(EntriesDe(arena))?
                .ok_or_else(|| Error::invalid_length(0, &self))?;
            let epilog = seq
                .next_element_seed(CommentDe(arena))?
                .ok_or_else(|| Error::invalid_length(1, &self))?;
            Ok(Dict {
                prolog,
                cells,
                epilog,
            })
        }
        fn visit_map() {
            let mut prolog = None;
            let mut entries = None;
            let mut epilog = None;
            while let Some(key) = map.next_key()? {
                match key {
                    DictFields::Prolog => {
                        if prolog.is_some() {
                            return Err(Error::duplicate_field("prolog"));
                        }
                        prolog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                    DictFields::Entries => {
                        if entries.is_some() {
                            return Err(Error::duplicate_field("value"));
                        }
                        entries = Some(map.next_value_seed(EntriesDe(arena))?);
                    }
                    DictFields::Epilog => {
                        if epilog.is_some() {
                            return Err(Error::duplicate_field("epilog"));
                        }
                        epilog = Some(map.next_value_seed(CommentDe(arena))?);
                    }
                }
            }
            let prolog = prolog.ok_or_else(|| Error::missing_field("prolog"))?;
            let cells = entries.ok_or_else(|| Error::missing_field("items"))?;
            let epilog = epilog.ok_or_else(|| Error::missing_field("epilog"))?;
            Ok(Dict {
                prolog,
                cells,
                epilog,
            })
        }
    }
}

super::serialize_deserialize_seed_visit! {
    #[expecting="a verbose File (hashbang, prolog, entries"]
    #[deserialize_struct]
    impl File {
        fn serialize() {
            let mut fields = s.serialize_struct("File", 3)?;
            fields.serialize_field("hashbang", &CommentSer(this.hashbang))?;
            fields.serialize_field("prolog", &CommentSer(this.prolog))?;
            fields.serialize_field("entries", &EntriesSer(this.cells))?;
            fields.end()
        }
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
