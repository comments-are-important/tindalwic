use super::*;
use regex::{Regex, RegexBuilder};
use std::sync::LazyLock;

#[test]
fn path_display() {
    assert_eq!(path!("zero", [1], "two").to_string(), ".zero[1].two");
}

#[test]
fn resolve_list() {
    let inner = Value::Text(Text::from("hello"));
    let list = Value::List(List::from(vec![inner]));

    let resolved = path!([0]).text(&list).unwrap();
    assert_eq!(Indented::from(0,resolved).to_string(), "\thello\n");
}

#[test]
fn resolve_failure() {
    let inner = Value::Text(Text::from("hello"));
    let list = Value::List(List::from(vec![inner]));

    path!([5]).value(&list).unwrap_err();
}

fn visible(string: &str) -> String {
    static DEDENT: LazyLock<Regex> =
        LazyLock::new(|| RegexBuilder::new("^ *").multi_line(true).build().unwrap());
    DEDENT.replace_all(string, "").replace("╶─▸", "\t").replace("▁▁▎","\n")
}

struct Expect(String);
impl Expect {
    fn from(&self, indent: usize, parse: &'static str) -> &Self {
        let parse = visible(parse);
        let vec: Vec<&str> = Comment::parse_utf8(&parse, indent).lines().collect();
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

#[test]
fn zzz() {
    let mut hi = String::from("hi");
    let mut text = Text::from(&hi[..]);
    text.epilog = Comment::some("comment");
    let mut root = Value::List(List::from(vec![Value::Text(text)]));
    //hi.clear(); // won't compile
    let result = path!([0]).text_mut(&mut root).unwrap();
    result.epilog = Comment::some("changed");
    //assert_eq!(text.epilog.unwrap().gfm.to_string(), "hi");
    hi.clear();
}

#[test]
fn encode_uncommented_file() {
    assert_eq!(
        File::from(vec![Keyed::from(
            "one",
            Value::List(List::from(vec![Value::Text(Text::from("\n"))]))
        )])
        .to_string(),
        visible(
            "[one]
            ╶─▸<>
            ╶─▸╶─▸
            ╶─▸╶─▸
            "
        )
    );
}
#[test]
fn encode_fully_commented_file() {
    let mut file = File::from(vec![])
        .with_hashbang("/usr/bin/env -S app argument")
        .with_prolog(" this is the prolog for the file");
    file.push(Keyed {
        key: "one",
        gap: true,
        before: Comment::some(" about key one"),
        value: Value::Text(Text::from("1").with_epilog(" about value one")),
    });
    assert_eq!(
        file.to_string(),
        visible(
            "#!/usr/bin/env -S app argument
            # this is the prolog for the file

            // about key one
            <one>
            ╶─▸1
            # about value one
            "
        )
    )
}
