use super::UTF8Ser;
use core::cell::Cell;
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};

struct Text<'a>(crate::Text<'a>);
impl<'a> Serialize for Text<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Text(this) = self;
        let mut fields = s.serialize_struct("Text", 2)?;
        fields.serialize_field("utf8", &UTF8Ser(this.utf8))?;
        fields.serialize_field("epilog", &UTF8Ser::opt(this.epilog))?;
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
        let mut fields = s.serialize_struct("List", 3)?;
        fields.serialize_field("prolog", &UTF8Ser::opt(this.prolog))?;
        fields.serialize_field("cells", &Items(this.cells))?;
        fields.serialize_field("epilog", &UTF8Ser::opt(this.epilog))?;
        fields.end()
    }
}

struct Entry<'a, 'store>(crate::Entry<'a, 'store>);
impl<'a, 'store> Serialize for Entry<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Entry(this) = self;
        let mut fields = s.serialize_struct("Entry", 4)?;
        fields.serialize_field("gap", &this.name.gap)?;
        fields.serialize_field("before", &UTF8Ser::opt(this.name.before))?;
        fields.serialize_field("key", this.name.key)?;
        fields.serialize_field("item", &Item(this.item))?;
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
        let mut fields = s.serialize_struct("Dict", 3)?;
        fields.serialize_field("prolog", &UTF8Ser::opt(this.prolog))?;
        fields.serialize_field("cells", &Entries(this.cells))?;
        fields.serialize_field("epilog", &UTF8Ser::opt(this.epilog))?;
        fields.end()
    }
}

/// serialize all fields, avoiding "skip_serializing_if"
pub struct Verbose<'a, 'store>(pub crate::File<'a, 'store>);
impl<'a, 'store> Serialize for Verbose<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Verbose(this) = self;
        let mut fields = s.serialize_struct("File", 3)?;
        fields.serialize_field("hashbang", &UTF8Ser::opt(this.hashbang))?;
        fields.serialize_field("prolog", &UTF8Ser::opt(this.prolog))?;
        fields.serialize_field("cells", &Entries(this.cells))?;
        fields.end()
    }
}
