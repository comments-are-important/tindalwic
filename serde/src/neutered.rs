use super::{ValueDe, ValueSer};
use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, Serializer};
use serde::ser::{SerializeMap as _, SerializeSeq as _};
use std::collections::HashSet;
use std::fmt;
use tindalwic::parse::{Build, Parse};
use tindalwic::{Data, File, List, Member};

const NAME_KEY: &'static str = "_tindalwic_name_";
const DATA_KEY: &'static str = "_tindalwic_data_";

struct ListSer<'a>(List<'a>);
impl<'a> Serialize for ListSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let ListSer(this) = self;
        let mut dedup = HashSet::new();
        let unique = this
            .members()
            .all(|it| it.name.is_empty() || dedup.insert(it.name.joined()));
        if dedup.is_empty() {
            let mut seq = s.serialize_seq(Some(this.len()))?;
            for member in this.members() {
                seq.serialize_element(&DataSer(member.data))?;
            }
            seq.end()
        } else if unique {
            let mut map = s.serialize_map(Some(this.len()))?;
            for member in this.members() {
                map.serialize_entry(&ValueSer(member.name), &DataSer(member.data))?;
            }
            map.end()
        } else {
            let mut seq = s.serialize_seq(Some(this.len()))?;
            for member in this.members() {
                seq.serialize_element(&PairSer(member))?;
            }
            seq.end()
        }
    }
}
struct PairSer<'a>(Member<'a>);
impl<'a> Serialize for PairSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let PairSer(this) = self;
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry(NAME_KEY, &ValueSer(this.name))?;
        map.serialize_entry(DATA_KEY, &DataSer(this.data))?;
        map.end()
    }
}
struct DataSer<'a>(Data<'a>);
impl<'a> Serialize for DataSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let DataSer(this) = self;
        match this {
            Data::Text(text) => ValueSer(text.lines).serialize(s),
            Data::List(list) => ListSer(*list).serialize(s),
        }
    }
}

struct DataDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for DataDe<'a, 'b> {
    type Value = Data<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for DataDe<'a, 'b> {
    type Value = Data<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a neutered item (simple value, list, or dictionary)")
    }
    fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
        self.visit_str(if v { "true" } else { "false" })
    }
    fn visit_i8<E: Error>(self, v: i8) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_i16<E: Error>(self, v: i16) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_i32<E: Error>(self, v: i32) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_i128<E: Error>(self, v: i128) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_u16<E: Error>(self, v: u16) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_u32<E: Error>(self, v: u32) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_u128<E: Error>(self, v: u128) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_f32<E: Error>(self, v: f32) -> Result<Self::Value, E> {
        self.visit_str(ryu::Buffer::new().format(v))
    }
    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        self.visit_str(ryu::Buffer::new().format(v))
    }
    fn visit_char<E: Error>(self, v: char) -> Result<Self::Value, E> {
        self.visit_str(&v.to_string())
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let DataDe(build) = self;
        Ok(ValueDe(build).visit_str(v)?.into())
    }
    fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        if v.is_ascii() {
            let value = unsafe { std::str::from_utf8_unchecked(v) };
            self.visit_str(value)
        } else {
            let value: String = v.iter().map(|&b| char::from(b)).collect();
            self.visit_str(&value)
        }
    }
    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Data::default())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let DataDe(build) = self;
        Ok(Data::List(ListDe(build).visit_seq(seq)?))
    }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let DataDe(build) = self;
        Ok(Data::List(ListDe(build).visit_map(map)?))
    }
}

struct ListDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ListDe<'a, 'b> {
    type Value = List<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ListDe<'a, 'b> {
    type Value = List<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a file (string keys, neutered item values)")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let ListDe(build) = self;
        let mut count = 0usize;
        while let Some(name) = map.next_key_seed(ValueDe(build))? {
            let mut member = Member::default();
            member.data = map.next_value_seed(DataDe(build))?;
            member.name = (if let Some(slice) = name.verbatim(0) {
                slice
            } else {
                build.intern(&name.joined()).map_err(A::Error::custom)?
            })
            .into();
            build.push_member(member).map_err(A::Error::custom)?;
            count += 1;
        }
        let members = build.finish_members(count).map_err(A::Error::custom)?;
        build.push_group(members.into()).map_err(A::Error::custom)?;
        let groups = build.finish_groups(1).map_err(A::Error::custom)?;
        Ok(groups.into())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let ListDe(build) = self;
        let mut count = 0usize;
        while let Some(data) = seq.next_element_seed(DataDe(build))? {
            let mut member: Member = data.into();
            if let Data::List(list) = data {
                let mut members = list.members();
                if let Some(one) = members.next() {
                    if let Some(two) = members.next() {
                        if let Data::Text(name) = one.data {
                            if one.name.only_line() == Some(NAME_KEY)
                                && two.name.only_line() == Some(DATA_KEY)
                            {
                                member.name = name.lines;
                                member.data = two.data;
                            }
                        }
                    }
                }
            }
            build.push_member(member).map_err(A::Error::custom)?;
            count += 1;
        }
        let members = build.finish_members(count).map_err(A::Error::custom)?;
        build.push_group(members.into()).map_err(A::Error::custom)?;
        let groups = build.finish_groups(1).map_err(A::Error::custom)?;
        Ok(groups.into())
    }
}

struct FileDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a file (string keys, neutered item values)")
    }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let FileDe(build) = self;
        let list = ListDe(build).visit_map(map)?;
        Ok(File::from_list_without_epilog(&list))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let FileDe(build) = self;
        let list = ListDe(build).visit_seq(seq)?;
        Ok(File::from_list_without_epilog(&list))
    }
}

/// serialize to a format that can't remember comments.
pub struct Neutered<'a>(pub File<'a>);
impl<'a> Serialize for Neutered<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Neutered(this) = self;
        ListSer(this.embed_without_shebang()).serialize(s)
    }
}
impl<'a> Neutered<'a> {
    /// call thusly: `Neutered::seed(&mut parse).deserialize(...)`
    /// the deserialize will likely fail unless parse.builder() supports intern
    pub fn seed<'de, 'b, P: Parse<'a>>(
        parse: &'b mut P,
    ) -> impl DeserializeSeed<'de, Value = File<'a>> + 'b
    where
        'a: 'b,
    {
        FileDe(parse.builder())
    }
}
