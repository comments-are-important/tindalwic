//use regex::{Regex, RegexBuilder};
//use std::{string::ToString, sync::LazyLock};
use tindalwic::*;

fn joined(text: &Text<'_>) -> String {
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
        let text = "hi";
    }
    assert_eq!(joined(&text), "hi");
}

#[test]
fn two_lines() {
    json! {
        let dict = {"key":"one\ntwo"};
    }
    assert_eq!(dict.to_string(), "{}\n\t<key>\n\t\tone\n\t\ttwo\n");
}

#[test]
fn nested_lists() {
    json! {
        let list = [[[["value"]]]];
    }
    assert_eq!(
        list.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue\n"
    );
    walk! {
        let text = [list][0][0][0]<0>.unwrap();
    }
    assert_eq!(joined(&*text), "value");
}

#[test]
fn nested_dicts() {
    json! {
        let dict = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}};
    }
    let file = File::new(dict.dict);
    assert_eq!(
        file.to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    walk! {
        let text = {file}{"a"}{"b"}{"c"}{"d"}<"k">.unwrap();
    }
    assert_eq!(Vec::from_iter(text.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let dict = {"a":{"b":["z"]}};
    }
    let file = File::new(dict.dict);
    walk! {
        let mut text = {file}{"a"}["b"]<0>.unwrap();
    }
    text.assign("c");
    set!(text);
    assert_eq!(file.to_string(), "{a}\n\t[b]\n\t\tc\n");
}

#[test]
fn change_in_dict() {
    json! {
        let dict = {"a":[{"b":"z"}]};
    }
    let file = File::new(dict.dict);
    walk! {
        let mut text = {file}["a"]{0}<"b">.unwrap();
    }
    text.assign("c");
    set!(text);
    assert_eq!(file.to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    json! {
        let dict = {"k":"v"};
    }
    let file = File::new(dict.dict);
    walk! {
        let mut text = {file}<"k">.unwrap();
    }
    text.before = Comment::some("b");
    text.epilog = Comment::some("c");
    set!(text);
    assert_eq!(file.to_string(), "//b\nk=v\n#c\n");
}

#[test]
fn change_structure() {
    let key = "k";
    json! {
        let dict = {key:["v"]};
    }
    walk! {
        let mut list = {dict}[key].unwrap();
    }
    json! {
        let patch = {"p":(list)};
    }
    set!(list, patch.to_value());
    assert_eq!(dict.to_string(), "{}\n\t{k}\n\t\t[p]\n\t\t\tv\n")
}

#[test]
fn prototype_input() {
    // proof that Arena might be used for Input.
    // no idea yet how to pick the const sizes for the arrays.
    let it = "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n";
    let value_cells = Value::array::<2>();
    let keyed_cells = Keyed::array::<7>();
    let mut arena = Arena::new(&value_cells, &keyed_cells);
    arena.text_in_list(&it[11..14]);
    arena.text_in_dict(&it[41..42], &it[43..44]);
    arena.dict_in_dict(&it[34..35], 0..1);
    arena.dict_in_dict(&it[27..28], 1..2);
    arena.dict_in_dict(&it[21..22], 2..3);
    arena.text_in_dict(&it[0..1], &it[2..5]);
    arena.list_in_dict(&it[7..8], 0..1);
    arena.dict_in_dict(&it[16..17], 3..4);
    arena.dict_in_list(4..7);
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
