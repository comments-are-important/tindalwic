use super::*;
use serde::de::{DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, Visitor};
use serde::de::{Error as _, VariantAccess as _};
use serde::ser::{Serialize, Serializer};
use serde::ser::{SerializeSeq as _, SerializeStruct as _};
use std::fmt;
use tindalwic::group::{Groups, Members};
use tindalwic::parse::{Build, Parse};
use tindalwic::{Data, File, Group, List, Member, Text};

tindalwic_macros::template! { ? enum ^
    struct Entity@Ser<'a>($Entity<'a>);
    impl <'a> Serialize for Entity@Ser<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
            let Entity@Ser(this) = self;
            match this { { ^
                $Entity::$Name($link) => Entity@Variants::$Name.serialize(s, &Link@Ser(*$link)),
            } }
        }
    }
    struct Entity@De<'a, 'b>(&'b mut dyn Build<'a>);
    impl<'de, 'a, 'b> DeserializeSeed<'de> for Entity@De<'a, 'b> {
        type Value = $Entity<'a>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
            d.deserialize_enum(~Entity, Entity@Variants::NAMES, self)
        }
    }
    impl<'de, 'a, 'b> Visitor<'de> for Entity@De<'a, 'b> {
        type Value = $Entity<'a>;
        fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
            let names = Entity@Variants::NAMES.join(" | ");
            write!(out, "Verbose {}: {names}", ~Entity)
        }
        fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> StdResult<Self::Value, A::Error> {
            let Entity@De(build) = self;
            let (this, access) = data.variant::<Entity@Variants>()?;
            Ok(match this { { ^
                Entity@Variants::$Name => $Entity::$Name(access.newtype_variant_seed(Link@De(build))?),
            } })
        }
    }
}

tindalwic_macros::template! { ? struct ^
    struct Entity@Ser<'a>($Entity<'a>);
    impl<'a> Serialize for Entity@Ser<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
            let Entity@Ser(this) = self;
            let mut should = [false; Entity@Fields::NAMES.len()];
            { ^
                if ( ? bool ^ this.$name )
                   ( ? Comment ^ this.$name.is_some() )
                   ( ? Lines ^ !this.$name.is_empty() )
                   ( ? array ^ !this.$name.is_empty() )
                   ( ? else ^ true )
                   { should[ Entity@Fields::$Name as usize ] = true; }
            }
            let mut fields = s.serialize_struct(~Entity, should.iter().map(|&it| it as usize).sum())?;
            { ^
                if should[ Entity@Fields::$Name as usize ] {
                fields.serialize_field(~name, &
                ( ? bool    ^ this.$name )
                ( ? else    ^ Link@Ser(this.$name) )
            )?; } }
            fields.end()
        }
    }
    struct Entity@De<'a, 'b>(&'b mut dyn Build<'a>);
    impl<'de, 'a, 'b> DeserializeSeed<'de> for Entity@De<'a, 'b> {
        type Value = $Entity<'a>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
            d.deserialize_struct(~Entity, Entity@Fields::NAMES, self)
        }
    }
    impl<'de, 'a, 'b> Visitor<'de> for Entity@De<'a, 'b> {
        type Value = $Entity<'a>;
        fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
            let names = Entity@Fields::NAMES.join(", ");
            write!(out, "Verbose {}: {names}", ~Entity)
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
            let Entity@De(build) = self;
            let mut result = $Entity::default();
            let mut seen = [false;Entity@Fields::NAMES.len()];
            while let Some(field) = map.next_key()? {
                if seen[field as usize] {
                    return Err(A::Error::duplicate_field(Entity@Fields::NAMES[field as usize]));
                }
                seen[field as usize] = true;
                match field { { ^
                    ( ? bool ^ Entity@Fields::$Name => result.$name = map.next_value()?, )
                    ( ? else ^ Entity@Fields::$Name => result.$name = map.next_value_seed(Link@De(build))?, )
                } }
            }
            Ok(result)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
            Err(A::Error::custom("visitor wants seq of fields, use Verbose for that"))
        }
    }
}

tindalwic_macros::template! { ? array ^
    struct Entity@Ser<'a>($Entity<'a>);
    impl<'a> Serialize for Entity@Ser<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> { %
            let Entity@Ser(this) = self;
            let mut seq = s.serialize_seq(Some(this.len()))?;
            for item in this {
                seq.serialize_element(&Link@Ser(item))?;
            }
            seq.end()
        }
    }
    struct Entity@De<'a, 'b>(&'b mut dyn Build<'a>);
    impl<'de, 'a, 'b> DeserializeSeed<'de> for Entity@De<'a, 'b> {
        type Value = $Entity<'a>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
            d.deserialize_seq(self)
        }
    }
    impl<'de, 'a, 'b> Visitor<'de> for Entity@De<'a, 'b> {
        type Value = $Entity<'a>;
        fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
            let kind = ~Entity;
            write!(out, "sequence of Verbose {kind}")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> { %
            let Entity@De(build) = self;
            let mut count = 0usize;
            while let Some(item) = seq.next_element_seed(Link@De(build))? {
                build.push_@link(item).map_err(A::Error::custom)?;
                count += 1;
            }
            Ok(build.finish_@entity(count).map_err(A::Error::custom)?)
        }
    }
}

/// serialize only used fields, ala "skip_serializing_if"
pub struct Compact<'a>(pub File<'a>);
impl<'a> Serialize for Compact<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let Compact(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Compact<'a> {
    /// call thusly: `Compact::seed(&build).deserialize(...)`
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
