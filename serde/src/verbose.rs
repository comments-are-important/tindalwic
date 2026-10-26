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
    struct Class~Ser<'a>($Class<'a>);
    impl <'a> Serialize for Class~Ser<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
            let Class~Ser(this) = self;
            match this { { ^
                $Class::$Name($link) => Class~Variants::$Name.serialize(s, &Link~Ser(*$link)),
            } }
        }
    }
    struct Class~De<'a, 'b>(&'b mut dyn Build<'a>);
    impl<'de, 'a, 'b> DeserializeSeed<'de> for Class~De<'a, 'b> {
        type Value = $Class<'a>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
            d.deserialize_enum(#Class, Class~Variants::NAMES, self)
        }
    }
    impl<'de, 'a, 'b> Visitor<'de> for Class~De<'a, 'b> {
        type Value = $Class<'a>;
        fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
            let names = Class~Variants::NAMES.join(" | ");
            write!(out, "Verbose {}: {names}", #Class)
        }
        fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> StdResult<Self::Value, A::Error> {
            let Class~De(build) = self;
            let (this, access) = data.variant::<Class~Variants>()?;
            Ok(match this { { ^
                Class~Variants::$Name => $Class::$Name(access.newtype_variant_seed(Link~De(build))?),
            } })
        }
    }
}

tindalwic_macros::template! { ? struct ^
    struct Class~Ser<'a>($Class<'a>);
    impl<'a> Serialize for Class~Ser<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
            let Class~Ser(this) = self;
            let mut fields = s.serialize_struct(#Class, Class~Fields::NAMES.len())?;
            { ^ fields.serialize_field(#name, &
                ( ? class ^ Link~Ser(this.$name) )
                ( ? else ^ this.$name )
            )?; }
            fields.end()
        }
    }
    struct Class~De<'a, 'b>(&'b mut dyn Build<'a>);
    impl<'de, 'a, 'b> DeserializeSeed<'de> for Class~De<'a, 'b> {
        type Value = $Class<'a>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
            d.deserialize_struct(#Class, Class~Fields::NAMES, self)
        }
    }
    impl<'de, 'a, 'b> Visitor<'de> for Class~De<'a, 'b> {
        type Value = $Class<'a>;
        fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
            let names = Class~Fields::NAMES.join(", ");
            write!(out, "Verbose {}: {names}", #Class)
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
            let Class~De(build) = self;
            let mut result = $Class::default();
            let mut seen = [false;Class~Fields::NAMES.len()];
            while let Some(field) = map.next_key()? {
                if seen[field as usize] {
                    return Err(A::Error::duplicate_field(Class~Fields::NAMES[field as usize]));
                }
                seen[field as usize] = true;
                match field { { ^
                    ( ? class ^ Class~Fields::$Name => result.$name = map.next_value_seed(Link~De(build))?, )
                    ( ? else ^ Class~Fields::$Name => result.$name = map.next_value()?, )
                } }
            }
            Ok(result)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> {
            let Class~De(build) = self;
            let mut result = $Class::default();
            { ^
                ( ? class ^ result.$name = seq.next_element_seed(Link~De(build))? )
                ( ? else ^ result.$name = seq.next_element()? )
                    .ok_or_else(|| A::Error::invalid_length(Class~Fields::$Name as usize, &Class~De(build)))?;
            }
            Ok(result)
        }
    }
}

tindalwic_macros::template! { ? array ^
    struct Class~Ser<'a>($Class<'a>);
    impl<'a> Serialize for Class~Ser<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> { %
            let Class~Ser(this) = self;
            let mut seq = s.serialize_seq(Some(this.len()))?;
            for item in this {
                seq.serialize_element(&Link~Ser(item))?;
            }
            seq.end()
        }
    }
    struct Class~De<'a, 'b>(&'b mut dyn Build<'a>);
    impl<'de, 'a, 'b> DeserializeSeed<'de> for Class~De<'a, 'b> {
        type Value = $Class<'a>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
            d.deserialize_seq(self)
        }
    }
    impl<'de, 'a, 'b> Visitor<'de> for Class~De<'a, 'b> {
        type Value = $Class<'a>;
        fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
            let kind = #Class;
            write!(out, "sequence of Verbose {kind}")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> { %
            let Class~De(build) = self;
            let mut count = 0usize;
            while let Some(item) = seq.next_element_seed(Link~De(build))? {
                build.push_~link(item).map_err(A::Error::custom)?;
                count += 1;
            }
            Ok(build.finish_~class(count).map_err(A::Error::custom)?)
        }
    }
}

/// serialize all fields, avoiding "skip_serializing_if"
pub struct Verbose<'a>(pub File<'a>);
impl<'a> Serialize for Verbose<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let Verbose(this) = self;
        FileSer(*this).serialize(s)
    }
}
impl<'a> Verbose<'a> {
    /// call thusly: `Verbose::seed(&build).deserialize(...)`
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
