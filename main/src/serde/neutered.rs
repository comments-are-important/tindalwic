extern crate alloc;

use super::{ValueDe, ValueSer, seeded};
use crate::{Dict, Entry, File, Item, List, Text, Value};
use alloc::format;
use alloc::string::{String, ToString};
use serde::de::Error;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

seeded! {
    #[expecting = "a neutered item (simple value, list, or dictionary)"]
    #[deserialize_any]
    impl Item {
        fn serialize() {
            match this {
                Item::Text(text) => ValueSer(text.value).serialize(s),
                Item::List(list) => ListSer(*list).serialize(s),
                Item::Dict(dict) => DictSer(*dict).serialize(s),
            }
        }
        fn visit_bool() {
            let value = if v { "true" } else { "false" };
            Ok(Item::text(value))
        }
        fn visit_i8() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_i16() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_i32() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_i64() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_i128() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_u8() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_u16() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_u32() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_u64() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_u128() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_f32() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_f64() {
            Ok(Item::text(arena.str(&format!("{:?}", v))))
        }
        fn visit_char() {
            Ok(Item::text(arena.str(&v.to_string())))
        }
        fn visit_str() {
            // defaults for `_borrowed_str` and `_string` arrive here
            let value = ValueDe::of(arena).visit_str(v)?;
            Ok(Item::Text(Text {
                value,
                epilog: None,
            }))
        }
        fn visit_bytes() {
            // defaults for `_borrowed_bytes` and `_byte_buf` arrive here
            if v.is_ascii() {
                // SAFETY: Verified it is ASCII.
                let value = unsafe { str::from_utf8_unchecked(v) };
                Ok(Item::text(arena.str(value)))
            } else {
                let value: String = v.iter().map(|&b| char::from(b)).collect();
                Ok(Item::text(arena.str(&value)))
            }
        }
        fn visit_unit() {
            // nulls in (for example) JSON become empty strings in Tindalwic
            Ok(Item::default())
        }
        fn visit_seq() {
            let list = ListDe::of(arena).visit_seq(seq)?;
            Ok(Item::List(list))
        }
        fn visit_map() {
            let dict = DictDe::of(arena).visit_map(map)?;
            Ok(Item::Dict(dict))
        }
        // defaults for non-simple `_none`, `_some`, `_enum`, `_newtype_struct`
        // all return Err - there is no clear choice for them, and they are uncommon
        // in the kinds of data for which Neutered makes sense, so that's fine.
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
            while let Some(item) = seq.next_element_seed(ItemDe::of(arena))? {
                arena
                    .item(item)
                    .map_err(|err| Error::custom(err.to_string()))?;
                count += 1;
            }
            arena
                .list(count)
                .map_err(|err| Error::custom(err.to_string()))
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
                let Entry { key, item, .. } = cell.get();
                let first = key.lines().next().unwrap_or(""); // TODO key.one_liner
                map.serialize_entry(first, &ItemSer(item))?;
            }
            map.end()
        }
        fn visit_map() {
            let mut count = 0usize;
            while let Some((key, item)) =
                map.next_entry_seed(ValueDe::of(arena), ItemDe::of(arena))?
            {
                let mut entry = Entry {
                    item,
                    ..Default::default()
                };
                if let Some(slice) = key.shortcut(0) {
                    entry.key = Value::wrap(slice);
                } else {
                    entry.key = Value::wrap(arena.str(&key.joined()));
                }
                arena
                    .entry(entry)
                    .map_err(|err| Error::custom(err.to_string()))?;
                count += 1;
            }
            arena
                .dict(count)
                .map_err(|err| Error::custom(err.to_string()))
        }
    }
} // !seeded

seeded! {
    #[expecting = "a file (string keys, neutered item values)"]
    #[deserialize_map]
    impl File {
        fn serialize() {
            DictSer(Dict {
                cells: this.cells,
                ..Default::default()
            })
            .serialize(s)
        }
        fn visit_map() {
            let dict = DictDe::of(arena).visit_map(map)?;
            Ok(File {
                cells: dict.cells,
                ..Default::default()
            })
        }
    }
} // !seeded

/// serialize to a format that can't remember comments.
pub struct Neutered<'a>(pub File<'a>);
impl<'a> Serialize for Neutered<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(this) = self;
        DictSer(Dict {
            cells: this.cells,
            ..Default::default()
        })
        .serialize(s)
    }
}
#[cfg(feature = "bumpalo")]
impl<'a> Neutered<'a> {
    /// call thusly: `Neutered::bumpalo_seed(&arena).deserialize(...)`
    pub fn bumpalo_seed<'de>(
        arena: &'de crate::bumpalo::Arena<'a>,
    ) -> impl serde::de::DeserializeSeed<'de, Value = File<'a>>
    where
        'a: 'de,
    {
        FileDe::of(&arena.builder)
    }
}
