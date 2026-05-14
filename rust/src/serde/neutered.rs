use super::{UTF8De, UTF8Ser};
use crate::alloc::Arena;
use crate::internals::Builder;
use crate::{Dict, Entry, File, Item, List, Text};
use core::fmt;
use serde::de::{DeserializeSeed, Deserializer, Error};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

super::serialize_deserialize_seed_visit! {
    Item("an item (string, list, or dictionary)")
    serialize {
        match this {
            Item::Text(text) => UTF8Ser(text.utf8).serialize(s),
            Item::List(list) => ListSer(*list).serialize(s),
            Item::Dict(dict) => DictSer(*dict).serialize(s),
        }
    }
    deserialize_any
    visit_borrowed_str {
        let utf8 = UTF8De(arena).visit_borrowed_str(v)?;
        Ok(Item::Text(Text { utf8, epilog: None }))
    }
    visit_str {
        let utf8 = UTF8De(arena).visit_str(v)?;
        Ok(Item::Text(Text { utf8, epilog: None }))
    }
    visit_seq {
        let list = ListDe(arena).visit_seq(seq)?;
        Ok(Item::List(list))
    }
    visit_map {
        let dict = DictDe(arena).visit_map(map)?;
        Ok(Item::Dict(dict))
    }
}

super::serialize_deserialize_seed_visit! {
    List("a list of items")
    serialize {
        let mut seq = s.serialize_seq(Some(this.cells.len()))?;
        for cell in this.cells.iter() {
            seq.serialize_element(&ItemSer(cell.get()))?;
        }
        seq.end()
    }
    deserialize_seq
    visit_seq {
        let mut count = 0usize;
        while let Some(item) = seq.next_element_seed(ItemDe(arena))? {
            arena.item(item);
            count += 1;
        }
        let list = arena.list(count).ok_or(Error::custom("out of memory"))?;
        Ok(list)
    }
}

super::serialize_deserialize_seed_visit! {
    Dict("a dictionary of entries (string keys, item values")
    serialize {
        let mut map = s.serialize_map(Some(this.cells.len()))?;
        for cell in this.cells.iter() {
            let Entry { name, item } = cell.get();
            map.serialize_entry(name.key, &ItemSer(item))?;
        }
        map.end()
    }
    deserialize_map
    visit_map {
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

/// serialize to a format that can't remember comments.
pub struct Neutered<'a, 'store>(pub File<'a, 'store>);
impl<'a, 'store> Serialize for Neutered<'a, 'store> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(file) = self;
        DictSer(Dict::wrap(file.cells)).serialize(s)
    }
}
impl<'de: 'a + 'store, 'a, 'store> Neutered<'a, 'store> {
    /// deserialize from a format lacking comments
    pub fn deserialize<D: Deserializer<'de>>(
        arena: &'de Arena<'a, 'store>,
        d: D,
    ) -> Result<File<'a, 'store>, D::Error> {
        let dict = d.deserialize_map(DictDe(arena))?;
        Ok(File::wrap(dict.cells))
    }
}
