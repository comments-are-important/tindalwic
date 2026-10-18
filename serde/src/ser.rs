use super::{Error, Result};
use serde::ser::Serialize;
use tindalwic::parse::Build;
use tindalwic::text::UTF8;
use tindalwic::{Data, File, List, Member, Text};

/// encode a value into a tindalwic file containing a single member with the given name.
pub fn to_tindalwic<'a, T: ?Sized + Serialize>(
    build: &mut dyn Build<'a>,
    name: &str,
    value: &T,
) -> Result<String> {
    let mut ser = DataSer { build };
    let mut member = Member::default();
    member.name = ser.intern(name)?.into();
    member.data = value.serialize(&mut ser)?;
    ser.push_member(member)?;
    let file = File::from_list_without_epilog(&ser.finish_list(1)?);
    Ok(file.to_string())
}

/// build a tindalwic [Data] from any `T: Serialize`
pub struct DataSer<'b, 'a> {
    build: &'b mut dyn Build<'a>,
}
impl<'c, 'b, 'a> DataSer<'b, 'a> {
    // gather all the .map_err to one place
    fn push_member(&mut self, member: Member<'a>) -> Result<()> {
        self.build.push_member(member).map_err(Error::new)
    }
    fn finish_list(&mut self, count: usize) -> Result<List<'a>> {
        self.build.finish_list(count).map_err(Error::new)
    }
    fn finish_data(&mut self, count: usize) -> Result<Data<'a>> {
        self.finish_list(count).map(|it| Data::List(it))
    }
    fn intern(&mut self, v: &str) -> Result<&'a str> {
        self.build.intern(v).map_err(Error::new)
    }
}
impl<'c, 'b, 'a> serde::Serializer for &'c mut DataSer<'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    type SerializeSeq = SeqSer<'c, 'b, 'a>;
    type SerializeTuple = SeqSer<'c, 'b, 'a>;
    type SerializeTupleStruct = SeqSer<'c, 'b, 'a>;
    type SerializeTupleVariant = TupleVariantSer<'c, 'b, 'a>;
    type SerializeMap = MapSer<'c, 'b, 'a>;
    type SerializeStruct = StructSer<'c, 'b, 'a>;
    type SerializeStructVariant = StructVariantSer<'c, 'b, 'a>;

    fn serialize_bool(self, v: bool) -> Result<Data<'a>> {
        self.serialize_str(if v { "true" } else { "false" })
    }
    fn serialize_i8(self, v: i8) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i128(self, v: i128) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u128(self, v: u128) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_f32(self, v: f32) -> Result<Data<'a>> {
        self.serialize_str(ryu::Buffer::new().format(v))
    }
    fn serialize_f64(self, v: f64) -> Result<Data<'a>> {
        self.serialize_str(ryu::Buffer::new().format(v))
    }
    fn serialize_char(self, v: char) -> Result<Data<'a>> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Data<'a>> {
        Ok(self.intern(v)?.into())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Data<'a>> {
        if v.is_ascii() {
            // SAFETY: Verified it is ASCII.
            let value = unsafe { std::str::from_utf8_unchecked(v) };
            self.serialize_str(value)
        } else {
            let value: String = v.iter().map(|&b| char::from(b)).collect();
            self.serialize_str(&value)
        }
    }

    fn serialize_none(self) -> Result<Data<'a>> {
        Ok(Data::List(List::default()))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Data<'a>> {
        let mut seq = SeqSer {
            ser: self,
            count: 0,
        };
        seq.push(value)?;
        seq.list()
    }

    fn serialize_unit(self) -> Result<Data<'a>> {
        self.serialize_none()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Data<'a>> {
        self.serialize_str(name)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
    ) -> Result<Data<'a>> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Data<'a>> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Data<'a>> {
        let mut member = Member::default();
        member.data = value.serialize(&mut *self)?;
        member.name = self.intern(variant)?.into();
        self.push_member(member)?;
        self.finish_data(1)
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
    ser: &'c mut DataSer<'b, 'a>,
    count: usize,
}
impl<'c, 'b, 'a> SeqSer<'c, 'b, 'a> {
    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let data = value.serialize(&mut *self.ser)?;
        self.ser.push_member(data.into())?;
        self.count += 1;
        Ok(())
    }
    fn list(self) -> Result<Data<'a>> {
        self.ser.finish_data(self.count)
    }
}
impl<'c, 'b, 'a> serde::ser::SerializeSeq for SeqSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Data<'a>> {
        self.list()
    }
}
impl<'c, 'b, 'a> serde::ser::SerializeTuple for SeqSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Data<'a>> {
        self.list()
    }
}
impl<'c, 'b, 'a> serde::ser::SerializeTupleStruct for SeqSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        self.push(v)
    }
    fn end(self) -> Result<Data<'a>> {
        self.list()
    }
}

/// helper for ItemSer
pub struct TupleVariantSer<'c, 'b, 'a> {
    ser: &'c mut DataSer<'b, 'a>,
    variant: &'a str,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeTupleVariant for TupleVariantSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let data = value.serialize(&mut *self.ser)?;
        self.ser.push_member(data.into())?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Data<'a>> {
        let mut member = Member::default();
        member.name = self.variant.into();
        member.data = self.ser.finish_data(self.count)?;
        self.ser.push_member(member)?;
        self.ser.finish_data(1)
    }
}

/// helper for ItemSer
pub struct MapSer<'c, 'b, 'a> {
    ser: &'c mut DataSer<'b, 'a>,
    key: Option<UTF8<'a>>,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeMap for MapSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        match key.serialize(&mut *self.ser)? {
            Data::Text(Text { lines, .. }) => {
                self.key = Some(lines);
                Ok(())
            }
            _ => Err(Error::new("map key must serialize to a string")),
        }
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let mut member = Member::default();
        member.data = value.serialize(&mut *self.ser)?;
        member.name = self
            .key
            .take()
            .ok_or_else(|| Error::new("value before key"))?;
        self.ser.push_member(member)?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Data<'a>> {
        Ok(self.ser.finish_data(self.count)?.into())
    }
}

/// helper for ItemSer
pub struct StructSer<'c, 'b, 'a> {
    ser: &'c mut DataSer<'b, 'a>,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeStruct for StructSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let mut member = Member::default();
        member.data = value.serialize(&mut *self.ser)?;
        member.name = self.ser.intern(key)?.into();
        self.ser.push_member(member)?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Data<'a>> {
        self.ser.finish_data(self.count)
    }
}

/// helper for ItemSer
pub struct StructVariantSer<'c, 'b, 'a> {
    ser: &'c mut DataSer<'b, 'a>,
    variant: &'a str,
    count: usize,
}
impl<'c, 'b, 'a> serde::ser::SerializeStructVariant for StructVariantSer<'c, 'b, 'a> {
    type Ok = Data<'a>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let mut member = Member::default();
        member.data = value.serialize(&mut *self.ser)?;
        member.name = self.ser.intern(key)?.into();
        self.ser.push_member(member)?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<Data<'a>> {
        let mut member = Member::default();
        member.data = self.ser.finish_data(self.count)?;
        member.name = self.variant.into();
        self.ser.push_member(member)?;
        self.ser.finish_data(1)
    }
}
