//use regex::{Regex, RegexBuilder};
//use std::{string::ToString, sync::LazyLock};
use tindalwic::*;

#[test]
fn macro_failures() {
    trybuild::TestCases::new().compile_fail("tests/macro_err/*.rs");
}

#[test]
fn empty_file() {
    tindalwic_json!(empty = {});
    assert!(empty.file.get().is_empty());
}

#[test]
fn one_text_value() {
    tindalwic_json!(simple = {"key":"value"});
    assert_eq!(simple.file.get().to_string(), "key=value\n");
}

#[test]
fn nested_lists() {
    tindalwic_json!(nested = {"key":[[[[["value"]]]]]});
    assert_eq!(
        nested.file.get().to_string(),
        "[key]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\t[]\n\t\t\t\t\tvalue\n"
    );
}

#[test]
fn nested_dicts() {
    tindalwic_json!(data = {"1":"one","2":"two","a":{"b":{"c":{"d":{"k":"v"}}}}});
    //println!("{:?}",data.file.get());
    assert_eq!(
        data.file.get().to_string(),
        "1=one\n2=two\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
}

#[test]
fn change_in_list() {
    tindalwic_json!(arena = {"a":{"b":["z"]}});
    tindalwic_walk!(arena{"a"}["b"]<0> |c,_t|c = Value::Text(Text::wrap("c"))).unwrap();
    assert_eq!(arena.file.get().to_string(), "{a}\n\t[b]\n\t\tc\n");
}

#[test]
fn change_in_dict() {
    tindalwic_json!(arena = {"a":[{"b":"z"}]});
    tindalwic_walk!(arena["a"]{0}<"b"> |c,_t|c.value = Value::Text(Text::wrap("c"))).unwrap();
    assert_eq!(arena.file.get().to_string(), "[a]\n\t{}\n\t\tb=c\n");
}

#[test]
fn inject_comments() {
    tindalwic_json!(data = {"k":"v"});
    tindalwic_walk!(data<"k">|c,t|t.epilog = Comment::some("c");c.before=Comment::some("b")).unwrap();
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
