//use regex::{Regex, RegexBuilder};
//use std::{string::ToString, sync::LazyLock};
use tindalwic::*;

fn joined(text: Text<'_>) -> String {
    text.lines().collect::<Vec<_>>().join("\n")
}

#[test]
fn macro_failures() {
    trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
}

#[test]
fn json_text() {
    json!(arena = "hi");
    assert!(arena.list().is_none());
    assert!(arena.dict().is_none());
    assert_eq!(joined(arena.text().unwrap()), "hi");
}

#[test]
fn empty_file() {
    json!(arena = {});
    assert!(arena.text().is_none());
    assert!(arena.list().is_none());
    let dict = arena.dict().unwrap();
    assert!(File::new(dict.dict).is_empty());
}

#[test]
fn two_lines() {
    json!(arena = {"key":"one\ntwo"});
    let dict = arena.dict().unwrap();
    assert_eq!(dict.to_string(), "{}\n\t<key>\n\t\tone\n\t\ttwo\n");
}

#[test]
fn nested_lists() {
    json!(arena = [[[["value"]]]]);
    let list = arena.list().unwrap();
    assert_eq!(
        list.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue\n"
    );
    let (text, _cell) = walk!([list][0][0][0]<0>).unwrap();
    assert_eq!(joined(text), "value");
}

#[test]
fn nested_dicts() {
    json!(arena = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}});
    let file = File::new(arena.dict().unwrap().dict);
    assert_eq!(
        file.to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    let (text, _cell) = walk!({file}{"a"}{"b"}{"c"}{"d"}<"k">).unwrap();
    assert_eq!(Vec::from_iter(text.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json!(arena = {"a":{"b":["z"]}});
    let file = File::new(arena.dict().unwrap().dict);
    let (_text, cell) = walk!({file}{"a"}["b"]<0>).unwrap();
    cell.set(Value::Text(Text::wrap("c")));
    assert_eq!(file.to_string(), "{a}\n\t[b]\n\t\tc\n");
}

#[test]
fn change_in_dict() {
    json!(arena = {"a":[{"b":"z"}]});
    let file = File::new(arena.dict().unwrap().dict);
    let (_text, cell) = walk!({file}["a"]{0}<"b">).unwrap();
    let mut keyed = cell.get();
    keyed.value = Value::Text(Text::wrap("c"));
    cell.set(keyed);
    assert_eq!(file.to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    json!(arena = {"k":"v"});
    let file = File::new(arena.dict().unwrap().dict);
    let (_text, cell) = walk!({file}<"k">).unwrap();
    let mut keyed = cell.get();
    keyed.before = Comment::some("b");
    if let Value::Text(ref mut text) = keyed.value {
        text.epilog = Comment::some("c");
    }
    cell.set(keyed);
    assert_eq!(file.to_string(), "//b\nk=v\n#c\n");
}

#[test]
fn parse_manually() {
    let mut arena = Arena {
        utf8_bytes: "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n",
        value_cells: &Value::array::<2>(),
        keyed_cells: &Keyed::array::<7>(),
        value_next: 0,
        keyed_next: 0,
    };
    arena
        .tv(11..14)
        .tk(41..42, 43..44)
        .dk(34..35, 0..1)
        .dk(27..28, 1..2)
        .dk(21..22, 2..3)
        .tk(0..1, 2..5)
        .lk(7..8, 0..1)
        .dk(16..17, 3..4)
        .dv(4..7);
    let file = File::new(arena.dict().unwrap().dict);
    assert_eq!(file.to_string(), arena.utf8_bytes);
}

// #[test]
// fn zzz() {
//     let mut hi = String::from("hi");
//     let mut root = tindalwic!([hi[..]]);
//     //hi.clear(); // won't compile
//     let result = path!([0]).text_mut(&mut root).unwrap();
//     result.epilog = Some("changed".into());
//     //assert_eq!(text.epilog.unwrap().gfm.to_string(), "hi");
//     hi.clear();
// }
