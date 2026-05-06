//! adapters for serde

use core::cell::Cell;
//use core::fmt::{self, Formatter};
use serde;
// use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, SerializeStruct, Serializer as Ser};

struct UTF8<'a>(super::UTF8<'a>);
impl<'a> Serialize for UTF8<'a> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.0.dedent == 0 || self.0.dedent == usize::MAX {
            s.serialize_str(self.0.slice)
        } else {
            s.serialize_str(&self.0.joined())
        }
    }
}

// struct Comment<'a, const LOBOTOMIZE: bool>(super::Comment<'a>);
// impl<'a> Serialize for Comment<'a, false> {
//     fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
//         let mut state = Ser::serialize_struct(s, "Comment", 1)?;
//         SerializeStruct::serialize_field(&mut state, "utf8", &UTF8(self.0.utf8))?;
//         SerializeStruct::end(state)
//     }
// }
// impl<'de, 'a> Deserialize<'de> for Comment<'a> {
//     fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//     where
//         __D: Deserializer<'de>,
//     {
//         #[allow(non_camel_case_types)]
//         #[doc(hidden)]
//         enum __Field {
//             __field0,
//             __ignore,
//         }
//         #[doc(hidden)]
//         struct __FieldVisitor;
//         #[automatically_derived]
//         impl<'de> Visitor<'de> for __FieldVisitor {
//             type Value = __Field;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "field identifier")
//             }
//             fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     0u64 => Ok(__Field::__field0),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_str<__E>(self, __value: &str) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     "utf8" => Ok(__Field::__field0),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_bytes<__E>(self, __value: &[u8]) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     b"utf8" => Ok(__Field::__field0),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//         }
//         #[automatically_derived]
//         impl<'de> Deserialize<'de> for __Field {
//             #[inline]
//             fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//             where
//                 __D: Deserializer<'de>,
//             {
//                 Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//             }
//         }
//         #[doc(hidden)]
//         struct __Visitor<'de, 'a> {
//             marker: PhantomData<Comment<'a>>,
//             lifetime: PhantomData<&'de ()>,
//         }
//         #[automatically_derived]
//         impl<'de, 'a> Visitor<'de> for __Visitor<'de, 'a> {
//             type Value = Comment<'a>;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "struct Comment")
//             }
//             #[inline]
//             fn visit_seq<__A>(self, mut __seq: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: SeqAccess<'de>,
//             {
//                 let __field0 = match SeqAccess::next_element::<UTF8<'a>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             0usize,
//                             &"struct Comment with 1 element",
//                         ));
//                     }
//                 };
//                 Ok(Comment { utf8: __field0 })
//             }
//             #[inline]
//             fn visit_map<__A>(self, mut __map: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: MapAccess<'de>,
//             {
//                 let mut __field0: Option<UTF8<'a>> = None;
//                 while let Some(__key) = MapAccess::next_key::<__Field>(&mut __map)? {
//                     match __key {
//                         __Field::__field0 => {
//                             if Option::is_some(&__field0) {
//                                 return Err(<__A::Error as Error>::duplicate_field("utf8"));
//                             }
//                             __field0 = Some(MapAccess::next_value::<UTF8<'a>>(&mut __map)?);
//                         }
//                         _ => {
//                             let _ = MapAccess::next_value::<IgnoredAny>(&mut __map)?;
//                         }
//                     }
//                 }
//                 let __field0 = match __field0 {
//                     Some(__field0) => __field0,
//                     None => de::missing_field("utf8")?,
//                 };
//                 Ok(Comment { utf8: __field0 })
//             }
//         }
//         #[doc(hidden)]
//         const FIELDS: &'static [&'static str] = &["utf8"];
//         Deserializer::deserialize_struct(
//             __deserializer,
//             "Comment",
//             FIELDS,
//             __Visitor {
//                 marker: PhantomData::<Comment<'a>>,
//                 lifetime: PhantomData,
//             },
//         )
//     }
// }

