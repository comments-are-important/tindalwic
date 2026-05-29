#![allow(missing_docs)]

use tindalwic::{Comment, Dict, Entry, File, Item, Value, arena, json, walk};

// #[test]
// fn macro_failures() {
//     trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
// }

fn some_comment<'a>(value: &'a str) -> Option<Comment<'a>> {
    Some(Comment {
        value: Value::new(value),
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
                let expected = {"key":"one\ntwo"}.unwrap();
            }
            assert_eq!(file.cells, expected.cells);
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
        item: Item::Dict(Dict::default()),
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
        let dict = {"key":"one\ntwo"}.unwrap();
    }
    assert_eq!(dict.to_string(), "{}\n\t<key>\n\t\tone\n\t\ttwo\n");
}

#[test]
fn nested_lists() {
    json! {
        let list = [[[["value"]]]].unwrap();
    }
    assert_eq!(
        list.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue\n"
    );
    walk! {
        let text = [list][0][0][0]<0>.unwrap();
    }
    assert_eq!(Vec::from_iter(text.value.lines()), vec!["value"]);
}

#[test]
fn nested_dicts() {
    json! {
        let dict = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}}.unwrap();
    }
    let mut keys = Vec::new();
    for entry in dict.cells {
        let entry = entry.get();
        keys.push(entry.key.lines().next().unwrap_or(""));
    }
    assert_eq!(keys, vec!["1", "2", "a"]);
    assert_eq!(
        dict.to_string(),
        "{}\n\t1=one\n\t[2]\n\t\ttwo\n\t{a}\n\t\t{b}\n\t\t\t{c}\n\t\t\t\t{d}\n\t\t\t\t\tk=v\n"
    );
    walk! {
        let text = {dict}{"a"}{"b"}{"c"}{"d"}<"k">.unwrap();
    }
    assert_eq!(Vec::from_iter(text.value.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let dict = {"a":{"b":["z"]}}.unwrap();
    }
    walk! {
        let (_text, cell) = {dict}{"a"}["b"]<0>.unwrap();
    }
    cell.set(Item::text("c"));
    assert_eq!(dict.to_string(), "{}\n\t{a}\n\t\t[b]\n\t\t\tc\n");
}

#[test]
fn change_in_dict() {
    json! {
        let dict = {"a":[{"b":"z"}]}.unwrap();
    }
    walk! {
        let (_text, mut entry, cell) = {dict}["a"]{0}<"b">.unwrap();
    }
    entry.item = Item::text("c");
    cell.set(entry);
    assert_eq!(dict.to_string(), "{}\n\t[a]\n\t\t{}\n\t\t\tb=c\n");
}

#[test]
fn inject_comments() {
    json! {
        let dict = {"k":"v"}.unwrap();
    }
    walk! {
        let (mut text, mut entry, cell) = {dict}<"k">.unwrap();
    }
    entry.before = some_comment("b");
    text.epilog = some_comment("c");
    entry.item = Item::Text(text);
    cell.set(entry);
    assert_eq!(dict.to_string(), "{}\n\t//b\n\tk=v\n\t#c\n");
}

#[test]
fn change_structure() {
    let key = "k";
    json! {
        let changing = {"k":["v"]}.unwrap();
    }
    walk! {
        let (mut resolved, cell) = {changing}[key]<0>.unwrap();
    }
    let b = String::from("b");
    resolved.epilog = some_comment(&b);
    json! {
        let patch = {"p":(resolved)}.unwrap();
    }
    cell.set(patch.into());
    assert_eq!(
        changing.to_string(),
        "{}\n\t[k]\n\t\t{}\n\t\t\tp=v\n\t\t\t#b\n"
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
