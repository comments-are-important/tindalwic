#![allow(missing_docs)]

use tindalwic::parse::Parse;
use tindalwic::{Comment, Entry, File, Item, arena, json, path};

// #[test]
// fn macro_failures() {
//     trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
// }

fn from_literal(literal: &'static str) -> String {
    let mut lines = literal.lines().enumerate();
    let Some((_, line)) = lines.next() else {
        return "".into();
    };
    assert!(line.is_empty(), "start on 2nd line");
    let Some((_, line)) = lines.next() else {
        return "".into();
    };
    let mut result = line.trim_start().to_owned();
    let prefix = &line[0..line.len() - result.len()];
    let mut more = lines.next();
    while let Some((_, line)) = more {
        let Some(mut remainder) = line.strip_prefix(prefix) else {
            break;
        };
        result.push('\n');
        while let Some(trailing) = remainder.strip_prefix("    ") {
            result.push('\t');
            remainder = trailing;
        }
        result.push_str(remainder);
        more = lines.next()
    }
    if let Some((num, line)) = more {
        assert!(lines.next().is_none(), "line {num} isn't indented");
        assert!(line.trim().is_empty(), "last line isn't blank");
    }
    result.push('\n');
    result
}

fn some_comment<'a>(value: &'a str) -> Option<Comment<'a>> {
    Some(Comment {
        value: value.into(),
    })
}

#[cfg(feature = "alloc")]
mod alloc_tests {
    use super::*;

    #[cfg(all(feature = "bumpalo", feature = "serde"))]
    mod serde_tests {
        use super::*;
        use bumpalo::Bump;
        use serde::de::DeserializeSeed;
        use tindalwic::bumpalo::Arena;
        use tindalwic::serde::Neutered;
        #[test]
        fn deserialize_file_from_json() {
            let bump = Bump::new();
            let mut arena = Arena::new(&bump);

            let mut de = serde_json::Deserializer::from_str(r#"{ "key":"one\ntwo" }"#);
            let file: File = Neutered::seed(&mut arena).deserialize(&mut de).unwrap();

            json! {
                let entries = {"key":"one\ntwo"}.unwrap();
            }
            assert_eq!(file.cells, entries);
        }
    }

    #[test]
    fn hashbang_avoidance() {
        let mut file = File::default();
        file.prolog = some_comment("!suspect");
        let encoded = file.to_string();
        assert_eq!(encoded, "#\n\t!suspect\n");
        arena! {
            let mut arena = <1dict>;
        }
        let parsed = arena.panic_first_error(&encoded);
        assert!(parsed.hashbang.is_none());
        assert_eq!(
            Vec::from_iter(parsed.prolog.unwrap().value.lines()),
            vec!["!suspect"]
        );
    }
}

#[test]
fn three_blank_comments() {
    let entry = Entry {
        before: some_comment(""),
        item: Item::dict(&[]),
        ..Default::default()
    };
    let entries = [core::cell::Cell::new(entry)];
    let file = File {
        hashbang: some_comment(""),
        prolog: some_comment(""),
        cells: &entries,
    };
    let encoded = file.to_string();
    let expect = "
        #!
        #
        //
        {}
    ";
    assert_eq!(encoded, from_literal(expect));
}
#[test]
fn text_stretch_bug() {
    let spaces = "
        [K]
            V
        #E
    ";
    let content = from_literal(spaces);
    assert_eq!("[K]\n\tV\n#E\n", content);
    arena! {
        let mut arena = <1dict,1list>;
    }
    let file = arena.panic_first_error(&content);
    assert_eq!(file.to_string(), content);
}

#[test]
fn two_lines() {
    json! {
        let entries = {"key":"one\ntwo"}.unwrap();
    }
    assert_eq!(
        File::try_from_dict_without_epilog(&Item::dict(entries))
            .unwrap()
            .to_string(),
        "<key>\n\tone\n\ttwo\n"
    );
}

#[test]
fn multi_line_key() {
    arena! {
        let mut arena = <2dict>;
    }
    let data = "@one\n\ttwo\n<>\n\tv\n";
    let file = arena.panic_first_error(data);
    assert_eq!(file.to_string(), data);
    let report = &mut |err| {
        print!("{err}");
        tindalwic::parse::Reported::Continue
    };
    assert!(arena.report_errors("@", report).is_none());
    assert!(arena.report_errors("@k", report).is_none());
    assert!(arena.report_errors("@k\n", report).is_none());
    assert!(arena.report_errors("@k\n<", report).is_none());
    assert!(arena.report_errors("@k\n<>", report).is_some());
    assert!(arena.report_errors("@k\n<x>", report).is_none());
}

#[test]
fn nested_lists() {
    json! {
        let items = [[[["value"]]]].unwrap();
    }
    let mut array = Entry::array::<1>();
    array[0].get_mut().item = Item::list(items);
    let file = File {
        cells: &array[..],
        ..Default::default()
    };
    assert_eq!(
        file.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue\n"
    );
    let cell = path!({""}[0][0][0][0]Text)
        .walk(file.embed_without_hashbang())
        .unwrap();
    let Item::Text { value, .. } = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["value"]);
}