struct Text<'a, const LOBOTOMIZE: bool>(super::Text<'a>);
impl<'a, const LOBOTOMIZE: bool> Serialize for Text<'a, LOBOTOMIZE> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        if LOBOTOMIZE {
            UTF8(self.0.utf8).serialize(s)
        } else {
            let count = 1 + self.0.epilog.is_some() as usize;
            let mut fields = s.serialize_struct("Text", count)?;
            fields.serialize_field("utf8", &UTF8(self.0.utf8))?;
            if let Some(epilog) = self.0.epilog {
                fields.serialize_field("epilog", &UTF8(epilog.utf8))?;
            }
            fields.end()
        }
    }
}
// impl<'de, 'a> Deserialize<'de> for Text<'a> {
//     fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//     where
//         __D: Deserializer<'de>,
//     {
//         #[allow(non_camel_case_types)]
//         #[doc(hidden)]
//         enum __Field {
//             __field0,
//             __field1,
//             __ignore,
//         }
//         #[doc(hidden)]
//         struct __FieldVisitor;
//         #[automatically_derived]
//         impl<'de> Visitor<'de> for __FieldVisitor {
//             type Value = __Field;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "field identifier")
//             }
//             fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     0u64 => Ok(__Field::__field0),
//                     1u64 => Ok(__Field::__field1),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_str<__E>(self, __value: &str) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     "utf8" => Ok(__Field::__field0),
//                     "epilog" => Ok(__Field::__field1),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_bytes<__E>(self, __value: &[u8]) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     b"utf8" => Ok(__Field::__field0),
//                     b"epilog" => Ok(__Field::__field1),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//         }
//         #[automatically_derived]
//         impl<'de> Deserialize<'de> for __Field {
//             #[inline]
//             fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//             where
//                 __D: Deserializer<'de>,
//             {
//                 Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//             }
//         }
//         #[doc(hidden)]
//         struct __Visitor<'de, 'a> {
//             marker: PhantomData<Text<'a>>,
//             lifetime: PhantomData<&'de ()>,
//         }
//         #[automatically_derived]
//         impl<'de, 'a> Visitor<'de> for __Visitor<'de, 'a> {
//             type Value = Text<'a>;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "struct Text")
//             }
//             #[inline]
//             fn visit_seq<__A>(self, mut __seq: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: SeqAccess<'de>,
//             {
//                 let __field0 = match SeqAccess::next_element::<UTF8<'a>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             0usize,
//                             &"struct Text with 2 elements",
//                         ));
//                     }
//                 };
//                 let __field1 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             1usize,
//                             &"struct Text with 2 elements",
//                         ));
//                     }
//                 };
//                 Ok(Text {
//                     utf8: __field0,
//                     epilog: __field1,
//                 })
//             }
//             #[inline]
//             fn visit_map<__A>(self, mut __map: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: MapAccess<'de>,
//             {
//                 let mut __field0: Option<UTF8<'a>> = None;
//                 let mut __field1: Option<Option<Comment<'a>>> = None;
//                 while let Some(__key) = MapAccess::next_key::<__Field>(&mut __map)? {
//                     match __key {
//                         __Field::__field0 => {
//                             if Option::is_some(&__field0) {
//                                 return Err(<__A::Error as Error>::duplicate_field("utf8"));
//                             }
//                             __field0 = Some(MapAccess::next_value::<UTF8<'a>>(&mut __map)?);
//                         }
//                         __Field::__field1 => {
//                             if Option::is_some(&__field1) {
//                                 return Err(<__A::Error as Error>::duplicate_field("epilog"));
//                             }
//                             __field1 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         _ => {
//                             let _ = MapAccess::next_value::<IgnoredAny>(&mut __map)?;
//                         }
//                     }
//                 }
//                 let __field0 = match __field0 {
//                     Some(__field0) => __field0,
//                     None => de::missing_field("utf8")?,
//                 };
//                 let __field1 = match __field1 {
//                     Some(__field1) => __field1,
//                     None => de::missing_field("epilog")?,
//                 };
//                 Ok(Text {
//                     utf8: __field0,
//                     epilog: __field1,
//                 })
//             }
//         }
//         #[doc(hidden)]
//         const FIELDS: &'static [&'static str] = &["utf8", "epilog"];
//         Deserializer::deserialize_struct(
//             __deserializer,
//             "Text",
//             FIELDS,
//             __Visitor {
//                 marker: PhantomData::<Text<'a>>,
//                 lifetime: PhantomData,
//             },
//         )
//     }
// }

struct Items<'w, 'a: 'w, 's: 'w, const LOBOTOMIZE: bool>(&'w [Cell<super::Item<'a, 's>>]);
impl<'w, 'a: 'w, 's: 'w, const LOBOTOMIZE: bool> Serialize for Items<'w, 'a, 's, LOBOTOMIZE> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for cell in self.0 {
            seq.serialize_element(&Item::<LOBOTOMIZE>(cell.get()))?;
        }
        seq.end()
    }
}

