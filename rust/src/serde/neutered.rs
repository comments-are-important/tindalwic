use super::{ArenaSeed, UTF8De, UTF8Ser, seeded};
use crate::alloc::Arena;
use crate::internals::Builder;
use crate::{Dict, Entry, File, Item, List, Text};
use serde::de::{DeserializeSeed, Error};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

seeded! {
    #[expecting = "a neutered item (string value, list, or dictionary)"]
    #[deserialize_any]
    impl Item {
        fn serialize() {
            match this {
                Item::Text(text) => UTF8Ser(text.utf8).serialize(s),
                Item::List(list) => ListSer(*list).serialize(s),
                Item::Dict(dict) => DictSer(*dict).serialize(s),
            }
        }
        fn visit_str() {
            let utf8 = UTF8De(arena).visit_str(v)?;
            Ok(Item::Text(Text { utf8, epilog: None }))
        }
        fn visit_seq() {
            let list = ListDe(arena).visit_seq(seq)?;
            Ok(Item::List(list))
        }
        fn visit_map() {
            let dict = DictDe(arena).visit_map(map)?;
            Ok(Item::Dict(dict))
        }
    }
} // !seeded

seeded! {
    #[expecting = "a list of neutered items"]
    #[deserialize_seq]
    impl List {
        fn serialize() {
            let mut seq = s.serialize_seq(Some(this.cells.len()))?;
            for cell in this.cells.iter() {
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
            let list = arena.list(count).ok_or(Error::custom("out of memory"))?;
            Ok(list)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a dictionary (string keys, neutered item values)"]
    #[deserialize_map]
    impl Dict {
        fn serialize() {
            let mut map = s.serialize_map(Some(this.cells.len()))?;
            for cell in this.cells.iter() {
                let Entry { name, item } = cell.get();
                map.serialize_entry(name.key, &ItemSer(item))?;
            }
            map.end()
        }
        fn visit_map() {
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
} // !seeded

seeded! {
    #[expecting = "a file (string keys, neutered item values)"]
    #[deserialize_map]
    impl File {
        fn serialize() {
            DictSer(Dict::wrap(this.cells)).serialize(s)
        }
        fn visit_map() {
            let dict = DictDe(arena).visit_map(map)?;
            Ok(File::wrap(dict.cells))
        }
    }
} // !seeded

/// serialize to a format that can't remember comments.
pub struct Neutered<'a>(pub File<'a>);
impl<'a> Serialize for Neutered<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(file) = self;
        DictSer(Dict::wrap(file.cells)).serialize(s)
    }
}
impl<'de, 'a: 'de> ArenaSeed<'de, 'a> for Neutered<'a> {
    fn seed(arena: &'de Arena<'a>) -> impl DeserializeSeed<'de, Value = File<'a>> {
        FileDe(arena)
    }
}
