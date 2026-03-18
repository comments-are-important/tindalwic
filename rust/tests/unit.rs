use regex::{Regex, RegexBuilder};
use std::{string::ToString, sync::LazyLock};
use tindalwic::*;

#[test]
fn path_display() {
    assert_eq!(path!("zero", [1], "two").to_string(), ".zero[1].two");
}

#[test]
fn resolve_list() {
    let list = tindalwic!(["hello", { "k" : ["world"] }]);
    assert_eq!(path!([0]).text(&list).unwrap().to_string(), "hello\n");
    assert_eq!(
        path!([1], "k", [0]).text(&list).unwrap().to_string(),
        "world\n"
    );
}

#[test]
fn resolve_failure() {
    path!([5])
        .value(&tindalwic!(["hello", "world"]))
        .unwrap_err();
}

fn visible(string: &str) -> String {
    static DEDENT: LazyLock<Regex> =
        LazyLock::new(|| RegexBuilder::new("^ *").multi_line(true).build().unwrap());
    DEDENT
        .replace_all(string, "")
        .replace("╶─▸", "\t")
        .replace("▁▁▎", "\n")
}

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
fn encode_uncommented_file() {
    assert_eq!(
        tindalwic!("one":"\n").to_string(),
        visible(
            "[one]
            ╶─▸<>
            ╶─▸╶─▸
            ╶─▸╶─▸
            "
        )
    );
}
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