struct List<'a, 'store, const LOBOTOMIZE: bool>(super::List<'a, 'store>);
impl<'a, 'store, const LOBOTOMIZE: bool> Serialize for List<'a, 'store, LOBOTOMIZE> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        if LOBOTOMIZE {
            Items::<LOBOTOMIZE>(self.0.cells).serialize(s)
        } else {
            let count = 1 + self.0.prolog.is_some() as usize + self.0.epilog.is_some() as usize;
            let mut fields = s.serialize_struct("List", count)?;
            if let Some(prolog) = self.0.prolog {
                fields.serialize_field("prolog", &UTF8(prolog.utf8))?;
            }
            fields.serialize_field("cells", &Items::<LOBOTOMIZE>(self.0.cells))?;
            if let Some(epilog) = self.0.epilog {
                fields.serialize_field("epilog", &UTF8(epilog.utf8))?;
            }
            fields.end()
        }
    }
}
// impl<'de, 'a, 'store> Deserialize<'de> for List<'a, 'store> {
//     fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//     where
//         __D: Deserializer<'de>,
//     {
//         #[allow(non_camel_case_types)]
//         #[doc(hidden)]
//         enum __Field {
//             __field0,
//             __field1,
//             __field2,
//             __ignore,
//         }
//         #[doc(hidden)]
//         struct __FieldVisitor;
//         #[automatically_derived]
//         impl<'de> Visitor<'de> for __FieldVisitor {
//             type Value = __Field;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "field identifier")
//             }
//             fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     0u64 => Ok(__Field::__field0),
//                     1u64 => Ok(__Field::__field1),
//                     2u64 => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_str<__E>(self, __value: &str) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     "cells" => Ok(__Field::__field0),
//                     "prolog" => Ok(__Field::__field1),
//                     "epilog" => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_bytes<__E>(self, __value: &[u8]) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     b"cells" => Ok(__Field::__field0),
//                     b"prolog" => Ok(__Field::__field1),
//                     b"epilog" => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//         }
//         #[automatically_derived]
//         impl<'de> Deserialize<'de> for __Field {
//             #[inline]
//             fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//             where
//                 __D: Deserializer<'de>,
//             {
//                 Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//             }
//         }
//         #[doc(hidden)]
//         struct __Visitor<'de, 'a, 'store> {
//             marker: PhantomData<List<'a, 'store>>,
//             lifetime: PhantomData<&'de ()>,
//         }
//         #[automatically_derived]
//         impl<'de, 'a, 'store> Visitor<'de> for __Visitor<'de, 'a, 'store> {
//             type Value = List<'a, 'store>;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "struct List")
//             }
//             #[inline]
//             fn visit_seq<__A>(self, mut __seq: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: SeqAccess<'de>,
//             {
//                 let __field0 = match {
//                     #[doc(hidden)]
//                     struct __DeserializeWith<'de, 'a, 'store> {
//                         value: &'store [Cell<Item<'a, 'store>>],
//                         phantom: PhantomData<List<'a, 'store>>,
//                         lifetime: PhantomData<&'de ()>,
//                     }
//                     #[automatically_derived]
//                     impl<'de, 'a, 'store> Deserialize<'de> for __DeserializeWith<'de, 'a, 'store> {
//                         fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//                         where
//                             __D: Deserializer<'de>,
//                         {
//                             Ok(__DeserializeWith {
//                                 value: crate::serde::deserialize_items(__deserializer)?,
//                                 phantom: PhantomData,
//                                 lifetime: PhantomData,
//                             })
//                         }
//                     }
//                     Option::map(
//                         SeqAccess::next_element::<__DeserializeWith<'de, 'a, 'store>>(&mut __seq)?,
//                         |__wrap| __wrap.value,
//                     )
//                 } {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             0usize,
//                             &"struct List with 3 elements",
//                         ));
//                     }
//                 };
//                 let __field1 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             1usize,
//                             &"struct List with 3 elements",
//                         ));
//                     }
//                 };
//                 let __field2 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             2usize,
//                             &"struct List with 3 elements",
//                         ));
//                     }
//                 };
//                 Ok(List {
//                     cells: __field0,
//                     prolog: __field1,
//                     epilog: __field2,
//                 })
//             }
//             #[inline]
//             fn visit_map<__A>(self, mut __map: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: MapAccess<'de>,
//             {
//                 let mut __field0: Option<&'store [Cell<Item<'a, 'store>>]> = None;
//                 let mut __field1: Option<Option<Comment<'a>>> = None;
//                 let mut __field2: Option<Option<Comment<'a>>> = None;
//                 while let Some(__key) = MapAccess::next_key::<__Field>(&mut __map)? {
//                     match __key {
//                         __Field::__field0 => {
//                             if Option::is_some(&__field0) {
//                                 return Err(<__A::Error as Error>::duplicate_field("cells"));
//                             }
//                             __field0 = Some({
//                                 #[doc(hidden)]
//                                 struct __DeserializeWith<'de, 'a, 'store> {
//                                     value: &'store [Cell<Item<'a, 'store>>],
//                                     phantom: PhantomData<List<'a, 'store>>,
//                                     lifetime: PhantomData<&'de ()>,
//                                 }
//                                 #[automatically_derived]
//                                 impl<'de, 'a, 'store> Deserialize<'de> for __DeserializeWith<'de, 'a, 'store> {
//                                     fn deserialize<__D>(
//                                         __deserializer: __D,
//                                     ) -> Result<Self, __D::Error>
//                                     where
//                                         __D: Deserializer<'de>,
//                                     {
//                                         Ok(__DeserializeWith {
//                                             value: crate::serde::deserialize_items(__deserializer)?,
//                                             phantom: PhantomData,
//                                             lifetime: PhantomData,
//                                         })
//                                     }
//                                 }
//                                 match MapAccess::next_value::<__DeserializeWith<'de, 'a, 'store>>(
//                                     &mut __map,
//                                 ) {
//                                     Ok(__wrapper) => __wrapper.value,
//                                     Err(__err) => {
//                                         return Err(__err);
//                                     }
//                                 }
//                             });
//                         }
//                         __Field::__field1 => {
//                             if Option::is_some(&__field1) {
//                                 return Err(<__A::Error as Error>::duplicate_field("prolog"));
//                             }
//                             __field1 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         __Field::__field2 => {
//                             if Option::is_some(&__field2) {
//                                 return Err(<__A::Error as Error>::duplicate_field("epilog"));
//                             }
//                             __field2 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         _ => {
//                             let _ = MapAccess::next_value::<IgnoredAny>(&mut __map)?;
//                         }
//                     }
//                 }
//                 let __field0 = match __field0 {
//                     Some(__field0) => __field0,
//                     None => {
//                         return Err(<__A::Error as Error>::missing_field("cells"));
//                     }
//                 };
//                 let __field1 = match __field1 {
//                     Some(__field1) => __field1,
//                     None => de::missing_field("prolog")?,
//                 };
//                 let __field2 = match __field2 {
//                     Some(__field2) => __field2,
//                     None => de::missing_field("epilog")?,
//                 };
//                 Ok(List {
//                     cells: __field0,
//                     prolog: __field1,
//                     epilog: __field2,
//                 })
//             }
//         }
//         #[doc(hidden)]
//         const FIELDS: &'static [&'static str] = &["cells", "prolog", "epilog"];
//         Deserializer::deserialize_struct(
//             __deserializer,
//             "List",
//             FIELDS,
//             __Visitor {
//                 marker: PhantomData::<List<'a, 'store>>,
//                 lifetime: PhantomData,
//             },
//         )
//     }
// }

