extern crate alloc;

use super::{ValueDe, ValueSer, seeded};
use crate::{Entry, File, Item};
use ::serde::ser::{SerializeMap as _, SerializeSeq as _};
use alloc::string::{String, ToString};
use serde::de::Error as _;

seeded! {
    #[expecting = "a neutered item (simple value, list, or dictionary)"]
    #[deserialize_any]
    impl Item {
        fn serialize() {
            match this {
                Item::Text { value, .. } => ValueSer(*value).serialize(s),
                Item::List { cells, .. } => ItemsSer(cells).serialize(s),
                Item::Dict { cells, .. } => EntriesSer(cells).serialize(s),
            }
        }
        fn visit_bool() {
            Ok(Item::text(if v { "true" } else { "false" }))
        }
        fn visit_i8() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_i16() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_i32() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_i64() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_i128() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_u8() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_u16() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_u32() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_u64() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_u128() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_f32() {
            let mut buffer = ryu::Buffer::new();
            Ok(Item::text(build.intern(buffer.format(v)).map_err(E::custom)?))
        }
        fn visit_f64() {
            let mut buffer = ryu::Buffer::new();
            Ok(Item::text(build.intern(buffer.format(v)).map_err(E::custom)?))
        }
        fn visit_char() {
            Ok(Item::text(build.intern(&v.to_string()).map_err(E::custom)?))
        }
        fn visit_str() {
            // defaults for `_borrowed_str` and `_string` arrive here
            let value = ValueDe::of(build).visit_str(v)?;
            Ok(Item::Text {
                value,
                epilog: None,
            })
        }
        fn visit_bytes() {
            // defaults for `_borrowed_bytes` and `_byte_buf` arrive here
            if v.is_ascii() {
                // SAFETY: Verified it is ASCII.
                let value = unsafe { str::from_utf8_unchecked(v) };
                Ok(Item::text(build.intern(value).map_err(E::custom)?))
            } else {
                let value: String = v.iter().map(|&b| char::from(b)).collect();
                Ok(Item::text(build.intern(&value).map_err(E::custom)?))
            }
        }
        fn visit_unit() {
            // nulls in (for example) JSON become empty strings in Tindalwic
            Ok(Item::default())
        }
        fn visit_seq() {
            Ok(Item::list(ItemsDe::of(build).visit_seq(seq)?))
        }
        fn visit_map() {
            Ok(Item::dict(EntriesDe::of(build).visit_map(map)?))
        }
        // defaults for non-simple `_none`, `_some`, `_enum`, `_newtype_struct`
        // all return Err - there is no clear choice for them, and they are uncommon
        // in the kinds of data for which Neutered makes sense, so that's fine.
    }
} // !seeded

seeded! {
    #[expecting = "a list of neutered items"]
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
            build.finish_items(count).map_err(A::Error::custom)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a dictionary (string keys, neutered item values)"]
    #[deserialize_map]
    impl Entries {
        fn serialize() {
            let mut map = s.serialize_map(Some(this.len()))?;
            for cell in this.iter() {
                let Entry { key, item, .. } = cell.get();
                let first = key.lines().next().unwrap_or(""); // TODO key.one_liner
                map.serialize_entry(first, &ItemSer(item))?;
            }
            map.end()
        }
        fn visit_map() {
            let mut count = 0usize;
            while let Some(key) = map.next_key_seed(ValueDe::of(build))? {
                let item = map.next_value_seed(ItemDe::of(build))?;
                let entry = Entry {
                    key: if let Some(slice) = key.verbatim(0) {
                        slice
                    } else {
                        build.intern(&key.joined()).map_err(A::Error::custom)?
                    }
                    .into(),
                    item,
                    ..Default::default()
                };
                build.push_entry(entry).map_err(A::Error::custom)?;
                count += 1;
            }
            build.finish_entries(count).map_err(A::Error::custom)
        }
    }
} // !seeded

seeded! {
    #[expecting = "a file (string keys, neutered item values)"]
    #[deserialize_map]
    impl File {
        fn serialize() {
            EntriesSer(this.cells).serialize(s)
        }
        fn visit_map() {
            let cells = EntriesDe::of(build).visit_map(map)?;
            Ok(File {
                hashbang: None,
                prolog: None,
                cells,
            })
        }
    }
} // !seeded

/// serialize to a format that can't remember comments.
pub struct Neutered<'a>(pub File<'a>);
impl<'a> ::serde::ser::Serialize for Neutered<'a> {
    fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Neutered<'a> {
    /// call thusly: `Neutered::seed(&mut parse).deserialize(...)`
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
