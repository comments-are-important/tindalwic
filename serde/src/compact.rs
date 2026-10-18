use super::*;
use serde::de::{DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, Visitor};
use serde::de::{Error as _, VariantAccess as _};
use serde::ser::{Serialize, Serializer};
use serde::ser::{SerializeSeq as _, SerializeStruct as _};
use std::fmt;
use tindalwic::group::{Groups, Members};
use tindalwic::parse::{Build, Parse};
use tindalwic::{Data, File, Group, List, Member, Text};

fn text_should<'a>(text: &Text<'a>) -> Flags<TextFields> {
    TextFields::assign(|field| match field {
        TextFields::Lines => !text.lines.is_empty(),
        TextFields::Epilog => text.epilog.is_some(),
        TextFields::Verbose => text.verbose,
    })
}
fn list_should<'a>(list: &List<'a>) -> Flags<ListFields> {
    ListFields::assign(|field| match field {
        ListFields::Prolog => list.prolog.is_some(),
        ListFields::Groups => !list.groups.is_empty(),
        ListFields::Epilog => list.epilog.is_some(),
    })
}
fn member_should<'a>(member: &Member<'a>) -> Flags<MemberFields> {
    MemberFields::assign(|field| match field {
        MemberFields::Comment => member.comment.is_some(),
        MemberFields::Name => !member.name.is_empty() || member.comment.is_some(),
        MemberFields::Data => match member.data {
            Data::Text(text) => text.epilog.is_some() || !text.lines.is_empty(),
            _ => true,
        },
        MemberFields::Longer => member.longer,
    })
}
fn group_should<'a>(group: &Group<'a>) -> Flags<GroupFields> {
    GroupFields::assign(|field| match field {
        GroupFields::Heading => group.heading.is_some(),
        GroupFields::Members => !group.members.is_empty(),
    })
}
fn file_should<'a>(file: &File<'a>) -> Flags<FileFields> {
    FileFields::assign(|field| match field {
        FileFields::Shebang => file.shebang.is_some(),
        FileFields::Prolog => file.prolog.is_some(),
        FileFields::Groups => !file.groups.is_empty(),
    })
}

