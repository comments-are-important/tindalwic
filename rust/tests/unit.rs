//use regex::{Regex, RegexBuilder};
//use std::{string::ToString, sync::LazyLock};
use tindalwic::*;

fn joined(text: Text<'_>) -> String {
    // commented out code in main lib is marginally more efficient
    // todo - move that out to a "with alloc" feature and use it here
    text.lines().collect::<Vec<_>>().join("\n")
}

// #[test]
// fn macro_failures() {
//     trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
// }

#[test]
fn json_text() {
    // this is a very expensive way to do Text::wrap("hi")
    // macro supports it to be consistent, but don't do it this way...
    json! {
        let arena = "hi";
    }
    assert!(arena.list().is_none());
    assert!(arena.dict().is_none());
    assert_eq!(joined(arena.text().unwrap()), "hi");
}

#[test]
fn two_lines() {
    json! {
        let arena = {"key":"one\ntwo"};
    }
    assert!(arena.text().is_none());
    assert!(arena.list().is_none());
    let dict = arena.dict().unwrap();
    assert_eq!(dict.to_string(), "{}\n\t<key>\n\t\tone\n\t\ttwo\n");
}

#[test]
fn nested_lists() {
    json! {
        let arena = [[[["value"]]]];
        let zzz = {"k":"v"};
    }
    let list = arena.list().unwrap();
    assert_eq!(
        list.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue\n"
    );
    walk!{
        let text = [list][0][0][0]<0>.unwrap();
    }
    assert_eq!(joined(*text), "value");
}

#[test]
fn nested_dicts() {
    json!{
        let arena = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}};
    }
    let file = File::new(arena.dict().unwrap().dict);
    assert_eq!(
        file.to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    walk!{
        let text = {file}{"a"}{"b"}{"c"}{"d"}<"k">.unwrap();
    }
    assert_eq!(Vec::from_iter(text.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json!{
        let arena = {"a":{"b":["z"]}};
    }
    let file = File::new(arena.dict().unwrap().dict);
    walk!{
        let text = {file}{"a"}["b"]<0>.unwrap();
    }
    text.assign("c");
    text.persist();
    assert_eq!(file.to_string(), "{a}\n\t[b]\n\t\tc\n");
}

#[test]
fn change_in_dict() {
    json!{
        let arena = {"a":[{"b":"z"}]};
    }
    let file = File::new(arena.dict().unwrap().dict);
    walk!{
        let text = {file}["a"]{0}<"b">.unwrap();
    }
    let mut keyed = text.cell.get();
    keyed.value = Text::wrap("c");
    text.cell.set(keyed);
    assert_eq!(file.to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    let value = "v";
    json!{
        let arena = {"k":value};
    }
    let file = File::new(arena.dict().unwrap().dict);
    walk!{
        let text = {file}<"k">.unwrap();
    }
    let mut keyed = text.cell.get();
    keyed.before = Comment::some("b");
    if let Value::Text(ref mut text) = keyed.value {
        text.epilog = Comment::some("c");
    }
    text.cell.set(keyed);
    assert_eq!(file.to_string(), "//b\nk=v\n#c\n");
}

#[test]
fn change_structure() {
    let key = "k";
    json!{
        let arena = {key:["v"]};
    }
    let dict = arena.dict().unwrap();
    walk!{
        let list = {dict}[key].unwrap();
    }
    json!{
        let patch = {"p":(list)};
    }
    let mut keyed = list.cell.get();
    keyed.value = patch.value();
    list.cell.set(keyed);
    assert_eq!(dict.to_string(), "{}\n\t{k}\n\t\t[p]\n\t\t\tv\n")
}

#[test]
fn prototype_input() {
    // proof that Arena might be used for Input.
    // no idea yet how to pick the const sizes for the arrays.
    let it = "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n";
    let mut arena = Arena {
        value_cells: &Value::array::<2>(),
        keyed_cells: &Keyed::array::<7>(),
        value_next: 0,
        keyed_next: 0,
    };
    arena
        .tv(&it[11..14])
        .tk(&it[41..42], &it[43..44])
        .dk(&it[34..35], 0..1)
        .dk(&it[27..28], 1..2)
        .dk(&it[21..22], 2..3)
        .tk(&it[0..1], &it[2..5])
        .lk(&it[7..8], 0..1)
        .dk(&it[16..17], 3..4)
        .dv(4..7);
    let file = File::new(arena.dict().unwrap().dict);
    assert_eq!(file.to_string(), it);
}

/*
// move to tests/macro_err/badlife.rs
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
