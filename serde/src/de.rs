use super::{Error, Result};
use serde::de::{DeserializeSeed, Unexpected, Visitor};
use serde::de::{Deserializer as _, Error as _};
use tindalwic::Data;
use tindalwic::parse::Parse;
use tindalwic::text::Lines;

/// decode tindalwic file containing a single member (any name) into a value
pub fn from_tindalwic<'de, T: ::serde::Deserialize<'de>>(
    parse: &mut (dyn Parse<'de> + 'de),
    encoded: &'de str,
) -> Result<T> {
    let file = parse.first_error(encoded).map_err(Error::custom)?;
    let mut members = file.members();
    let Some(member) = members.next() else {
        return Err(Error::new("file has zero members"));
    };
    if members.next().is_some() {
        return Err(Error::new("file has multiple members"));
    }
    let value = T::deserialize(DataDe {
        encoded,
        data: member.data,
    })?;
    Ok(value)
}

/// [conventional](https://serde.rs/conventions.html) Deserializer type
#[derive(Clone, Copy, Debug)]
pub struct DataDe<'de, 'a> {
    encoded: &'de str,
    data: Data<'a>,
}
impl<'de, 'a> DataDe<'de, 'a> {
    fn with_data(&self, data: Data<'a>) -> Self {
        DataDe {
            encoded: self.encoded,
            data,
        }
    }
    fn with_text(&self, value: Lines<'a>) -> Self {
        self.with_data(Data::Text(value.into()))
    }
    fn parse<T: std::str::FromStr>(&self) -> Option<T> {
        if let Data::Text(text) = self.data {
            if let Some(verbatim) = text.lines.verbatim(0) {
                if let Ok(value) = verbatim.trim().parse::<T>() {
                    return Some(value);
                }
            } else if let Ok(value) = text.lines.joined().trim().parse::<T>() {
                return Some(value);
            }
        }
        None
    }
    fn outlive(&self, value: &Lines<'a>) -> Option<&'de str> {
        if let Some(verbatim) = value.verbatim(0) {
            let base = self.encoded.as_ptr() as usize;
            let mut start = verbatim.as_ptr() as usize;
            if base <= start && start < base + self.encoded.len() {
                start -= base;
                return Some(&self.encoded[start..start + verbatim.len()]);
            }
        }
        None
    }
}
impl<'de, 'a> serde::de::IntoDeserializer<'de, Error> for DataDe<'de, 'a> {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
impl<'de, 'a> serde::Deserializer<'de> for DataDe<'de, 'a> {
    type Error = Error;

