use super::{Error, Result};
use serde::ser::Serialize;
use tindalwic::parse::Build;
use tindalwic::{Entries, Entry, File, Item, Items, Name, Value};

/// encode a type that is compatible with dictionary into a tindalwic data file.
pub fn to_tindalwic<'a, T: ?Sized + Serialize>(
    build: &mut dyn Build<'a>,
    value: &T,
) -> Result<String> {
    let item = {
        let mut ser = ItemSer { build };
        value.serialize(&mut ser)?
    };
    let file = File::try_from_dict_without_epilog(&item)
        .ok_or_else(|| Error::new("top-level value must serialize to a map or struct"))?;
    Ok(file.to_string())
}

/// build a tindalwic [Item] from any `T: Serialize`
pub struct ItemSer<'b, 'a> {
    build: &'b mut dyn Build<'a>,
}
impl<'c, 'b, 'a> ItemSer<'b, 'a> {
    // gather all the .map_err to one place
    fn push_item(&mut self, item: Item<'a>) -> Result<()> {
        self.build.push_item(item).map_err(Error::new)
    }
    fn finish_items(&mut self, count: usize) -> Result<Items<'a>> {
        self.build.finish_items(count).map_err(Error::new)
    }
    fn push_entry(&mut self, entry: Entry<'a>) -> Result<()> {
        self.build.push_entry(entry).map_err(Error::new)
    }
    fn finish_entries(&mut self, count: usize) -> Result<Entries<'a>> {
        self.build.finish_entries(count).map_err(Error::new)
    }
    fn intern(&mut self, v: &str) -> Result<&'a str> {
        self.build.intern(v).map_err(Error::new)
    }
}
impl<'c, 'b, 'a> serde::Serializer for &'c mut ItemSer<'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    type SerializeSeq = SeqSer<'c, 'b, 'a>;
    type SerializeTuple = SeqSer<'c, 'b, 'a>;
    type SerializeTupleStruct = SeqSer<'c, 'b, 'a>;
    type SerializeTupleVariant = TupleVariantSer<'c, 'b, 'a>;
    type SerializeMap = MapSer<'c, 'b, 'a>;
    type SerializeStruct = StructSer<'c, 'b, 'a>;
    type SerializeStructVariant = StructVariantSer<'c, 'b, 'a>;

    fn serialize_bool(self, v: bool) -> Result<Item<'a>> {
        self.serialize_str(if v { "true" } else { "false" })
    }
    fn serialize_i8(self, v: i8) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i128(self, v: i128) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u128(self, v: u128) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_f32(self, v: f32) -> Result<Item<'a>> {
        self.serialize_str(ryu::Buffer::new().format(v))
    }
    fn serialize_f64(self, v: f64) -> Result<Item<'a>> {
        self.serialize_str(ryu::Buffer::new().format(v))
    }
    fn serialize_char(self, v: char) -> Result<Item<'a>> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Item<'a>> {
        Ok(Item::text(self.intern(v)?.into()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Item<'a>> {
        if v.is_ascii() {
            // SAFETY: Verified it is ASCII.
            let value = unsafe { std::str::from_utf8_unchecked(v) };
            self.serialize_str(value)
        } else {
            let value: String = v.iter().map(|&b| char::from(b)).collect();
            self.serialize_str(&value)
        }
    }

    fn serialize_none(self) -> Result<Item<'a>> {
        Ok(Item::list(&[]))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Item<'a>> {
        let mut seq = SeqSer {
            ser: self,
            count: 0,
        };
        seq.push(value)?;
        seq.list()
    }

    fn serialize_unit(self) -> Result<Item<'a>> {
        self.serialize_none()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Item<'a>> {
        self.serialize_str(name)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
    ) -> Result<Item<'a>> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Item<'a>> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Item<'a>> {
        let item = value.serialize(&mut *self)?;
        let key = self.intern(variant)?.into();
        self.push_entry(Entry { name: key, item })?;
        let cells = self.finish_entries(1)?;
        Ok(Item::dict(cells))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSer {
            ser: self,
            count: 0,
        })
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SeqSer {
            ser: self,
            count: 0,
        })
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SeqSer {
            ser: self,
            count: 0,
        })
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        let variant = self.intern(variant)?;
        Ok(TupleVariantSer {
            ser: self,
            variant,
            count: 0,
        })
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSer {
            ser: self,
            key: None,
            count: 0,
        })
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(StructSer {
            ser: self,
            count: 0,
        })
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let variant = self.intern(variant)?;
        Ok(StructVariantSer {
            ser: self,
            variant,
            count: 0,
        })
    }
}

