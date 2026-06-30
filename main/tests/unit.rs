#![allow(missing_docs)]

use std::collections::HashMap;
#[cfg(feature = "alloc")]
use tindalwic::alloc::from_literal;
use tindalwic::parse::Parse as _;
use tindalwic::{Comment, Entry, File, Item, Value, arena, json, path};

#[test]
fn value_eq() {
    let value: Value<'_> = "ONE\nTWO\nTHREE".into();
    assert_eq!(value, Value::slice_prefix(2, "ONE\n\t\tTWO\n\t\tTHREE"));
    assert_eq!(
        3,
        Value::slice_prefix(1, "X\n\t").verbatim(1).unwrap().len()
    );
}

#[test]
fn from_dict() {
    assert!(File::try_from_dict_without_epilog(&Item::text("nope")).is_none());
    assert!(File::try_from_dict_without_epilog(&Item::list(&[])).is_none());
}

#[test]
fn hashbang_avoidance() {
    let mut file = File::default();
    file.prolog = Comment::some("!suspect");
    let encoded = file.to_string();
    assert_eq!(encoded, "#\n\t!suspect\n");
    arena! {
        let mut arena = <1dict>;
    }
    let parsed = arena.panic_first_error(&encoded);
    assert!(parsed.hashbang.is_none());
    assert_eq!(
        Vec::from_iter(parsed.prolog.unwrap().value.lines()),
        vec!["!suspect"]
    );
}

#[test]
#[cfg(feature = "alloc")]
fn three_blank_comments() {
    let entry = Entry {
        before: Comment::some(""),
        item: Item::dict(&[]),
        ..Default::default()
    };
    let entries = [core::cell::Cell::new(entry)];
    let file = File {
        hashbang: Comment::some(""),
        prolog: Comment::some(""),
        cells: &entries,
    };
    let encoded = file.to_string();
    let expect = "
        #!
        #
        //
        {}
    ";
    assert_eq!(encoded, from_literal(expect));
}
#[test]
#[cfg(feature = "alloc")]
fn text_stretch_bug() {
    let spaces = "
        [K]
            V
        #E
    ";
    let content = from_literal(spaces);
    assert_eq!("[K]\n\tV\n#E\n", content);
    arena! {
        let mut arena = <1dict,1list>;
    }
    let file = arena.panic_first_error(&content);
    assert_eq!(file.to_string(), content);
}

#[test]
fn two_lines() {
    json! {
        let entries = {"key":"one\ntwo"}.unwrap();
    }
    assert_eq!(
        File::try_from_dict_without_epilog(&Item::dict(entries))
            .unwrap()
            .to_string(),
        "<key>\n\tone\n\ttwo\n"
    );
}

#[test]
fn multi_line_key() {
    arena! {
        let mut arena = <2dict>;
    }
    let data = "@one\n\ttwo\n<>\n\tv\n";
    let file = arena.panic_first_error(data);
    assert_eq!(file.to_string(), data);
    let report = &mut |err| {
        print!("{err}");
        tindalwic::parse::Reported::Continue
    };
    assert!(arena.report_errors("@", report).is_none());
    assert!(arena.report_errors("@k", report).is_none());
    assert!(arena.report_errors("@k\n", report).is_none());
    assert!(arena.report_errors("@k\n<", report).is_none());
    assert!(arena.report_errors("@k\n<>", report).is_some());
    assert!(arena.report_errors("@k\n<x>", report).is_none());
}

#[test]
fn nested_lists() {
    json! {
        let items = [[[["value"]]]].unwrap();
    }
    let mut array = Entry::array::<1>();
    array[0].get_mut().item = Item::list(items);
    let file = File {
        cells: &array[..],
        ..Default::default()
    };
    assert_eq!(
        file.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue\n"
    );
    let cell = path!({""}[0][0][0][0]Text)
        .walk(file.embed_without_hashbang())
        .unwrap();
    let Item::Text { value, .. } = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["value"]);
}

#[test]
fn nested_dicts() {
    json! {
        let entries = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}}.unwrap();
    }
    let dict = Item::dict(entries);
    let mut keys = Vec::new();
    for entry in entries {
        let entry = entry.get();
        keys.push(entry.key.lines().next().unwrap_or(""));
    }
    assert_eq!(keys, vec!["1", "2", "a"]);
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v\n"
    );
    let cell = path!({"a"}{"b"}{"c"}{"d"}{"k"}Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get().item else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let entries = {"a":{"b":["v"]}}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"a"}{"b"}[0]Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let epilog = Comment::some("c");
    cell.set(Item::Text { value, epilog });
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "{a}\n\t[b]\n\t\tv\n\t\t#c\n"
    );
}

#[test]
fn change_in_dict() {
    json! {
        let entries = {"a":[{"b":"z"}]}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"a"}[0]{"b"}Text).walk(dict).unwrap();
    let mut entry = cell.get();
    entry.item = Item::text("c");
    cell.set(entry);
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "[a]\n\t{}\n\t\tb=c\n"
    );
}

