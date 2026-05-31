#![allow(missing_docs)]

use tindalwic::{Comment, Entry, File, Item, arena, json, path};

// #[test]
// fn macro_failures() {
//     trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
// }

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
            let arena = Arena::new(&bump);

            let mut de = serde_json::Deserializer::from_str(r#"{ "key":"one\ntwo" }"#);
            let file: File = Neutered::bumpalo_seed(&arena).deserialize(&mut de).unwrap();

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
        let parsed = arena.parse_or_panic(&encoded);
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
    assert_eq!(encoded, "#!\n#\n//\n{}\n");
}
#[test]
fn text_stretch_bug() {
    let content = "[K]\n\tV\n#L\n";
    arena! {
        let mut arena = <1dict,1list>;
    }
    let file = arena.parse_or_panic(content);
    assert_eq!(file.to_string(), content);
}

#[test]
fn two_lines() {
    json! {
        let entries = {"key":"one\ntwo"}.unwrap();
    }
    assert_eq!(
        File::raise(&Item::dict(entries)).unwrap().to_string(),
        "<key>\n\tone\n\ttwo\n"
    );
}

#[test]
fn multi_line_key() {
    arena! {
        let mut arena = <2dict>;
    }
    let data = "@one\n\ttwo\n<>\n\tv\n";
    let file = arena.parse_or_panic(data);
    assert_eq!(file.to_string(), data);
    let report = |err| {
        print!("{err}");
        tindalwic::parse::Reported::Continue
    };
    assert!(arena.parse("@", report).is_none());
    assert!(arena.parse("@k", report).is_none());
    assert!(arena.parse("@k\n", report).is_none());
    assert!(arena.parse("@k\n<", report).is_none());
    assert!(arena.parse("@k\n<>", report).is_some());
    assert!(arena.parse("@k\n<x>", report).is_none());
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
    let cell = path!({""}[0][0][0][0]Text).walk(file.lower()).unwrap();
    let Item::Text { value, .. } = cell.get() else {
        panic!("impossible because walk checks");
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
        File::raise(&dict).unwrap().to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    let cell = path!({"a"}{"b"}{"c"}{"d"}{"k"}Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get().item else {
        panic!("impossible because walk checks");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let entries = {"a":{"b":["z"]}}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"a"}{"b"}[0]Text).walk(dict).unwrap();
    let Item::Text { .. } = cell.get() else {
        panic!("impossible because walk checks");
    };
    cell.set(Item::text("c"));
    assert_eq!(
        File::raise(&dict).unwrap().to_string(),
        "{a}\n\t[b]\n\t\tc\n"
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
        File::raise(&dict).unwrap().to_string(),
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
        panic!("impossible because path checked");
    };
    let epilog = some_comment("c");
    entry.before = some_comment("b");
    entry.item = Item::Text { value, epilog };
    cell.set(entry);
    assert_eq!(File::raise(&dict).unwrap().to_string(), "//b\nk=v\n#c\n");
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
        panic!("impossible because path checked");
    };
    let b = String::from("b");
    let epilog = some_comment(&b);
    json! {
        let patch = {"p":(Item::Text { value, epilog })}.unwrap();
    }
    cell.set(Item::dict(patch));
    assert_eq!(
        File::raise(&dict).unwrap().to_string(),
        "[k]\n\t{}\n\t\tp=v\n\t\t#b\n"
    )
}

/*
// move to tests/macro_err/
#[test]
#[cfg(feature = "alloc")]
fn json_text() {
    // used to work but now gets red squiggled
    json! {
        let just_a_text = "hi".unwrap();
        let just_a_copy = (just_a_text).unwrap();
    }
    assert_eq!(just_a_text.joined(), "hi");
    assert_eq!(just_a_copy.joined(), "hi");
}
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