struct Entries<'w, 'a: 'w, 's: 'w, const LOBOTOMIZE: bool>(&'w [Cell<super::Entry<'a, 's>>]);
impl<'w, 'a: 'w, 's: 'w, const LOBOTOMIZE: bool> Serialize for Entries<'w, 'a, 's, LOBOTOMIZE> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        if LOBOTOMIZE {
            let mut map = s.serialize_map(Some(self.0.len()))?;
            for cell in self.0 {
                let super::Entry { name, item } = cell.get();
                map.serialize_entry(name.key, &Item::<LOBOTOMIZE>(item))?;
            }
            map.end()
        } else {
            struct Entry<'a, 'store>(super::Entry<'a, 'store>);
            impl<'a, 'store> Serialize for Entry<'a, 'store> {
                fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
                    let count = 2
                        + self.0.name.gap as usize
                        + self.0.name.before.is_some() as usize
                        + match self.0.item {
                            super::Item::Text(text) => text.epilog.is_some() as usize,
                            super::Item::List(list) => {
                                list.prolog.is_some() as usize + list.epilog.is_some() as usize
                            }
                            super::Item::Dict(dict) => {
                                dict.prolog.is_some() as usize + dict.epilog.is_some() as usize
                            }
                        };
                    let mut fields = s.serialize_struct("Entry", count)?;
                    if self.0.name.gap {
                        fields.serialize_field("gap", &true)?;
                    }
                    if let Some(before) = self.0.name.before {
                        fields.serialize_field("before", &UTF8(before.utf8))?;
                    }
                    fields.serialize_field("key", self.0.name.key)?;
                    if let Some(prolog) = match self.0.item {
                        super::Item::Text(_) => None,
                        super::Item::List(list) => list.prolog,
                        super::Item::Dict(dict) => dict.prolog,
                    } {
                        fields.serialize_field("prolog", &UTF8(prolog.utf8))?;
                    }
                    fields.serialize_field("item", &Item::<false>(self.0.item))?;
                    if let Some(epilog) = match self.0.item {
                        super::Item::Text(text) => text.epilog,
                        super::Item::List(list) => list.epilog,
                        super::Item::Dict(dict) => dict.epilog,
                    } {
                        fields.serialize_field("epilog", &UTF8(epilog.utf8))?;
                    }
                    fields.end()
                }
            }
            let mut seq = s.serialize_seq(Some(self.0.len()))?;
            for cell in self.0 {
                seq.serialize_element(&Entry::<'a, 's>(cell.get()))?;
            }
            seq.end()
        }
    }
}