#[test]
fn nested_dicts() {
    json! {
        let entries = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}}.unwrap();
    }
    let dict = Item::dict(entries);
    let mut keys = Vec::new();
    for entry in entries {
        let entry = entry.get();
        keys.push(entry.key.lines().next().unwrap_or(""));
    }
    assert_eq!(keys, vec!["1", "2", "a"]);
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    let cell = path!({"a"}{"b"}{"c"}{"d"}{"k"}Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get().item else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let entries = {"a":{"b":["v"]}}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"a"}{"b"}[0]Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let epilog = some_comment("c");
    cell.set(Item::Text { value, epilog });
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "{a}\n\t[b]\n\t\tv\n\t\t#c\n"
    );
}

#[test]
fn change_in_dict() {
    json! {
        let entries = {"a":[{"b":"z"}]}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"a"}[0]{"b"}Text).walk(dict).unwrap();
    let mut entry = cell.get();
    entry.item = Item::text("c");
    cell.set(entry);
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "[a]\n\t{}\n\t\tb=c\n"
    );
}

#[test]
fn inject_comments() {
    json! {
        let entries = {"k":"v"}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"k"}Text).walk(dict).unwrap();
    let mut entry = cell.get();
    let Item::Text { value, .. } = entry.item else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let epilog = some_comment("c");
    entry.before = some_comment("b");
    entry.item = Item::Text { value, epilog };
    cell.set(entry);
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "//b\nk=v\n#c\n"
    );
}

