use super::{Error, Result};
use serde::de::{DeserializeSeed, Unexpected, Visitor};
use serde::de::{Deserializer as _, Error as _};
use tindalwic::parse::Parse;
use tindalwic::{Entry, Item, Value};

/// decode tindalwic data file into a type that is compatible with dictionary
pub fn from_tindalwic<'de, T: ::serde::Deserialize<'de>>(
    parse: &mut (dyn Parse<'de> + 'de),
    encoded: &'de str,
) -> Result<T> {
    let item = parse
        .first_error(encoded)
        .map_err(Error::custom)?
        // terrible - keep file around so comments can be serialized later...
        .embed_without_hashbang();
    let value = T::deserialize(ItemDe { encoded, item })?;
    Ok(value)
}

#[derive(Copy, Clone)]
pub struct ItemDe<'de, 'a> {
    encoded: &'de str,
    item: Item<'a>,
}
impl<'de, 'a> ItemDe<'de, 'a> {
    fn with_item(&self, item: Item<'a>) -> Self {
        ItemDe {
            encoded: self.encoded,
            item,
        }
    }
    fn with_text(&self, value: Value<'a>) -> Self {
        self.with_item(Item::Text {
            value,
            epilog: None,
        })
    }
    fn parse<T: std::str::FromStr>(&self) -> Option<T> {
        if let Item::Text { value, .. } = self.item {
            if let Some(verbatim) = value.verbatim(0) {
                if let Ok(value) = verbatim.trim().parse::<T>() {
                    return Some(value);
                }
            } else if let Ok(value) = value.joined().trim().parse::<T>() {
                return Some(value);
            }
        }
        None
    }
    fn outlive(&self, value: Value<'a>) -> Option<&'de str> {
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
impl<'de, 'a> serde::de::IntoDeserializer<'de, Error> for ItemDe<'de, 'a> {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
impl<'de, 'a> serde::Deserializer<'de> for ItemDe<'de, 'a> {
    type Error = Error;

    /// use Item kind (without conversion) to dispatch to visitor.
    /// all other methods attempt a conversion but fall back here to get
    /// the no conversion behavior or (more often) a good error.
    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        match self.item {
            Item::Text { value, .. } => {
                if let Some(verbatim) = self.outlive(value) {
                    v.visit_borrowed_str(verbatim)
                } else {
                    v.visit_string(value.joined())
                }
            }
            Item::List { cells, .. } => {
                let items = cells.iter().map(|cell| self.with_item(cell.get()));
                v.visit_seq(serde::de::value::SeqDeserializer::new(items))
            }
            Item::Dict { cells, .. } => {
                let entries = cells.iter().map(|cell| {
                    let Entry { key, item, .. } = cell.get();
                    (self.with_text(key), self.with_item(item))
                });
                v.visit_map(serde::de::value::MapDeserializer::new(entries))
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
        if let Item::Text { value, .. } = self.item {
            if let Some(verbatim) = value.verbatim(0) {
                if let Some(only) = only_char(verbatim) {
                    return v.visit_char(only);
                }
            } else {
                let joined = value.joined();
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
        if let Item::Text { value, .. } = self.item {
            if let Some(verbatim) = self.outlive(value).filter(|it| it.is_ascii()) {
                return v.visit_borrowed_bytes(verbatim.as_bytes());
            }
            use ::bytes::{BufMut, BytesMut};
            let value = value.joined();
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
        if let Item::List { cells, .. } = self.item {
            return match cells {
                [] => v.visit_none(),
                [value] => v.visit_some(self.with_item(value.get())),
                _ => Err(Error::new(
                    "can't make option from list with more than one item",
                )),
            };
        }
        self.deserialize_any(v)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value> {
        if let Item::List { cells, .. } = self.item {
            if cells.is_empty() {
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
        if let Item::Text { value, .. } = self.item {
            if let Some(verbatim) = value.verbatim(0) {
                if verbatim == name {
                    return v.visit_unit();
                }
            } else {
                let joined = value.joined();
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
        self.deserialize_any(v)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        self.deserialize_any(v)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        match self.item {
            Item::Text { value, .. } => v.visit_enum(EnumDe {
                de: &self,
                name: value,
                payload: None,
            }),
            Item::Dict { cells: [entry], .. } => {
                let Entry { key, item, .. } = entry.get();
                v.visit_enum(EnumDe {
                    de: &self,
                    name: key,
                    payload: Some(item),
                })
            }
            _ => self.deserialize_any(v),
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
    de: &'i ItemDe<'de, 'a>, // the container of this encoded enum
    name: Value<'a>,
    payload: Option<Item<'a>>,
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
    de: &'i ItemDe<'de, 'a>, // the container of this encoded enum
    payload: Option<Item<'a>>,
}
impl<'de, 'a, 'i> serde::de::VariantAccess<'de> for VariantDe<'de, 'a, 'i> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        if let Some(item) = self.payload {
            match item {
                Item::Text { value, .. } => {
                    if value.is_empty() {
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
                Item::List { .. } => Err(Error::invalid_type(Unexpected::Seq, &"unit variant")),
                Item::Dict { .. } => Err(Error::invalid_type(Unexpected::Map, &"unit variant")),
            }
        } else {
            Ok(())
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        if let Some(item) = self.payload {
            seed.deserialize(self.de.with_item(item))
        } else {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            ))
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value> {
        if let Some(item) = self.payload {
            self.de.with_item(item).deserialize_seq(v)
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
            self.de.with_item(item).deserialize_struct("", fields, v)
        } else {
            Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            ))
        }
    }
}
