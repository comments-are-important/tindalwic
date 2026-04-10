//use regex::{Regex, RegexBuilder};
//use std::{string::ToString, sync::LazyLock};
use tindalwic::*;

#[test]
fn macro_failures() {
    trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
}

#[test]
fn empty_file() {
    json!(empty = {});
    assert!(empty.file.get().is_empty());
}

#[test]
fn one_text_value() {
    json!(simple = {"key":"value"});
    assert_eq!(simple.file.get().to_string(), "key=value\n");
}

#[test]
fn nested_lists() {
    json!(nested = {"key":[[[[["value"]]]]]});
    assert_eq!(
        nested.file.get().to_string(),
        "[key]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\t[]\n\t\t\t\t\tvalue\n"
    );
    let (text, _cell) = walk!(nested.file.get(), ["key"][0][0][0][0]<0>).unwrap();
    assert_eq!(text.lines().collect::<Vec<_>>().join("\n"), "value");
}

#[test]
fn nested_dicts() {
    json!(data = {"1":"one","2":"two","a":{"b":{"c":{"d":{"k":"v"}}}}});
    assert_eq!(
        data.file.get().to_string(),
        "1=one\n2=two\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    let (text, _cell) = walk!(data.file.get(), {"a"}{"b"}{"c"}{"d"}<"k">).unwrap();
    assert_eq!(Vec::from_iter(text.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json!(arena = {"a":{"b":["z"]}});
    let (_text, cell) = walk!(arena.file.get(), {"a"}["b"]<0>).unwrap();
    cell.set(Value::Text(Text::wrap("c")));
    assert_eq!(arena.file.get().to_string(), "{a}\n\t[b]\n\t\tc\n");
}

#[test]
fn change_in_dict() {
    json!(arena = {"a":[{"b":"z"}]});
    let (_text,cell) = walk!(arena.file.get(), ["a"]{0}<"b">).unwrap();
    let mut keyed = cell.get();
    keyed.value = Value::Text(Text::wrap("c"));
    cell.set(keyed);
    assert_eq!(arena.file.get().to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    json!(data = {"k":"v"});
    let (_text,cell) = walk!(data.file.get(), <"k">).unwrap();
    let mut keyed = cell.get();
    keyed.before = Comment::some("b");
    if let Value::Text(ref mut text) = keyed.value {
        text.epilog = Comment::some("c");
    }
    cell.set(keyed);
    assert_eq!(data.file.get().to_string(), "//b\nk=v\n#c\n");
}

// #[test]
// fn resolve_list() {
//     let list:Value<'_> = tindalwic!(["hello", { "k" : ["world"] }]);
//     assert_eq!(list.at(0).text().unwrap().to_string(), "hello\n");
//     assert_eq!(list.at(1).key("k").text().unwrap().to_string(), "world\n");
// }

// #[test]
// fn resolve_failure() {
//     path!([5])
//         .value(&tindalwic!(["hello", "world"]))
//         .unwrap_err();
// }

// fn visible(string: &str) -> String {
//     static DEDENT: LazyLock<Regex> =
//         LazyLock::new(|| RegexBuilder::new("^ *").multi_line(true).build().unwrap());
//     DEDENT
//         .replace_all(string, "")
//         .replace("╶─▸", "\t")
//         .replace("▁▁▎", "\n")
// }

/*
struct Expect(String);
impl Expect {
    fn from(&self, indent: usize, parse: &'static str) -> &Self {
        let parse = visible(parse);
        let encoded = Encoded::parse(&parse, indent);
        let vec: Vec<&str> = Comment { encoded }.lines().collect();
        assert_eq!(vec.join("\n"), self.0);
        self
    }
}

#[test]
fn parse_comments() {
    Expect(visible("c")).from(0, "c");

    Expect(visible("a▁▁▎b"))
        .from(0, "a▁▁▎╶─▸b▁▁▎...")
        .from(1, "a▁▁▎╶─▸╶─▸b▁▁▎╶─▸...")
        .from(2, "a▁▁▎╶─▸╶─▸╶─▸b▁▁▎╶─▸╶─▸...");

    Expect(visible("a▁▁▎╶─▸b"))
        .from(0, "a▁▁▎╶─▸╶─▸b▁▁▎...")
        .from(1, "a▁▁▎╶─▸╶─▸╶─▸b▁▁▎╶─▸...");
}
*/

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

//#[test]
// fn encode_uncommented_file() {
//     assert_eq!(
//         tindalwic!("one":"\n").to_string(),
//         visible(
//             "[one]
//             ╶─▸<>
//             ╶─▸╶─▸
//             ╶─▸╶─▸
//             "
//         )
//     );
// }
// #[test]
// fn encode_fully_commented_file() {
//     let file = File {
//         hashbang: Some("/usr/bin/env -S app argument".into()),
//         prolog: Some(" this is the prolog for the file".into()),
//         vec: vec![
//             Keyed {
//                 gap: true,
//                 before: Some(" about key one".into()),
//                 key: "one",
//                 value: tindalwic!(<> "1" ; # epilog=" about value one"),
//             },
//             Keyed {
//                 gap: true,
//                 before: Some(" about key two".into()),
//                 key: "two",
//                 value: tindalwic!([]
//                     # " about list two";
//                     <> "2"
//                     //epilog=" after list two",
//                 ),
//             },
//             Keyed {
//                 gap: true,
//                 before: Some(" about key three".into()),
//                 key: "three",
//                 value: Value::Dict(Dict {
//                     prolog: Some(" about dict three".into()),
//                     vec: vec![],
//                     epilog: Some(" after dict three".into()),
//                 }),
//             },
//         ],
//     };
//     assert_eq!(
//         file.to_string(),
//         visible(
//             "#!/usr/bin/env -S app argument
//             # this is the prolog for the file

//             // about key one
//             one=1
//             # about value one

//             // about key two
//             [two]
//             ╶─▸# about list two
//             ╶─▸2
//             # after list two

//             // about key three
//             {three}
//             ╶─▸# about dict three
//             # after dict three
//             "
//         )
//     )
// }
