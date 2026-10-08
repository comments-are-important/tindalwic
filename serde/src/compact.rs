use super::*;
use serde::de::{DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, Visitor};
use serde::de::{Error as _, VariantAccess as _};
use serde::ser::{Serialize, Serializer};
use serde::ser::{SerializeSeq as _, SerializeStruct as _};
use std::fmt;
use tindalwic::parse::{Build, Parse};

struct ItemSer<'a>(Item<'a>);
impl<'a> Serialize for ItemSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let ItemSer(this) = self;
        match this {
            Item::Text(text) => ItemVariants::Text.serialize(s, &TextSer(*text)),
            Item::List(list) => ItemVariants::List.serialize(s, &ListSer(*list)),
            Item::Dict(dict) => ItemVariants::Dict.serialize(s, &DictSer(*dict)),
        }
    }
}
struct ItemDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ItemDe<'a, 'b> {
    type Value = Item<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_enum(ItemVariants::KIND, ItemVariants::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ItemDe<'a, 'b> {
    type Value = Item<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = ItemVariants::KIND;
        let names = ItemVariants::NAMES.join(" | ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> StdResult<Self::Value, A::Error> {
        let ItemDe(build) = self;
        let (this, access) = data.variant::<ItemVariants>()?;
        Ok(match this {
            ItemVariants::Text => Item::Text(access.newtype_variant_seed(TextDe(build))?),
            ItemVariants::List => Item::List(access.newtype_variant_seed(ListDe(build))?),
            ItemVariants::Dict => Item::Dict(access.newtype_variant_seed(DictDe(build))?),
        })
    }
}

struct CommentSer<'a>(Comment<'a>);
impl<'a> Serialize for CommentSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let CommentSer(this) = self;
        let should = CommentFields::assign(|field| match field {
            CommentFields::Gap => this.gap != 0,
            CommentFields::Value => this.value.is_some(),
        });
        let mut fields = s.serialize_struct(CommentFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                CommentFields::Gap => fields.serialize_field(field.into(), &this.gap)?,
                CommentFields::Value => {
                    fields.serialize_field(field.into(), &MaybeSer(this.value))?
                }
            }
        }
        fields.end()
    }
}
struct CommentDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for CommentDe<'a, 'b> {
    type Value = Comment<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(CommentFields::KIND, CommentFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for CommentDe<'a, 'b> {
    type Value = Comment<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = CommentFields::KIND;
        let names = CommentFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let CommentDe(build) = self;
        let mut result = Comment::default();
        let mut seen = CommentFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                CommentFields::Gap => result.gap = map.next_value()?,
                CommentFields::Value => result.value = map.next_value_seed(MaybeDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct TextSer<'a>(Text<'a>);
impl<'a> Serialize for TextSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let TextSer(this) = self;
        let should = TextFields::assign(|field| match field {
            TextFields::Value => !this.value.is_empty(),
            TextFields::Epilog => this.epilog.is_some(),
        });
        let mut fields = s.serialize_struct(TextFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                TextFields::Value => fields.serialize_field(field.into(), &ValueSer(this.value))?,
                TextFields::Epilog => {
                    fields.serialize_field(field.into(), &MaybeSer(this.epilog))?
                }
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
                TextFields::Value => result.value = map.next_value_seed(ValueDe(build))?,
                TextFields::Epilog => result.epilog = map.next_value_seed(MaybeDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct ItemsSer<'a>(Items<'a>);
impl<'a> Serialize for ItemsSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let ItemsSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&ItemSer(cell.get()))?;
        }
        seq.end()
    }
}
struct ItemsDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for ItemsDe<'a, 'b> {
    type Value = Items<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for ItemsDe<'a, 'b> {
    type Value = Items<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = ItemVariants::KIND;
        write!(out, "sequence of {STYLE} {kind}")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> {
        let ItemsDe(build) = self;
        let mut count = 0usize;
        while let Some(item) = seq.next_element_seed(ItemDe(build))? {
            build.push_item(item).map_err(A::Error::custom)?;
            count += 1;
        }
        Ok(build.finish_items(count).map_err(A::Error::custom)?)
    }
}

struct ListSer<'a>(List<'a>);
impl<'a> Serialize for ListSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let ListSer(this) = self;
        let should = ListFields::assign(|field| match field {
            ListFields::Prolog => this.prolog.value.is_some(),
            ListFields::Items => !this.items.is_empty(),
            ListFields::Epilog => this.epilog.value.is_some(),
        });
        let mut fields = s.serialize_struct(ListFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                ListFields::Prolog => {
                    fields.serialize_field(field.into(), &CommentSer(this.prolog))?
                }
                ListFields::Items => fields.serialize_field(field.into(), &ItemsSer(this.items))?,
                ListFields::Epilog => {
                    fields.serialize_field(field.into(), &CommentSer(this.epilog))?
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
                ListFields::Prolog => result.prolog = map.next_value_seed(CommentDe(build))?,
                ListFields::Items => result.items = map.next_value_seed(ItemsDe(build))?,
                ListFields::Epilog => result.epilog = map.next_value_seed(CommentDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct NameSer<'a>(Name<'a>);
impl<'a> Serialize for NameSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let NameSer(this) = self;
        let should = NameFields::assign(|field| match field {
            NameFields::Comment => this.comment.value.is_some() || this.comment.gap != 0,
            NameFields::Key => !this.key.is_empty(),
        });
        let mut fields = s.serialize_struct(NameFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                NameFields::Comment => {
                    fields.serialize_field(field.into(), &CommentSer(this.comment))?
                }
                NameFields::Key => fields.serialize_field(field.into(), &ValueSer(this.key))?,
            }
        }
        fields.end()
    }
}
struct NameDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for NameDe<'a, 'b> {
    type Value = Name<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(NameFields::KIND, NameFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for NameDe<'a, 'b> {
    type Value = Name<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = NameFields::KIND;
        let names = NameFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let NameDe(build) = self;
        let mut result = Name::default();
        let mut seen = NameFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                NameFields::Comment => result.comment = map.next_value_seed(CommentDe(build))?,
                NameFields::Key => result.key = map.next_value_seed(ValueDe(build))?,
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct EntrySer<'a>(Entry<'a>);
impl<'a> Serialize for EntrySer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let EntrySer(this) = self;
        let should = EntryFields::assign(|field| match field {
            EntryFields::Name => !this.name.key.is_empty() || this.name.comment.value.is_some(),
            EntryFields::Item => match this.item {
                Item::Text(Text { value, epilog, .. }) => epilog.is_some() || !value.is_empty(),
                _ => true,
            },
        });
        let mut fields = s.serialize_struct(EntryFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                EntryFields::Name => fields.serialize_field(field.into(), &NameSer(this.name))?,
                EntryFields::Item => fields.serialize_field(field.into(), &ItemSer(this.item))?,
            }
        }
        fields.end()
    }
}
struct EntryDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for EntryDe<'a, 'b> {
    type Value = Entry<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(EntryFields::KIND, EntryFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for EntryDe<'a, 'b> {
    type Value = Entry<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = EntryFields::KIND;
        let names = EntryFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let EntryDe(build) = self;
        let mut result = Entry::default();
        let mut seen = EntryFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                EntryFields::Name => {
                    result.name = map.next_value_seed(NameDe(build))?;
                }
                EntryFields::Item => {
                    result.item = map.next_value_seed(ItemDe(build))?;
                }
            }
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> StdResult<Self::Value, A::Error> {
        Err(A::Error::custom("visitor wants seq of fields, use Verbose"))
    }
}

struct EntriesSer<'a>(Entries<'a>);
impl<'a> Serialize for EntriesSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let EntriesSer(this) = self;
        let mut seq = s.serialize_seq(Some(this.len()))?;
        for cell in this.iter() {
            seq.serialize_element(&EntrySer(cell.get()))?;
        }
        seq.end()
    }
}
struct EntriesDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for EntriesDe<'a, 'b> {
    type Value = Entries<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for EntriesDe<'a, 'b> {
    type Value = Entries<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = EntryFields::KIND;
        write!(out, "sequence of {STYLE} {kind}")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> StdResult<Self::Value, A::Error> {
        let EntriesDe(build) = self;
        let mut count = 0usize;
        while let Some(entry) = seq.next_element_seed(EntryDe(build))? {
            build.push_entry(entry).map_err(A::Error::custom)?;
            count += 1;
        }
        Ok(build.finish_entries(count).map_err(A::Error::custom)?)
    }
}

struct DictSer<'a>(Dict<'a>);
impl<'a> Serialize for DictSer<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> StdResult<S::Ok, S::Error> {
        let DictSer(dict) = self;
        let should = DictFields::assign(|field| match field {
            DictFields::Prolog => dict.prolog.value.is_some(),
            DictFields::Entries => !dict.entries.is_empty(),
            DictFields::Epilog => dict.epilog.value.is_some(),
        });
        let mut fields = s.serialize_struct(DictFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                DictFields::Prolog => {
                    fields.serialize_field(field.into(), &MaybeSer(dict.prolog.value))?
                }
                DictFields::Entries => {
                    fields.serialize_field(field.into(), &EntriesSer(dict.entries))?
                }
                DictFields::Epilog => {
                    fields.serialize_field(field.into(), &MaybeSer(dict.epilog.value))?
                }
            }
        }
        fields.end()
    }
}
struct DictDe<'a, 'b>(&'b mut dyn Build<'a>);
impl<'de, 'a, 'b> DeserializeSeed<'de> for DictDe<'a, 'b> {
    type Value = Dict<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> StdResult<Self::Value, D::Error> {
        d.deserialize_struct(DictFields::KIND, DictFields::NAMES, self)
    }
}
impl<'de, 'a, 'b> Visitor<'de> for DictDe<'a, 'b> {
    type Value = Dict<'a>;
    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let kind = DictFields::KIND;
        let names = DictFields::NAMES.join(", ");
        write!(out, "{STYLE} {kind}: {names}")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> StdResult<Self::Value, A::Error> {
        let DictDe(build) = self;
        let mut result = Dict::default();
        let mut seen = DictFields::each(false);
        while let Some(field) = map.next_key()? {
            match seen.once::<A, _>(field, |it| A::Error::duplicate_field(it))? {
                DictFields::Prolog => result.prolog = map.next_value_seed(CommentDe(build))?,
                DictFields::Entries => result.entries = map.next_value_seed(EntriesDe(build))?,
                DictFields::Epilog => result.epilog = map.next_value_seed(CommentDe(build))?,
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
        let should = FileFields::assign(|field| match field {
            FileFields::Hashbang => this.hashbang.is_some(),
            FileFields::Prolog => this.prolog.value.is_some(),
            FileFields::Entries => !this.entries.is_empty(),
        });
        let mut fields = s.serialize_struct(FileFields::KIND, should.count())?;
        for field in should.selected() {
            match field {
                FileFields::Hashbang => {
                    fields.serialize_field(field.into(), &MaybeSer(this.hashbang))?
                }
                FileFields::Prolog => {
                    fields.serialize_field(field.into(), &MaybeSer(this.prolog.value))?
                }
                FileFields::Entries => {
                    fields.serialize_field(field.into(), &EntriesSer(this.entries))?
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
                FileFields::Hashbang => result.hashbang = map.next_value_seed(MaybeDe(build))?,
                FileFields::Prolog => result.prolog = map.next_value_seed(CommentDe(build))?,
                FileFields::Entries => result.entries = map.next_value_seed(EntriesDe(build))?,
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