struct Dict<'a, 'store, const LOBOTOMIZE: bool>(super::Dict<'a, 'store>);
impl<'a, 'store, const LOBOTOMIZE: bool> Serialize for Dict<'a, 'store, LOBOTOMIZE> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        if LOBOTOMIZE {
            Entries::<LOBOTOMIZE>(self.0.cells).serialize(s)
        } else {
            let count = 1 + self.0.prolog.is_some() as usize + self.0.epilog.is_some() as usize;
            let mut fields = s.serialize_struct("Dict", count)?;
            if let Some(prolog) = self.0.prolog {
                fields.serialize_field("prolog", &UTF8(prolog.utf8))?;
            }
            fields.serialize_field("cells", &Entries::<LOBOTOMIZE>(self.0.cells))?;
            if let Some(epilog) = self.0.epilog {
                fields.serialize_field("epilog", &UTF8(epilog.utf8))?;
            }
            fields.end()
        }
    }
}
// impl<'de, 'a, 'store> Deserialize<'de> for Dict<'a, 'store> {
//     fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//     where
//         __D: Deserializer<'de>,
//     {
//         #[allow(non_camel_case_types)]
//         #[doc(hidden)]
//         enum __Field {
//             __field0,
//             __field1,
//             __field2,
//             __ignore,
//         }
//         #[doc(hidden)]
//         struct __FieldVisitor;
//         #[automatically_derived]
//         impl<'de> Visitor<'de> for __FieldVisitor {
//             type Value = __Field;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "field identifier")
//             }
//             fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     0u64 => Ok(__Field::__field0),
//                     1u64 => Ok(__Field::__field1),
//                     2u64 => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_str<__E>(self, __value: &str) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     "cells" => Ok(__Field::__field0),
//                     "prolog" => Ok(__Field::__field1),
//                     "epilog" => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_bytes<__E>(self, __value: &[u8]) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     b"cells" => Ok(__Field::__field0),
//                     b"prolog" => Ok(__Field::__field1),
//                     b"epilog" => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//         }
//         #[automatically_derived]
//         impl<'de> Deserialize<'de> for __Field {
//             #[inline]
//             fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//             where
//                 __D: Deserializer<'de>,
//             {
//                 Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//             }
//         }
//         #[doc(hidden)]
//         struct __Visitor<'de, 'a, 'store> {
//             marker: PhantomData<Dict<'a, 'store>>,
//             lifetime: PhantomData<&'de ()>,
//         }
//         #[automatically_derived]
//         impl<'de, 'a, 'store> Visitor<'de> for __Visitor<'de, 'a, 'store> {
//             type Value = Dict<'a, 'store>;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "struct Dict")
//             }
//             #[inline]
//             fn visit_seq<__A>(self, mut __seq: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: SeqAccess<'de>,
//             {
//                 let __field0 = match {
//                     #[doc(hidden)]
//                     struct __DeserializeWith<'de, 'a, 'store> {
//                         value: &'store [Cell<Entry<'a, 'store>>],
//                         phantom: PhantomData<Dict<'a, 'store>>,
//                         lifetime: PhantomData<&'de ()>,
//                     }
//                     #[automatically_derived]
//                     impl<'de, 'a, 'store> Deserialize<'de> for __DeserializeWith<'de, 'a, 'store> {
//                         fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//                         where
//                             __D: Deserializer<'de>,
//                         {
//                             Ok(__DeserializeWith {
//                                 value: crate::serde::deserialize_entries(__deserializer)?,
//                                 phantom: PhantomData,
//                                 lifetime: PhantomData,
//                             })
//                         }
//                     }
//                     Option::map(
//                         SeqAccess::next_element::<__DeserializeWith<'de, 'a, 'store>>(&mut __seq)?,
//                         |__wrap| __wrap.value,
//                     )
//                 } {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             0usize,
//                             &"struct Dict with 3 elements",
//                         ));
//                     }
//                 };
//                 let __field1 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             1usize,
//                             &"struct Dict with 3 elements",
//                         ));
//                     }
//                 };
//                 let __field2 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             2usize,
//                             &"struct Dict with 3 elements",
//                         ));
//                     }
//                 };
//                 Ok(Dict {
//                     cells: __field0,
//                     prolog: __field1,
//                     epilog: __field2,
//                 })
//             }
//             #[inline]
//             fn visit_map<__A>(self, mut __map: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: MapAccess<'de>,
//             {
//                 let mut __field0: Option<&'store [Cell<Entry<'a, 'store>>]> = None;
//                 let mut __field1: Option<Option<Comment<'a>>> = None;
//                 let mut __field2: Option<Option<Comment<'a>>> = None;
//                 while let Some(__key) = MapAccess::next_key::<__Field>(&mut __map)? {
//                     match __key {
//                         __Field::__field0 => {
//                             if Option::is_some(&__field0) {
//                                 return Err(<__A::Error as Error>::duplicate_field("cells"));
//                             }
//                             __field0 = Some({
//                                 #[doc(hidden)]
//                                 struct __DeserializeWith<'de, 'a, 'store> {
//                                     value: &'store [Cell<Entry<'a, 'store>>],
//                                     phantom: PhantomData<Dict<'a, 'store>>,
//                                     lifetime: PhantomData<&'de ()>,
//                                 }
//                                 #[automatically_derived]
//                                 impl<'de, 'a, 'store> Deserialize<'de> for __DeserializeWith<'de, 'a, 'store> {
//                                     fn deserialize<__D>(
//                                         __deserializer: __D,
//                                     ) -> Result<Self, __D::Error>
//                                     where
//                                         __D: Deserializer<'de>,
//                                     {
//                                         Ok(__DeserializeWith {
//                                             value: crate::serde::deserialize_entries(
//                                                 __deserializer,
//                                             )?,
//                                             phantom: PhantomData,
//                                             lifetime: PhantomData,
//                                         })
//                                     }
//                                 }
//                                 match MapAccess::next_value::<__DeserializeWith<'de, 'a, 'store>>(
//                                     &mut __map,
//                                 ) {
//                                     Ok(__wrapper) => __wrapper.value,
//                                     Err(__err) => {
//                                         return Err(__err);
//                                     }
//                                 }
//                             });
//                         }
//                         __Field::__field1 => {
//                             if Option::is_some(&__field1) {
//                                 return Err(<__A::Error as Error>::duplicate_field("prolog"));
//                             }
//                             __field1 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         __Field::__field2 => {
//                             if Option::is_some(&__field2) {
//                                 return Err(<__A::Error as Error>::duplicate_field("epilog"));
//                             }
//                             __field2 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         _ => {
//                             let _ = MapAccess::next_value::<IgnoredAny>(&mut __map)?;
//                         }
//                     }
//                 }
//                 let __field0 = match __field0 {
//                     Some(__field0) => __field0,
//                     None => {
//                         return Err(<__A::Error as Error>::missing_field("cells"));
//                     }
//                 };
//                 let __field1 = match __field1 {
//                     Some(__field1) => __field1,
//                     None => de::missing_field("prolog")?,
//                 };
//                 let __field2 = match __field2 {
//                     Some(__field2) => __field2,
//                     None => de::missing_field("epilog")?,
//                 };
//                 Ok(Dict {
//                     cells: __field0,
//                     prolog: __field1,
//                     epilog: __field2,
//                 })
//             }
//         }
//         #[doc(hidden)]
//         const FIELDS: &'static [&'static str] = &["cells", "prolog", "epilog"];
//         Deserializer::deserialize_struct(
//             __deserializer,
//             "Dict",
//             FIELDS,
//             __Visitor {
//                 marker: PhantomData::<Dict<'a, 'store>>,
//                 lifetime: PhantomData,
//             },
//         )
//     }
// }