struct DataSer<'a>(Data<'a>);
impl<'a> Serialize for DataSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let DataSer(this) = self;
        match this {
            Data::Text(text) => DataVariants::Text.serialize(s, &TextSer(*text)),
            Data::List(list) => DataVariants::List.serialize(s, &ListSer(*list)),
        }
    }
}
struct DataDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for DataDe<'a, 'b> {
    type Value = Data<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_enum(DataVariants::KIND, DataVariants::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for DataDe<'a, 'b> {
    type Value = Data<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = DataVariants::KIND;
        let names = DataVariants::NAMES.join(" | ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> StdResult<Self::Value, A::Error> {
        let DataDe(build) = self;
        let (this, access) = data.variant::<DataVariants>()?;
        Ok(match this {
            DataVariants::Text => Data::Text(access.newtype_variant_seed(TextDe(build))?),
            DataVariants::List => Data::List(access.newtype_variant_seed(ListDe(build))?),
        })
    }
}

struct TextSer<'a>(Text<'a>);
impl<'a> Serialize for TextSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let TextSer(this) = self;
        let should = text_should(this);
        let mut fields = s.serialize_struct(TextFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                TextFields::Lines => fields.serialize_field(field.into(), &ValueSer(this.lines))?,
                TextFields::Epilog => {
                    fields.serialize_field(field.into(), &MaybeSer(this.epilog))?
                }
                TextFields::Verbose => fields.serialize_field(field.into(), &this.verbose)?,
            }
        }
        fields.end()
    }
}
struct TextDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for TextDe<'a, 'b> {
    type Value = Text<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(TextFields::KIND, TextFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for TextDe<'a, 'b> {
    type Value = Text<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = TextFields::KIND;
        let names = TextFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let TextDe(build) = self;
        let mut result = Text::default();
        let mut seen = TextFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                TextFields::Lines => result.lines = map.next_value_seed(ValueDe(build))?,
                TextFields::Epilog => result.epilog = map.next_value_seed(MaybeDe(build))?,
                TextFields::Verbose => result.verbose = map.next_value()?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct GroupsSer<'a>(Groups<'a>);
impl<'a> Serialize for GroupsSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let GroupsSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for item in this {
            seq.serialize_element(&GroupSer(item))?;
        }
        seq.end()
    }
}
struct GroupsDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for GroupsDe<'a, 'b> {
    type Value = Groups<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for GroupsDe<'a, 'b> {
    type Value = Groups<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = GroupFields::KIND;
        write!(out, "sequence of {STYLE} {kind}")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> {
        let GroupsDe(build) = self;
        let mut count = 0usize;
        while let Some(item) = seq.next_element_seed(GroupDe(build))? {
            build.push_group(item).map_err(A::Error::custom)?;
            count += 1;
        }
        Ok(build.finish_groups(count).map_err(A::Error::custom)?)
    }
}

struct ListSer<'a>(List<'a>);
impl<'a> Serialize for ListSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let ListSer(this) = self;
        let should = list_should(this);
        let mut fields = s.serialize_struct(ListFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                ListFields::Prolog => {
                    fields.serialize_field(field.into(), &MaybeSer(this.prolog))?
                }
                ListFields::Groups => {
                    fields.serialize_field(field.into(), &GroupsSer(this.groups))?
                }
                ListFields::Epilog => {
                    fields.serialize_field(field.into(), &MaybeSer(this.epilog))?
                }
            }
        }
        fields.end()
    }
}
struct ListDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ListDe<'a, 'b> {
    type Value = List<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(ListFields::KIND, ListFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ListDe<'a, 'b> {
    type Value = List<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = ListFields::KIND;
        let names = ListFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let ListDe(build) = self;
        let mut result = List::default();
        let mut seen = ListFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                ListFields::Prolog => result.prolog = map.next_value_seed(MaybeDe(build))?,
                ListFields::Groups => result.groups = map.next_value_seed(GroupsDe(build))?,
                ListFields::Epilog => result.epilog = map.next_value_seed(MaybeDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct MemberSer<'a>(Member<'a>);
impl<'a> Serialize for MemberSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let MemberSer(this) = self;
        let should = member_should(this);
        let mut fields = s.serialize_struct(MemberFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                MemberFields::Comment => {
                    fields.serialize_field(field.into(), &MaybeSer(this.comment))?
                }
                MemberFields::Name => fields.serialize_field(field.into(), &ValueSer(this.name))?,
                MemberFields::Data => fields.serialize_field(field.into(), &DataSer(this.data))?,
                MemberFields::Longer => fields.serialize_field(field.into(), &this.longer)?,
            }
        }
        fields.end()
    }
}
struct MemberDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for MemberDe<'a, 'b> {
    type Value = Member<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(MemberFields::KIND, MemberFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for MemberDe<'a, 'b> {
    type Value = Member<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = MemberFields::KIND;
        let names = MemberFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let MemberDe(build) = self;
        let mut result = Member::default();
        let mut seen = MemberFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                MemberFields::Comment => {
                    result.comment = map.next_value_seed(MaybeDe(build))?;
                }
                MemberFields::Name => {
                    result.name = map.next_value_seed(ValueDe(build))?;
                }
                MemberFields::Data => {
                    result.data = map.next_value_seed(DataDe(build))?;
                }
                MemberFields::Longer => {
                    result.longer = map.next_value()?;
                }
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct MembersSer<'a>(Members<'a>);
impl<'a> Serialize for MembersSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let MembersSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for entry in this {
            seq.serialize_element(&MemberSer(entry))?;
        }
        seq.end()
    }
}
struct MembersDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for MembersDe<'a, 'b> {
    type Value = Members<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for MembersDe<'a, 'b> {
    type Value = Members<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = MemberFields::KIND;
        write!(out, "sequence of {STYLE} {kind}")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> {
        let MembersDe(build) = self;
        let mut count = 0usize;
        while let Some(entry) = seq.next_element_seed(MemberDe(build))? {
            build.push_member(entry).map_err(A::Error::custom)?;
            count += 1;
        }
        Ok(build.finish_members(count).map_err(A::Error::custom)?)
    }
}

struct GroupSer<'a>(Group<'a>);
impl<'a> Serialize for GroupSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let GroupSer(this) = self;
        let should = group_should(this);
        let mut fields = s.serialize_struct(GroupFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                GroupFields::Heading => {
                    fields.serialize_field(field.into(), &MaybeSer(this.heading))?
                }
                GroupFields::Members => {
                    fields.serialize_field(field.into(), &MembersSer(this.members))?
                }
            }
        }
        fields.end()
    }
}
struct GroupDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for GroupDe<'a, 'b> {
    type Value = Group<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(GroupFields::KIND, GroupFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for GroupDe<'a, 'b> {
    type Value = Group<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = GroupFields::KIND;
        let names = GroupFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let GroupDe(build) = self;
        let mut result = Group::default();
        let mut seen = GroupFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                GroupFields::Heading => result.heading = map.next_value_seed(MaybeDe(build))?,
                GroupFields::Members => result.members = map.next_value_seed(MembersDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct FileSer<'a>(File<'a>);
impl<'a> Serialize for FileSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let FileSer(this) = self;
        let should = file_should(this);
        let mut fields = s.serialize_struct(FileFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                FileFields::Shebang => {
                    fields.serialize_field(field.into(), &MaybeSer(this.shebang))?
                }
                FileFields::Prolog => {
                    fields.serialize_field(field.into(), &MaybeSer(this.prolog))?
                }
                FileFields::Groups => {
                    fields.serialize_field(field.into(), &GroupsSer(this.groups))?
                }
            }
        }
        fields.end()
    }
}
struct FileDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(FileFields::KIND, FileFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for FileDe<'a, 'b> {
    type Value = File<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = FileFields::KIND;
        let names = FileFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let FileDe(build) = self;
        let mut result = File::default();
        let mut seen = FileFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                FileFields::Shebang => result.shebang = map.next_value_seed(MaybeDe(build))?,
                FileFields::Prolog => result.prolog = map.next_value_seed(MaybeDe(build))?,
                FileFields::Groups => result.groups = map.next_value_seed(GroupsDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
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

const STYLE: &'static str = stringify!(Compact);