    /// use Item kind (without conversion) to dispatch to visitor.
    /// all other methods attempt a conversion but fall back here to get
    /// the no conversion behavior or (more often) a good error.
    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        match &self.data {
            Data::Text(text) => {
                if let Some(verbatim) = self.outlive(&text.lines) {
                    v.visit_borrowed_str(verbatim)
                } else {
                    v.visit_string(text.lines.joined())
                }
            }
            Data::List(list) => {
                let items = list.members().map(|it| self.with_data(it.data));
                v.visit_seq(serde::de::value::SeqDeserializer::new(items))
            }
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<bool>() {
            return v.visit_bool(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_i8<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<i8>() {
            return v.visit_i8(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<i16>() {
            return v.visit_i16(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<i32>() {
            return v.visit_i32(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<i64>() {
            return v.visit_i64(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<i128>() {
            return v.visit_i128(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<u8>() {
            return v.visit_u8(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<u16>() {
            return v.visit_u16(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<u32>() {
            return v.visit_u32(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<u64>() {
            return v.visit_u64(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<u128>() {
            return v.visit_u128(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_f32<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<f32>() {
            return v.visit_f32(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Some(value) = self.parse::<f64>() {
            return v.visit_f64(value);
        }
        self.deserialize_any(v)
    }
    fn deserialize_char<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        fn only_char(of: &str) -> Option<char> {
            let mut iter = of.trim().chars();
            match (iter.next(), iter.next()) {
                (Some(first), None) => Some(first),
                _ => None,
            }
        }
        if let Data::Text(text) = &self.data {
            if let Some(verbatim) = text.lines.verbatim(0) {
                if let Some(only) = only_char(verbatim) {
                    return v.visit_char(only);
                }
            } else {
                let joined = text.lines.joined();
                if let Some(only) = only_char(&joined) {
                    return v.visit_char(only);
                }
            }
        }
        self.deserialize_any(v)
    }

    fn deserialize_str<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }
    fn deserialize_string<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Data::Text(text) = &self.data {
            if let Some(verbatim) = self.outlive(&text.lines).filter(|it| it.is_ascii()) {
                return v.visit_borrowed_bytes(verbatim.as_bytes());
            }
            use ::bytes::{BufMut, BytesMut};
            let value = text.lines.joined();
            let mut bytes = BytesMut::with_capacity(value.len());
            for ch in value.chars() {
                bytes.put_u8(
                    u8::try_from(ch)
                        .map_err(|_| Error::new("text has char too big to fit in u8"))?,
                );
            }
            return v.visit_byte_buf(bytes.to_vec());
        }
        self.deserialize_any(v)
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        self.deserialize_bytes(v)
    }

    fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Data::List(list) = &self.data {
            let mut members = list.members();
            let Some(first) = members.next() else {
                return v.visit_none();
            };
            return if members.next().is_some() {
                Err(Error::new(
                    "can't make option from list with more than one item",
                ))
            } else {
                v.visit_some(self.with_data(first.data))
            };
        }
        self.deserialize_any(v)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Data::List(list) = &self.data {
            if list.members().next().is_none() {
                return v.visit_unit();
            }
        }
        self.deserialize_any(v)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        v: V,
    ) -> Result<V::Value> {
        if let Data::Text(text) = &self.data {
            if let Some(verbatim) = text.lines.verbatim(0) {
                if verbatim == name {
                    return v.visit_unit();
                }
            } else {
                let joined = text.lines.joined();
                if joined == name {
                    return v.visit_unit();
                }
            }
        }
        self.deserialize_any(v)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        v: V,
    ) -> Result<V::Value> {
        v.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        v: V,
    ) -> Result<V::Value> {
        self.deserialize_any(v)
    }

    fn deserialize_map<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Data::List(list) = self.data {
            let items = list
                .members()
                .map(|it| (self.with_data(it.name.into()), self.with_data(it.data)));
            return v.visit_map(serde::de::value::MapDeserializer::new(items));
        }
        self.deserialize_any(v)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        self.deserialize_map(v)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        match &self.data {
            Data::Text(text) => v.visit_enum(EnumDe {
                de: &self,
                name: text.lines,
                payload: None,
            }),
            Data::List(list) => {
                if let Some(entry) = list.members().next() {
                    v.visit_enum(EnumDe {
                        de: &self,
                        name: entry.name,
                        payload: Some(entry.data),
                    })
                } else {
                    self.deserialize_map(v)
                }
            }
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }
}
struct EnumDe<'de, 'a, 'i> {
    de: &'i DataDe<'de, 'a>, // the container of this encoded enum
    name: Lines<'a>,
    payload: Option<Data<'a>>,
}
impl<'de, 'a, 'i> serde::de::EnumAccess<'de> for EnumDe<'de, 'a, 'i> {
    type Error = Error;
    type Variant = VariantDe<'de, 'a, 'i>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let EnumDe { de, name, payload } = self;
        Ok((
            seed.deserialize(self.de.with_text(name))?,
            VariantDe { de, payload },
        ))
    }
}
struct VariantDe<'de, 'a, 'i> {
    de: &'i DataDe<'de, 'a>, // the container of this encoded enum
    payload: Option<Data<'a>>,
}
impl<'de, 'a, 'i> serde::de::VariantAccess<'de> for VariantDe<'de, 'a, 'i> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        if let Some(item) = self.payload {
            match item {
                Data::Text(text) => {
                    if text.lines.is_empty() {
                        Ok(())
                    } else {
                        // could Unexpected::Str but that needs borrowed slice
                        // TODO try out this generic message and see if it is good enough
                        Err(Error::invalid_type(
                            Unexpected::Other("text"),
                            &"unit variant",
                        ))
                    }
                }
                _ => Err(Error::invalid_type(Unexpected::Seq, &"unit variant")),
            }
        } else {
            Ok(())
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        if let Some(item) = self.payload {
            seed.deserialize(self.de.with_data(item))
        } else {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            ))
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
        if let Some(item) = self.payload {
            self.de.with_data(item).deserialize_seq(v)
        } else {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            ))
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        if let Some(item) = self.payload {
            self.de.with_data(item).deserialize_struct("", fields, v)
        } else {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            ))
        }
    }
}