struct Item<'a, 'store, const LOBOTOMIZE: bool>(super::Item<'a, 'store>);
impl<'a, 'store, const LOBOTOMIZE: bool> Serialize for Item<'a, 'store, LOBOTOMIZE> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            super::Item::Text(text) => {
                s.serialize_newtype_variant("Item", 0, "Text", &Text::<LOBOTOMIZE>(text))
            }
            super::Item::List(list) => {
                s.serialize_newtype_variant("Item", 1, "List", &List::<LOBOTOMIZE>(list))
            }
            super::Item::Dict(dict) => {
                s.serialize_newtype_variant("Item", 2, "Dict", &Dict::<LOBOTOMIZE>(dict))
            }
        }
    }
}
// impl<'de, 'a, 'store> Deserialize<'de> for Item<'a, 'store> {
//     fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//     where
//         __D: Deserializer<'de>,
//     {
//         #[allow(non_camel_case_types)]
//         #[doc(hidden)]
//         enum __Field {
//             __field0,
//             __field1,
//             __field2,
//         }
//         #[doc(hidden)]
//         struct __FieldVisitor;
//         #[automatically_derived]
//         impl<'de> Visitor<'de> for __FieldVisitor {
//             type Value = __Field;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "variant identifier")
//             }
//             fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     0u64 => Ok(__Field::__field0),
//                     1u64 => Ok(__Field::__field1),
//                     2u64 => Ok(__Field::__field2),
//                     _ => Err(Error::invalid_value(
//                         Unexpected::Unsigned(__value),
//                         &"variant index 0 <= i < 3",
//                     )),
//                 }
//             }
//             fn visit_str<__E>(self, __value: &str) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     "Text" => Ok(__Field::__field0),
//                     "List" => Ok(__Field::__field1),
//                     "Dict" => Ok(__Field::__field2),
//                     _ => Err(Error::unknown_variant(__value, VARIANTS)),
//                 }
//             }
//             fn visit_bytes<__E>(self, __value: &[u8]) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     b"Text" => Ok(__Field::__field0),
//                     b"List" => Ok(__Field::__field1),
//                     b"Dict" => Ok(__Field::__field2),
//                     _ => {
//                         let __value = &from_utf8_lossy(__value);
//                         Err(Error::unknown_variant(__value, VARIANTS))
//                     }
//                 }
//             }
//         }
//         #[automatically_derived]
//         impl<'de> Deserialize<'de> for __Field {
//             #[inline]
//             fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//             where
//                 __D: Deserializer<'de>,
//             {
//                 Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//             }
//         }
//         #[doc(hidden)]
//         struct __Visitor<'de, 'a, 'store> {
//             marker: PhantomData<Item<'a, 'store>>,
//             lifetime: PhantomData<&'de ()>,
//         }
//         #[automatically_derived]
//         impl<'de, 'a, 'store> Visitor<'de> for __Visitor<'de, 'a, 'store> {
//             type Value = Item<'a, 'store>;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "enum Item")
//             }
//             fn visit_enum<__A>(self, __data: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: EnumAccess<'de>,
//             {
//                 match EnumAccess::variant(__data)? {
//                     (__Field::__field0, __variant) => Result::map(
//                         VariantAccess::newtype_variant::<Text<'a>>(__variant),
//                         Item::Text,
//                     ),
//                     (__Field::__field1, __variant) => Result::map(
//                         VariantAccess::newtype_variant::<List<'a, 'store>>(__variant),
//                         Item::List,
//                     ),
//                     (__Field::__field2, __variant) => Result::map(
//                         VariantAccess::newtype_variant::<Dict<'a, 'store>>(__variant),
//                         Item::Dict,
//                     ),
//                 }
//             }
//         }
//         #[doc(hidden)]
//         const VARIANTS: &'static [&'static str] = &["Text", "List", "Dict"];
//         Deserializer::deserialize_enum(
//             __deserializer,
//             "Item",
//             VARIANTS,
//             __Visitor {
//                 marker: PhantomData::<Item<'a, 'store>>,
//                 lifetime: PhantomData,
//             },
//         )
//     }
// }

