use super::UTF8Ser;
use core::cell::Cell;
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

struct Text<'a>(crate::Text<'a>);
impl<'a> Serialize for Text<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Text(this) = self;
        let count = 1 + this.epilog.is_some() as usize;
        let mut fields = s.serialize_struct("Text", count)?;
        fields.serialize_field("utf8", &UTF8Ser(this.utf8))?;
        if let Some(epilog) = this.epilog {
            fields.serialize_field("epilog", &UTF8Ser(epilog.utf8))?;
        }
        fields.end()
    }
}

struct Item<'a, 'store>(crate::Item<'a, 'store>);
impl<'a, 'store> Serialize for Item<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Item(this) = self;
        match this {
            crate::Item::Text(text) => s.serialize_newtype_variant("Item", 0, "Text", &Text(*text)),
            crate::Item::List(list) => s.serialize_newtype_variant("Item", 1, "List", &List(*list)),
            crate::Item::Dict(dict) => s.serialize_newtype_variant("Item", 2, "Dict", &Dict(*dict)),
        }
    }
}

struct Items<'w, 'a: 'w, 's: 'w>(&'w [Cell<crate::Item<'a, 's>>]);
impl<'w, 'a: 'w, 's: 'w> Serialize for Items<'w, 'a, 's> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Items(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&Item(cell.get()))?;
        }
        seq.end()
    }
}

struct List<'a, 'store>(crate::List<'a, 'store>);
impl<'a, 'store> Serialize for List<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let List(this) = self;
        let count = 1 + this.prolog.is_some() as usize + this.epilog.is_some() as usize;
        let mut fields = s.serialize_struct("List", count)?;
        if let Some(prolog) = this.prolog {
            fields.serialize_field("prolog", &UTF8Ser(prolog.utf8))?;
        }
        fields.serialize_field("cells", &Items(this.cells))?;
        if let Some(epilog) = this.epilog {
            fields.serialize_field("epilog", &UTF8Ser(epilog.utf8))?;
        }
        fields.end()
    }
}

struct Entry<'a, 'store>(crate::Entry<'a, 'store>);
impl<'a, 'store> Serialize for Entry<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Entry(this) = self;
        let count = 2
            + this.name.gap as usize
            + this.name.before.is_some() as usize
            + match this.item {
                crate::Item::Text(text) => text.epilog.is_some() as usize,
                crate::Item::List(list) => {
                    list.prolog.is_some() as usize + list.epilog.is_some() as usize
                }
                crate::Item::Dict(dict) => {
                    dict.prolog.is_some() as usize + dict.epilog.is_some() as usize
                }
            };
        let mut fields = s.serialize_struct("Entry", count)?;
        if this.name.gap {
            fields.serialize_field("gap", &true)?;
        }
        if let Some(before) = this.name.before {
            fields.serialize_field("before", &UTF8Ser(before.utf8))?;
        }
        fields.serialize_field("key", this.name.key)?;
        if let Some(prolog) = match this.item {
            crate::Item::Text(_) => None,
            crate::Item::List(list) => list.prolog,
            crate::Item::Dict(dict) => dict.prolog,
        } {
            fields.serialize_field("prolog", &UTF8Ser(prolog.utf8))?;
        }
        fields.serialize_field("item", &Item(this.item))?;
        if let Some(epilog) = match this.item {
            crate::Item::Text(text) => text.epilog,
            crate::Item::List(list) => list.epilog,
            crate::Item::Dict(dict) => dict.epilog,
        } {
            fields.serialize_field("epilog", &UTF8Ser(epilog.utf8))?;
        }
        fields.end()
    }
}

struct Entries<'w, 'a: 'w, 's: 'w>(&'w [Cell<crate::Entry<'a, 's>>]);
impl<'w, 'a: 'w, 's: 'w> Serialize for Entries<'w, 'a, 's> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Entries(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&Entry::<'a, 's>(cell.get()))?;
        }
        seq.end()
    }
}

struct Dict<'a, 'store>(crate::Dict<'a, 'store>);
impl<'a, 'store> Serialize for Dict<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Dict(this) = self;
        let count = 1 + this.prolog.is_some() as usize + this.epilog.is_some() as usize;
        let mut fields = s.serialize_struct("Dict", count)?;
        if let Some(prolog) = this.prolog {
            fields.serialize_field("prolog", &UTF8Ser(prolog.utf8))?;
        }
        fields.serialize_field("cells", &Entries(this.cells))?;
        if let Some(epilog) = this.epilog {
            fields.serialize_field("epilog", &UTF8Ser(epilog.utf8))?;
        }
        fields.end()
    }
}

/// serialize only used fields, ala "skip_serializing_if"
pub struct Compact<'a, 'store>(pub crate::File<'a, 'store>);
impl<'a, 'store> Serialize for Compact<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Compact(this) = self;
        let count = 1 + this.hashbang.is_some() as usize + this.prolog.is_some() as usize;
        let mut fields = s.serialize_struct("File", count)?;
        if let Some(hashbang) = this.hashbang {
            fields.serialize_field("hashbang", &UTF8Ser(hashbang.utf8))?;
        }
        if let Some(prolog) = this.prolog {
            fields.serialize_field("prolog", &UTF8Ser(prolog.utf8))?;
        }
        fields.serialize_field("cells", &Entries(this.cells))?;
        fields.end()
    }
}