#[test]
fn inject_comments() {
    json! {
        let entries = {"k":"v"}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({"k"}Text).walk(dict).unwrap();
    let mut entry = cell.get();
    let Item::Text { value, .. } = entry.item else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let epilog = Comment::some("c");
    entry.before = Comment::some("b");
    entry.item = Item::Text { value, epilog };
    cell.set(entry);
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "//b\nk=v\n#c\n"
    );
}

#[test]
fn change_structure() {
    let key = "k";
    json! {
        let entries = {key:["v"]}.unwrap();
    }
    let dict = Item::dict(entries);
    let cell = path!({key}[0]Text).walk(dict).unwrap();
    let Item::Text { value, .. } = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let b = String::from("b");
    let epilog = Comment::some(&b);
    json! {
        let patch = {"p":(Item::Text { value, epilog })}.unwrap();
    }
    cell.set(Item::dict(patch));
    assert_eq!(
        File::try_from_dict_without_epilog(&dict)
            .unwrap()
            .to_string(),
        "[k]\n\t{}\n\t\tp=v\n\t\t#b\n"
    )
}

#[test]
fn hash_map() {
    json! {
        let entries = {"":"0","a":"1","b":"2","c\nc":"3"}.unwrap();
    }
    let mut map = HashMap::new();
    for entry in entries {
        let Entry { key, item, .. } = entry.get();
        map.insert(key, item);
    }
    assert_eq!(map.len(), entries.len());
}

#[test]
#[cfg(feature = "bumpalo")]
fn parse_alloc() {
    let bump = bumpalo::Bump::new();
    let mut arena = tindalwic::bumpalo::Arena::new(&bump);
    let file = arena.panic_first_error("k=v\n");
    assert_eq!(file.to_string(), "k=v\n");
}
#[test]
#[cfg(feature = "bumpalo")]
fn invalid() {
    let bump = bumpalo::Bump::new();
    let mut arena = tindalwic::bumpalo::Arena::new(&bump);
    let Err(errors) = arena.collect_errors("nope", usize::MAX) else {
        panic!("got a file expected parse error")
    };
    assert_eq!(errors.len(), 1);
}

macro_rules! assert_lines_eq {
        // checking this gets repetitive without Vec
        ($value:ident, $($line:literal),*) => {
            let mut it = $value.lines();
            $(assert_eq!(it.next(), Some($line));)*
            assert_eq!(it.next(), None);
        };
    }

#[test]
fn empty() {
    arena! {
        let mut arena = <10dict,10list>;
    }
    let file = arena.panic_first_error("");
    assert!(!arena.completed().is_some());
    assert!(file.hashbang.is_none());
    assert!(file.prolog.is_none());
    assert!(file.cells.is_empty());
}

#[test]
fn key_eq_value() {
    arena! {
        let mut arena = <1dict>;
    }
    let file = arena.panic_first_error("k=v");
    assert!(arena.completed().is_some());
    assert!(file.hashbang.is_none());
    assert!(file.prolog.is_none());
    assert_eq!(file.cells.len(), 1);
    let key: Value<'_> = "k".into();
    let Some(position) = key.find_linearly_in(file.cells) else {
        panic!("no 'k' key found");
    };
    let Item::Text { value, .. } = file.cells[position].get().item else {
        panic!("not text?");
    };
    assert_lines_eq!(value, "v");
}
#[test]
fn sub_list() {
    arena! {
        let mut arena = <3list,1dict>;
    }
    let file = arena.panic_first_error("[k]\n\t1\n\t2\n\t3");
    assert!(arena.completed().is_some());
    assert_eq!(file.cells.len(), 1);
    let key: Value<'_> = "k".into();
    let Some(position) = key.find_linearly_in(file.cells) else {
        panic!("no 'k' key found");
    };
    let Item::List { cells, .. } = file.cells[position].get().item else {
        panic!("not list?");
    };
    assert_eq!(cells.len(), 3);
    let Item::Text { value: one, .. } = cells[0].get() else {
        panic!("not text?");
    };
    assert_lines_eq!(one, "1");
    let Item::Text { value: two, .. } = cells[1].get() else {
        panic!("not text?");
    };
    assert_lines_eq!(two, "2");
    let Item::Text { value: three, .. } = cells[2].get() else {
        panic!("not text?");
    };
    assert_lines_eq!(three, "3");
}
#[test]
fn sub_dict() {
    arena! {
        let mut arena = <2dict>;
    }
    let file = arena.panic_first_error("{z}\n\t<k>\n\t\tv");
    assert!(arena.completed().is_some());
    use tindalwic::walk::*;

    let Item::Text { value, .. } = Path::<true>::new(&[
        Branch::Entry("z".into()),
        Branch::Entry("k".into()),
        Branch::Text,
    ])
    .walk(file.embed_without_hashbang())
    .unwrap()
    .get()
    .item
    else {
        panic!("not text?")
    };
    assert_lines_eq!(value, "v");
}