/// the tindalwic model can be mapped to the serde data model in these ways.
pub enum Mode<'a, 'store> {
    /// map all the data, including all comments.
    Tindalwic(super::File<'a, 'store>),
    /// dumbed down to bridge to formats that mistreat comments.
    Lobotomized(super::File<'a, 'store>),
}
impl<'a, 'store> Serialize for Mode<'a, 'store> {
    fn serialize<S: Ser>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Mode::Lobotomized(file) => Entries::<true>(file.cells).serialize(s),
            Mode::Tindalwic(file) => {
                let count = 1 + file.hashbang.is_some() as usize + file.prolog.is_some() as usize;
                let mut fields = s.serialize_struct("File", count)?;
                if let Some(hashbang) = file.hashbang {
                    fields.serialize_field("hashbang", &UTF8(hashbang.utf8))?;
                }
                if let Some(prolog) = file.prolog {
                    fields.serialize_field("prolog", &UTF8(prolog.utf8))?;
                }
                fields.serialize_field("cells", &Entries::<false>(file.cells))?;
                fields.end()
            }
        }
    }
}

// impl<'de, 'a, 'store> Deserialize<'de> for File<'a, 'store> {
//     fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//     where
//         __D: Deserializer<'de>,
//     {
//         #[allow(non_camel_case_types)]
//         #[doc(hidden)]
//         enum __Field {
//             __field0,
//             __field1,
//             __field2,
//             __ignore,
//         }
//         #[doc(hidden)]
//         struct __FieldVisitor;
//         #[automatically_derived]
//         impl<'de> Visitor<'de> for __FieldVisitor {
//             type Value = __Field;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "field identifier")
//             }
//             fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     0u64 => Ok(__Field::__field0),
//                     1u64 => Ok(__Field::__field1),
//                     2u64 => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_str<__E>(self, __value: &str) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     "cells" => Ok(__Field::__field0),
//                     "hashbang" => Ok(__Field::__field1),
//                     "prolog" => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//             fn visit_bytes<__E>(self, __value: &[u8]) -> Result<Self::Value, __E>
//             where
//                 __E: Error,
//             {
//                 match __value {
//                     b"cells" => Ok(__Field::__field0),
//                     b"hashbang" => Ok(__Field::__field1),
//                     b"prolog" => Ok(__Field::__field2),
//                     _ => Ok(__Field::__ignore),
//                 }
//             }
//         }
//         #[automatically_derived]
//         impl<'de> Deserialize<'de> for __Field {
//             #[inline]
//             fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//             where
//                 __D: Deserializer<'de>,
//             {
//                 Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//             }
//         }
//         #[doc(hidden)]
//         struct __Visitor<'de, 'a, 'store> {
//             marker: PhantomData<File<'a, 'store>>,
//             lifetime: PhantomData<&'de ()>,
//         }
//         #[automatically_derived]
//         impl<'de, 'a, 'store> Visitor<'de> for __Visitor<'de, 'a, 'store> {
//             type Value = File<'a, 'store>;
//             fn expecting(&self, __formatter: &mut Formatter) -> fmt::Result {
//                 Formatter::write_str(__formatter, "struct File")
//             }
//             #[inline]
//             fn visit_seq<__A>(self, mut __seq: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: SeqAccess<'de>,
//             {
//                 let __field0 = match {
//                     #[doc(hidden)]
//                     struct __DeserializeWith<'de, 'a, 'store> {
//                         value: &'store [Cell<Entry<'a, 'store>>],
//                         phantom: PhantomData<File<'a, 'store>>,
//                         lifetime: PhantomData<&'de ()>,
//                     }
//                     #[automatically_derived]
//                     impl<'de, 'a, 'store> Deserialize<'de> for __DeserializeWith<'de, 'a, 'store> {
//                         fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
//                         where
//                             __D: Deserializer<'de>,
//                         {
//                             Ok(__DeserializeWith {
//                                 value: crate::serde::deserialize_entries(__deserializer)?,
//                                 phantom: PhantomData,
//                                 lifetime: PhantomData,
//                             })
//                         }
//                     }
//                     Option::map(
//                         SeqAccess::next_element::<__DeserializeWith<'de, 'a, 'store>>(&mut __seq)?,
//                         |__wrap| __wrap.value,
//                     )
//                 } {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             0usize,
//                             &"struct File with 3 elements",
//                         ));
//                     }
//                 };
//                 let __field1 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             1usize,
//                             &"struct File with 3 elements",
//                         ));
//                     }
//                 };
//                 let __field2 = match SeqAccess::next_element::<Option<Comment<'a>>>(&mut __seq)? {
//                     Some(__value) => __value,
//                     None => {
//                         return Err(Error::invalid_length(
//                             2usize,
//                             &"struct File with 3 elements",
//                         ));
//                     }
//                 };
//                 Ok(File {
//                     cells: __field0,
//                     hashbang: __field1,
//                     prolog: __field2,
//                 })
//             }
//             #[inline]
//             fn visit_map<__A>(self, mut __map: __A) -> Result<Self::Value, __A::Error>
//             where
//                 __A: MapAccess<'de>,
//             {
//                 let mut __field0: Option<&'store [Cell<Entry<'a, 'store>>]> = None;
//                 let mut __field1: Option<Option<Comment<'a>>> = None;
//                 let mut __field2: Option<Option<Comment<'a>>> = None;
//                 while let Some(__key) = MapAccess::next_key::<__Field>(&mut __map)? {
//                     match __key {
//                         __Field::__field0 => {
//                             if Option::is_some(&__field0) {
//                                 return Err(<__A::Error as Error>::duplicate_field("cells"));
//                             }
//                             __field0 = Some({
//                                 #[doc(hidden)]
//                                 struct __DeserializeWith<'de, 'a, 'store> {
//                                     value: &'store [Cell<Entry<'a, 'store>>],
//                                     phantom: PhantomData<File<'a, 'store>>,
//                                     lifetime: PhantomData<&'de ()>,
//                                 }
//                                 #[automatically_derived]
//                                 impl<'de, 'a, 'store> Deserialize<'de> for __DeserializeWith<'de, 'a, 'store> {
//                                     fn deserialize<__D>(
//                                         __deserializer: __D,
//                                     ) -> Result<Self, __D::Error>
//                                     where
//                                         __D: Deserializer<'de>,
//                                     {
//                                         Ok(__DeserializeWith {
//                                             value: crate::serde::deserialize_entries(
//                                                 __deserializer,
//                                             )?,
//                                             phantom: PhantomData,
//                                             lifetime: PhantomData,
//                                         })
//                                     }
//                                 }
//                                 match MapAccess::next_value::<__DeserializeWith<'de, 'a, 'store>>(
//                                     &mut __map,
//                                 ) {
//                                     Ok(__wrapper) => __wrapper.value,
//                                     Err(__err) => {
//                                         return Err(__err);
//                                     }
//                                 }
//                             });
//                         }
//                         __Field::__field1 => {
//                             if Option::is_some(&__field1) {
//                                 return Err(<__A::Error as Error>::duplicate_field("hashbang"));
//                             }
//                             __field1 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         __Field::__field2 => {
//                             if Option::is_some(&__field2) {
//                                 return Err(<__A::Error as Error>::duplicate_field("prolog"));
//                             }
//                             __field2 =
//                                 Some(MapAccess::next_value::<Option<Comment<'a>>>(&mut __map)?);
//                         }
//                         _ => {
//                             let _ = MapAccess::next_value::<IgnoredAny>(&mut __map)?;
//                         }
//                     }
//                 }
//                 let __field0 = match __field0 {
//                     Some(__field0) => __field0,
//                     None => {
//                         return Err(<__A::Error as Error>::missing_field("cells"));
//                     }
//                 };
//                 let __field1 = match __field1 {
//                     Some(__field1) => __field1,
//                     None => de::missing_field("hashbang")?,
//                 };
//                 let __field2 = match __field2 {
//                     Some(__field2) => __field2,
//                     None => de::missing_field("prolog")?,
//                 };
//                 Ok(File {
//                     cells: __field0,
//                     hashbang: __field1,
//                     prolog: __field2,
//                 })
//             }
//         }
//         #[doc(hidden)]
//         const FIELDS: &'static [&'static str] = &["cells", "hashbang", "prolog"];
//         Deserializer::deserialize_struct(
//             __deserializer,
//             "File",
//             FIELDS,
//             __Visitor {
//                 marker: PhantomData::<File<'a, 'store>>,
//                 lifetime: PhantomData,
//             },
//         )
//     }
// }
