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
    use std::fmt::Debug;
    use tindalwic::parse::Parse as _;
    use tindalwic::serde::format::{from_tindalwic, to_tindalwic};

    fn check<T>(value: T)
    where
        T: Debug + PartialEq + Serialize + for<'de> Deserialize<'de>,
    {
        println!("# {value:?}");
        let json = serde_json::to_string_pretty(&value).unwrap();
        println!("## serde_json\n{}", json);
        let json: T = serde_json::from_str(&json).unwrap();
        if json != value {
            println!("### != {json:?}");
        }
        let mut data = BTreeMap::new();
        data.insert("data", &value);
        let bump = bumpalo::Bump::new();
        let mut arena = tindalwic::bumpalo::Arena::new(&bump);
        let string = to_tindalwic(arena.builder(), &data).unwrap();
        println!("## tindalwic\n{string}");
        let mut file: BTreeMap<&str, T> = from_tindalwic(&mut arena, &string).unwrap();
        // TODO: be better than json, be equal to value...
        assert_eq!(json, file.remove("data").unwrap());
        assert!(file.is_empty());
    }

    #[test]
    fn boolean() {
        check(true);
        check(false);
    }
    #[test]
    fn signed_1_byte() {
        check(i8::MIN);
        check(0i8);
        check(i8::MAX);
    }
    #[test]
    fn signed_2_bytes() {
        check(i16::MIN);
        check(0i16);
        check(i16::MAX);
    }
    #[test]
    fn signed_4_bytes() {
        check(i32::MIN);
        check(0i32);
        check(i32::MAX);
    }
    #[test]
    fn signed_8_bytes() {
        check(i64::MIN);
        check(0i64);
        check(i64::MAX);
    }
    #[test]
    fn signed_16_bytes() {
        check(i128::MIN);
        check(0i128);
        check(i128::MAX);
    }
    #[test]
    fn unsigned_1_byte() {
        check(u8::MIN);
        check(u8::MAX);
    }
    #[test]
    fn unsigned_2_bytes() {
        check(u16::MIN);
        check(u16::MAX);
    }
    #[test]
    fn unsigned_4_bytes() {
        check(u32::MIN);
        check(u32::MAX);
    }
    #[test]
    fn unsigned_8_bytes() {
        check(u64::MIN);
        check(u64::MAX);
    }
    #[test]
    fn unsigned_16_bytes() {
        check(u128::MIN);
        check(u128::MAX);
    }
    #[test]
    fn float_4_bytes() {
        check(f32::MIN);
        check(f32::MAX);
        check(f32::EPSILON);
        check(f32::MIN_POSITIVE);
        check(std::f32::consts::E);
        check(std::f32::consts::PI);
    }
    #[test]
    fn float_8_bytes() {
        check(f64::MIN);
        check(f64::MAX);
        check(f64::EPSILON);
        check(f64::MIN_POSITIVE);
        check(std::f64::consts::E);
        check(std::f64::consts::PI);
    }
    #[test]
    fn character() {
        check(char::MIN);
        check(char::MAX);
    }
    #[test]
    fn string() {
        check(String::from(""));
        check(String::from("hello"));
        //check("");
    }
    #[test]
    fn byte_array() {
        check(Vec::<u8>::new());
        check(vec![u8::MIN, u8::MAX]);
    }
    #[test]
    fn option_and_unit() {
        check(());
        check(Option::<u8>::None);
        check(Some(u8::MAX));
        check(Some(()));
        check(Option::<Option<u8>>::Some(None));
    }
    #[test]
    fn tuple() {
        check((false, true));
        check([0, 1, 2, 3, 4, 5]);
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
        check(Enum::Unit);
        check(Enum::Newtype(false));
        check(Enum::Tuple(false, true));
        check(Enum::Struct {
            one: false,
            two: true,
        });
    }
    #[test]
    fn structs() {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        struct Unit;
        check(Unit);
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        struct Newtype(bool);
        check(Newtype(false));
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        struct Tuple(bool, bool);
        check(Tuple(false, true));
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        struct Struct {
            one: bool,
            two: bool,
        }
        check(Struct {
            one: false,
            two: true,
        });
    }
    #[test]
    fn seq() {
        check(Vec::<bool>::new());
        check(vec!['a', 'b', 'c', 'd', 'e']);
    }
    #[test]
    fn map() {
        check(BTreeMap::<String, char>::new());
        let mut map = BTreeMap::<String, char>::new();
        map.insert("zero".into(), '0');
        map.insert("one".into(), '1');
        map.insert("two".into(), '2');
        map.insert("three".into(), '3');
        check(map);
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
