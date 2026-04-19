#![allow(missing_docs)]

use tindalwic::*;

// #[test]
// fn macro_failures() {
//     trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
// }

#[test]
#[cfg(feature = "alloc")]
fn json_text() {
    // this is just Text::wrap("hi") - no arena
    json! {
        let value = "hi";
    }
    assert_eq!(value.joined(), "hi");
}

#[test]
fn two_lines() {
    json! {
        let dict = {"key":"one\ntwo"};
    }
    assert_eq!(dict.to_string(), "{}\n\t<key>\n\t\tone\n\t\ttwo\n");
}

#[test]
#[cfg(feature = "alloc")]
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
    assert_eq!(text.joined(), "value");
}

#[test]
fn nested_dicts() {
    json! {
        let dict = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}};
    }
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
        let dict = {"a":{"b":["z"]}};
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
        let dict = {"a":[{"b":"z"}]};
    }
    let file = File::wrap(dict.cells);
    walk! {
        let (mut text, name, cell) = {dict}["a"]{0}<"b">.unwrap();
    }
    text = Text::wrap("c");
    cell.set(Entry{name,item:text.into()});
    assert_eq!(file.to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    json! {
        let dict = {"k":"v"};
    }
    let file = File::wrap(dict.cells);
    walk! {
        let (mut text, mut name, cell) = {dict}<"k">.unwrap();
    }
    name.before = Comment::some("b");
    text.epilog = Comment::some("c");
    cell.set(Entry{name,item:text.into()});
    assert_eq!(file.to_string(), "//b\nk=v\n#c\n");
}

#[test]
fn change_structure() {
    let key = "k";
    json! {
        let changing = {key:["v"]};
    }
    walk! {
        let (mut resolved, cell) = {changing}[key]<0>.unwrap();
    }
    resolved.epilog = Comment::some("b");
    json! {
        let patch = {"p":(resolved.into())};
    }
    cell.set(patch.into());
    assert_eq!(changing.to_string(), "{}\n\t[k]\n\t\t{}\n\t\t\tp=v\n\t\t\t#b\n")
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