#[test]
fn change_structure() {
    let key = "k";
    json! {
        let entries = {key:["v"]}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({key}[0]Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let b = String::from("b");
    let epilog = some_comment(&b);
    json! {
        let patch = {"p":(Item::Text { value, epilog })}.unwrap();
    }
    cell.set(Item::dict(patch));
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "[k]\n\t{}\n\t\tp=v\n\t\t#b\n"
    )
}

#[cfg(all(feature = "bumpalo", feature = "serde"))]
mod data_format {
    use ::serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use tindalwic::serde::format::{Error, Result, from_tindalwic, to_tindalwic};

    fn round_trip<T: Serialize + for<'de> Deserialize<'de>>(
        value: &T, // important this is a ref
    ) -> Result<T> {
        let mut data = BTreeMap::new();
        data.insert("data", value);
        let string = to_tindalwic(&data)?;
        let mut file: BTreeMap<&str, T> = from_tindalwic(&string)?;
        file.remove("data")
            .ok_or_else(|| Error::new("where did data go?"))
    }

    #[test]
    fn boolean() {
        assert_eq!(true, round_trip(&true).unwrap());
        assert_eq!(false, round_trip(&false).unwrap());
    }
    #[test]
    fn signed_1_byte() {
        assert_eq!(i8::MIN, round_trip(&i8::MIN).unwrap());
        assert_eq!(i8::MAX, round_trip(&i8::MAX).unwrap());
    }
    #[test]
    fn signed_2_bytes() {
        assert_eq!(i16::MIN, round_trip(&i16::MIN).unwrap());
        assert_eq!(i16::MAX, round_trip(&i16::MAX).unwrap());
    }
    #[test]
    fn signed_4_bytes() {
        assert_eq!(i32::MIN, round_trip(&i32::MIN).unwrap());
        assert_eq!(i32::MAX, round_trip(&i32::MAX).unwrap());
    }
    #[test]
    fn signed_8_bytes() {
        assert_eq!(i64::MIN, round_trip(&i64::MIN).unwrap());
        assert_eq!(i64::MAX, round_trip(&i64::MAX).unwrap());
    }
    #[test]
    fn signed_16_bytes() {
        assert_eq!(i128::MIN, round_trip(&i128::MIN).unwrap());
        assert_eq!(i128::MAX, round_trip(&i128::MAX).unwrap());
    }
    #[test]
    fn unsigned_1_byte() {
        assert_eq!(u8::MIN, round_trip(&u8::MIN).unwrap());
        assert_eq!(u8::MAX, round_trip(&u8::MAX).unwrap());
    }
    #[test]
    fn unsigned_2_bytes() {
        assert_eq!(u16::MIN, round_trip(&u16::MIN).unwrap());
        assert_eq!(u16::MAX, round_trip(&u16::MAX).unwrap());
    }
    #[test]
    fn unsigned_4_bytes() {
        assert_eq!(u32::MIN, round_trip(&u32::MIN).unwrap());
        assert_eq!(u32::MAX, round_trip(&u32::MAX).unwrap());
    }
    #[test]
    fn unsigned_8_bytes() {
        assert_eq!(u64::MIN, round_trip(&u64::MIN).unwrap());
        assert_eq!(u64::MAX, round_trip(&u64::MAX).unwrap());
    }
    #[test]
    fn unsigned_16_bytes() {
        assert_eq!(u128::MIN, round_trip(&u128::MIN).unwrap());
        assert_eq!(u128::MAX, round_trip(&u128::MAX).unwrap());
    }
    #[test]
    fn float_4_bytes() {
        assert_eq!(f32::MIN, round_trip(&f32::MIN).unwrap());
        assert_eq!(f32::MAX, round_trip(&f32::MAX).unwrap());
        assert_eq!(f32::EPSILON, round_trip(&f32::EPSILON).unwrap());
        assert_eq!(f32::MIN_POSITIVE, round_trip(&f32::MIN_POSITIVE).unwrap());
        const E: f32 = std::f32::consts::E;
        assert_eq!(E, round_trip(&E).unwrap());
        const PI: f32 = std::f32::consts::PI;
        assert_eq!(PI, round_trip(&PI).unwrap());
    }
    #[test]
    fn float_8_bytes() {
        assert_eq!(f64::MIN, round_trip(&f64::MIN).unwrap());
        assert_eq!(f64::MAX, round_trip(&f64::MAX).unwrap());
        assert_eq!(f64::EPSILON, round_trip(&f64::EPSILON).unwrap());
        assert_eq!(f64::MIN_POSITIVE, round_trip(&f64::MIN_POSITIVE).unwrap());
        const E: f64 = std::f64::consts::E;
        assert_eq!(E, round_trip(&E).unwrap());
        const PI: f64 = std::f64::consts::PI;
        assert_eq!(PI, round_trip(&PI).unwrap());
    }
    #[test]
    fn character() {
        assert_eq!(char::MIN, round_trip(&char::MIN).unwrap());
        assert_eq!(char::MAX, round_trip(&char::MAX).unwrap());
    }
    #[test]
    fn string() {
        let data = String::from("");
        assert_eq!(data, round_trip(&data).unwrap());
        let data = String::from("hello");
        assert_eq!(data, round_trip(&data).unwrap());
        // TODO: let data = "";
        // assert_eq!(data, round_trip(&data).unwrap());
    }
    #[test]
    fn byte_array() {
        let data = Vec::<u8>::new();
        assert_eq!(data, round_trip(&data).unwrap());
        let data = vec![u8::MIN, u8::MAX];
        assert_eq!(data, round_trip(&data).unwrap());
    }
    #[test]
    fn option_and_unit() {
        assert_eq!((), round_trip(&()).unwrap());
        let data: Option<u8> = None;
        assert_eq!(data, round_trip(&data).unwrap());
        let data: Option<u8> = Some(u8::MAX);
        assert_eq!(data, round_trip(&data).unwrap());
        // TODO: Some(()) comes back as None
    }
    #[test]
    fn tuple() {
        let data = (false, true);
        assert_eq!(data, round_trip(&data).unwrap());
        let data = [0, 1, 2, 3, 4, 5, 6];
        assert_eq!(data, round_trip(&data).unwrap());
    }
    #[test]
    fn enums() {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        enum Enum {
            Unit,
            Newtype(bool),
            Tuple(bool, bool),
            Struct { one: bool, two: bool },
        }
        let data = Enum::Unit;
        assert_eq!(data, round_trip(&data).unwrap());
        let data = Enum::Newtype(false);
        assert_eq!(data, round_trip(&data).unwrap());
        let data = Enum::Tuple(false, true);
        assert_eq!(data, round_trip(&data).unwrap());
        let data = Enum::Struct {
            one: false,
            two: true,
        };
        assert_eq!(data, round_trip(&data).unwrap());
    }
    #[test]
    fn structs() {
        // TODO: invalid type string ""
        // #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        // struct Unit;
        // let data = Unit;
        // assert_eq!(data, round_trip(&data).unwrap());

        // TODO: invalid type string "false"
        // #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        // struct Newtype(bool);
        // let data = Newtype(false);
        // assert_eq!(data, round_trip(&data).unwrap());

        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        struct Tuple(bool, bool);
        let data = Tuple(false, true);
        assert_eq!(data, round_trip(&data).unwrap());
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        struct Struct {
            one: bool,
            two: bool,
        }
        let data = Struct {
            one: false,
            two: true,
        };
        assert_eq!(data, round_trip(&data).unwrap());
    }
    #[test]
    fn seq() {
        let data = Vec::<bool>::new();
        assert_eq!(data, round_trip(&data).unwrap());
        let data = vec!['a', 'b', 'c', 'd', 'e'];
        assert_eq!(data, round_trip(&data).unwrap());
    }
    #[test]
    fn map() {
        let mut data = BTreeMap::<String, char>::new();
        assert_eq!(data, round_trip(&data).unwrap());
        data.insert("zero".into(), '0');
        data.insert("one".into(), '1');
        data.insert("two".into(), '2');
        data.insert("three".into(), '3');
        assert_eq!(data, round_trip(&data).unwrap());
    }
}

/*
// move to tests/macro_err/
fn zzz() {
    let mut hi = String::from("hi");
    let mut root = tindalwic!([hi[..]]);
    //hi.clear(); // won't compile
    let result = path!([0]).text_mut(&mut root).unwrap();
    result.epilog = Some("changed".into());
    //assert_eq!(text.epilog.unwrap().gfm.to_string(), "hi");
    hi.clear();
}
*/
