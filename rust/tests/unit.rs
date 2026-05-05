#![allow(missing_docs)]

use std::cell::Cell;
use tindalwic::*;

// #[test]
// fn macro_failures() {
//     trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
// }

#[test]
#[cfg(feature = "alloc")]
fn hashbang_avoidance() {
    let nothing = [];
    let mut file = File::wrap(&nothing);
    file.prolog = Comment::some("!suspect");
    let encoded = file.to_string();
    assert_eq!(encoded, "#\n\t!suspect\n");
    arena! {
        $crate = crate;
        let mut arena = <1dict>;
    }
    let parsed = arena.parse_or_panic(&encoded).unwrap();
    assert!(parsed.hashbang.is_none());
    assert_eq!(
        Vec::from_iter(parsed.prolog.unwrap().lines()),
        vec!["!suspect"]
    );
}
#[test]
fn three_blank_comments() {
    let nothing = [];
    let name = Name {
        gap: false,
        before: Comment::some(""),
        key: "",
    };
    let entries = [Cell::new(Entry {
        name,
        item: Item::Dict(Dict::wrap(&nothing)),
    })];
    let mut file = File::wrap(&entries);
    file.hashbang = Comment::some("");
    file.prolog = Comment::some("");
    let encoded = file.to_string();
    assert_eq!(encoded, "#!\n#\n//\n{}\n");
}
#[test]
fn text_stretch_bug() {
    let content = "[K]\n\tV\n#L\n";
    arena! {
        let mut arena = <1dict,1list>;
    }
    let file = arena.parse_or_panic(content).unwrap();
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
    assert_eq!(Vec::from_iter(text.lines()), vec!["value"]);
}

#[test]
fn nested_dicts() {
    json! {
        let dict = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}}.unwrap();
    }
    let mut keys = Vec::new();
    for entry in dict {
        keys.push(entry.name.key);
    }
    assert_eq!(keys, vec!["1", "2", "a"]);
    assert_eq!(
        File::wrap(dict.cells).to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    walk! {
        let text = {dict}{"a"}{"b"}{"c"}{"d"}<"k">.unwrap();
    }
    assert_eq!(Vec::from_iter(text.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let dict = {"a":{"b":["z"]}}.unwrap();
    }
    walk! {
        let (mut text, cell) = {dict}{"a"}["b"]<0>.unwrap();
    }
    text = Text::wrap("c");
    cell.set(text.into());
    assert_eq!(File::wrap(dict.cells).to_string(), "{a}\n\t[b]\n\t\tc\n");
}

#[test]
fn change_in_dict() {
    json! {
        let dict = {"a":[{"b":"z"}]}.unwrap();
    }
    let file = File::wrap(dict.cells);
    walk! {
        let (mut text, name, cell) = {dict}["a"]{0}<"b">.unwrap();
    }
    text = Text::wrap("c");
    cell.set(Entry {
        name,
        item: text.into(),
    });
    assert_eq!(file.to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    json! {
        let dict = {"k":"v"}.unwrap();
    }
    let file = File::wrap(dict.cells);
    walk! {
        let (mut text, mut name, cell) = {dict}<"k">.unwrap();
    }
    name.before = Comment::some("b");
    text.epilog = Comment::some("c");
    cell.set(Entry {
        name,
        item: text.into(),
    });
    assert_eq!(file.to_string(), "//b\nk=v\n#c\n");
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
    resolved.epilog = Comment::some(&b);
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