/// helper for ItemSer
pub struct SeqSer<'c, 'b, 'a> {
    ser: &'c mut ItemSer<'b, 'a>,
    count: usize,
}
impl<'c, 'b, 'a> SeqSer<'c, 'b, 'a> {
    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let item = value.serialize(&mut *self.ser)?;
        self.ser.push_item(item)?;
        self.count += 1;
        Ok(())
    }
    fn list(self) -> Result<Item<'a>> {
        Ok(Item::list(self.ser.finish_items(self.count)?))
    }
}
impl<'c, 'b, 'a> serde::ser::SerializeSeq for SeqSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Item<'a>> {
        self.list()
    }
}
impl<'c, 'b, 'a> serde::ser::SerializeTuple for SeqSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Item<'a>> {
        self.list()
    }
}
impl<'c, 'b, 'a> serde::ser::SerializeTupleStruct for SeqSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Item<'a>> {
        self.list()
    }
}

/// helper for ItemSer
pub struct TupleVariantSer<'c, 'b, 'a> {
    ser: &'c mut ItemSer<'b, 'a>,
    variant: &'a str,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeTupleVariant for TupleVariantSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let item = value.serialize(&mut *self.ser)?;
        self.ser.push_item(item)?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Item<'a>> {
        let list = Item::list(self.ser.finish_items(self.count)?);
        self.ser.push_entry(Entry {
            name: self.variant.into(),
            item: list,
        })?;
        let cells = self.ser.finish_entries(1)?;
        Ok(Item::dict(cells))
    }
}

/// helper for ItemSer
pub struct MapSer<'c, 'b, 'a> {
    ser: &'c mut ItemSer<'b, 'a>,
    key: Option<Value<'a>>,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeMap for MapSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        match key.serialize(&mut *self.ser)? {
            Item::Text { value, .. } => {
                self.key = Some(value);
                Ok(())
            }
            _ => Err(Error::new("map key must serialize to a string")),
        }
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let item = value.serialize(&mut *self.ser)?;
        let value = self
            .key
            .take()
            .ok_or_else(|| Error::new("value before key"))?;
        self.ser.push_entry(Entry {
            name: Name {
                key: value,
                ..Default::default()
            },
            item,
        })?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Item<'a>> {
        Ok(Item::dict(self.ser.finish_entries(self.count)?))
    }
}

/// helper for ItemSer
pub struct StructSer<'c, 'b, 'a> {
    ser: &'c mut ItemSer<'b, 'a>,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeStruct for StructSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let item = value.serialize(&mut *self.ser)?;
        let key = self.ser.intern(key)?;
        self.ser.push_entry(Entry {
            name: key.into(),
            item,
        })?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Item<'a>> {
        let cells = self.ser.finish_entries(self.count)?;
        Ok(Item::dict(cells))
    }
}

/// helper for ItemSer
pub struct StructVariantSer<'c, 'b, 'a> {
    ser: &'c mut ItemSer<'b, 'a>,
    variant: &'a str,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeStructVariant for StructVariantSer<'c, 'b, 'a> {
    type Ok = Item<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let item = value.serialize(&mut *self.ser)?;
        let key = self.ser.intern(key)?;
        self.ser.push_entry(Entry {
            name: key.into(),
            item,
        })?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Item<'a>> {
        let dict = Item::dict(self.ser.finish_entries(self.count)?);
        self.ser.push_entry(Entry {
            name: self.variant.into(),
            item: dict,
        })?;
        let cells = self.ser.finish_entries(1)?;
        Ok(Item::dict(cells))
    }
}
