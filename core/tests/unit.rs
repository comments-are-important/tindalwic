#![allow(missing_docs)]

use std::collections::HashMap;
#[cfg(feature = "alloc")]
use tindalwic::alloc::from_literal;
use tindalwic::parse::Parse as _;
use tindalwic::{Dict, Entry, File, Item, List, Text, Value, arena, json, path};

#[test]
fn from_dict() {
    assert!(File::try_from_dict_without_epilog(&Item::Text("nope".into())).is_none());
    assert!(File::try_from_dict_without_epilog(&Item::List(List::default())).is_none());
}

#[test]
#[cfg(feature = "alloc")]
fn three_blank_comments() {
    let mut entry = Entry::default();
    entry.name.comment = "".into();
    entry.item = Item::Dict(Dict::default());
    let entries = [core::cell::Cell::new(entry)];
    let file = File {
        hashbang: Some("".into()),
        prolog: "".into(),
        entries: &entries,
        trailing: 1,
    };
    let encoded = file.to_string();
    let expect = "
        #!
        //
        ///
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
        //E
    ";
    let content = from_literal(spaces);
    assert_eq!("[K]\n\tV\n//E", content);
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
    let expected = "
        <key>
            one
            two
    ";
    assert_eq!(File::from(entries).to_string(), from_literal(expected));
}

#[test]
fn multi_line_key() {
    arena! {
        let mut arena = <4dict,1list>;
    }
    let data = "@\n\tone\n\ttwo\n<>\n\tv\n@\n\tl\n\t\n[]\n@\n\td\n\t\n{}";
    let file = arena.panic_first_error(data);
    assert_eq!(file.to_string(), data);
    let report = &mut |err| {
        print!("{err}");
        tindalwic::parse::Reported::Continue
    };
    assert!(arena.report_errors("@", report).is_none());
    assert!(arena.report_errors("@k", report).is_none());
    assert!(arena.report_errors("@\n\tk", report).is_none());
    assert!(arena.report_errors("@\n\tk\n", report).is_none());
    assert!(arena.report_errors("@\n\tk\n<", report).is_none());
    assert!(arena.report_errors("@\n\tk\n<>", report).is_some());
    assert!(arena.report_errors("@\n\tk\n<x>", report).is_none());
}

#[test]
#[cfg(feature = "bumpalo")]
fn walk_error() {
    let bump = bumpalo::Bump::new();
    let mut arena = tindalwic::bumpalo::Arena::new(&bump);
    let file = arena.panic_first_error("[data]\n\tzero\n\t{}\n\t\tk=v");
    path!({"data"}List).walk_file(&file).unwrap();
    path!({"data"}[0]List).walk_file(&file).unwrap_err();
    path!({"data"}[0]Text).walk_file(&file).unwrap();
    path!({"data"}[1]{"x"}Text).walk_file(&file).unwrap_err();
    path!({"data"}[1]{"k"}Text).walk_file(&file).unwrap();
    assert_eq!(
        path!({"data"}[7]Text)
            .walk_file(&file)
            .unwrap_err()
            .to_string(),
        "walk ({data}[7]): index out of bounds"
    );
}
#[test]
fn nested_lists() {
    json! {
        let items = [[[["value"]]]].unwrap();
    }
    let mut array = Entry::array::<1>();
    array[0].get_mut().item = Item::List(items.into());
    let file = File {
        entries: &array[..],
        ..Default::default()
    };
    assert_eq!(
        file.to_string(),
        "[]\n\t[]\n\t\t[]\n\t\t\t[]\n\t\t\t\tvalue"
    );
    let cell = path!({""}[0][0][0][0]Text).walk_file(&file).unwrap();
    let Item::Text(Text { value, .. }) = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["value"]);
}

#[test]
fn nested_dicts() {
    json! {
        let entries = {"1":"one","2":["two"],"a":{"b":{"c":{"d":{"k":"v"}}}}}.unwrap();
    }
    let mut keys = Vec::new();
    for entry in entries {
        let entry = entry.get();
        keys.push(entry.name.key.lines().next().unwrap_or(""));
    }
    assert_eq!(keys, vec!["1", "2", "a"]);
    assert_eq!(
        File::from(entries).to_string(),
        "1=one\n[2]\n\ttwo\n{a}\n\t{b}\n\t\t{c}\n\t\t\t{d}\n\t\t\t\tk=v"
    );
    let cell = path!({"a"}{"b"}{"c"}{"d"}{"k"}Text)
        .walk_item(entries.into())
        .unwrap();
    let Item::Text(Text { value, .. }) = cell.get().item else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    assert_eq!(Vec::from_iter(value.lines()), vec!["v"]);
}

#[test]
fn change_in_list() {
    json! {
        let entries = {"a":{"b":["v"]}}.unwrap();
    }
    let cell = path!({"a"}{"b"}[0]Text).walk_item(entries.into()).unwrap();
    let Item::Text(mut text) = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    text.epilog = Some("c".into());
    cell.set(text.into());
    assert_eq!(
        File::from(entries).to_string(),
        "{a}\n\t[b]\n\t\tv\n\t\t//c"
    );
}

#[test]
fn change_in_dict() {
    json! {
        let entries = {"a":[{"b":"z"}]}.unwrap();
    }
    let cell = path!({"a"}[0]{"b"}Text).walk_item(entries.into()).unwrap();
    let mut entry = cell.get();
    entry.item = Item::Text("c".into());
    cell.set(entry);
    assert_eq!(File::from(entries).to_string(), "[a]\n\t{}\n\t\tb=c");
}

#[test]
fn inject_comments() {
    json! {
        let entries = {"k":"v"}.unwrap();
    }
    let cell = path!({"k"}Text).walk_item(entries.into()).unwrap();
    let mut entry = cell.get();
    let Item::Text(mut text) = entry.item else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    text.epilog = Some("c".into());
    entry.name.comment = "b".into();
    entry.item = text.into();
    cell.set(entry);
    assert_eq!(File::from(entries).to_string(), "///b\nk=v\n//c");
}

#[test]
fn change_structure() {
    let key = "k";
    json! {
        let entries = {key:["v"]}.unwrap();
    }
    let cell = path!({key}[0]Text).walk_item(entries.into()).unwrap();
    let Item::Text(mut text) = cell.get() else {
        unreachable!("this destructuring always succeeds because path walk did");
    };
    let b = String::from("b");
    text.epilog = Some((&b[..]).into());
    json! {
        let patch = {"p":(text)}.unwrap();
    }
    cell.set(Item::Dict(patch.into()));
    assert_eq!(
        File::from(entries).to_string(),
        "[k]\n\t{}\n\t\tp=v\n\t\t//b"
    )
}

#[test]
fn hash_map() {
    json! {
        let entries = {"":"0","a":"1","b":"2","c\nc":"3"}.unwrap();
    }
    let mut map = HashMap::new();
    for entry in entries {
        let Entry { name, item, .. } = entry.get();
        map.insert(name.key, item);
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
    assert!(file.prolog.value.is_none());
    assert!(file.entries.is_empty());
}

#[test]
fn key_eq_value() {
    arena! {
        let mut arena = <1dict>;
    }
    let file = arena.panic_first_error("k=v");
    assert!(arena.completed().is_some());
    assert!(file.hashbang.is_none());
    assert!(file.prolog.value.is_none());
    assert_eq!(file.entries.len(), 1);
    let key: Value<'_> = "k".into();
    let Some(position) = key.find_linearly_in(file.entries) else {
        panic!("no 'k' key found");
    };
    let Item::Text(Text { value, .. }) = file.entries[position].get().item else {
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
    assert_eq!(file.entries.len(), 1);
    let key: Value<'_> = "k".into();
    let Some(position) = key.find_linearly_in(file.entries) else {
        panic!("no 'k' key found");
    };
    let Item::List(List { items: cells, .. }) = file.entries[position].get().item else {
        panic!("not list?");
    };
    assert_eq!(cells.len(), 3);
    let Item::Text(Text { value: one, .. }) = cells[0].get() else {
        panic!("not text?");
    };
    assert_lines_eq!(one, "1");
    let Item::Text(Text { value: two, .. }) = cells[1].get() else {
        panic!("not text?");
    };
    assert_lines_eq!(two, "2");
    let Item::Text(Text { value: three, .. }) = cells[2].get() else {
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

    let Item::Text(Text { value, .. }) = Path::<true>::new(&[
        Branch::Entry("z".into()),
        Branch::Entry("k".into()),
        Branch::Text,
    ])
    .walk_file(&file)
    .unwrap()
    .get()
    .item
    else {
        panic!("not text?")
    };
    assert_lines_eq!(value, "v");
}

#[cfg(feature = "bumpalo")]
mod parse_err {
    use bumpalo::Bump;
    use std::cell::Cell;
    //use tindalwic::alloc::from_literal;
    use tindalwic::bumpalo::Arena as HeapArena;
    use tindalwic::capped::Arena as StackArena;
    use tindalwic::parse::{Parse, ParseError};
    use tindalwic::{Entries, Entry, Item, Items, path};
    const NO_ITEMS: Items = &[];
    const NO_ENTRIES: Entries = &[];

    #[test]
    fn intern_needs_bumpalo() {
        let mut arena = StackArena::wrap(NO_ITEMS, NO_ENTRIES);
        assert_eq!(arena.builder().intern(""), Err("intern not supported"));
    }

    #[test]
    fn not_enough_room() {
        let mut arena = StackArena::wrap(NO_ITEMS, NO_ENTRIES);
        assert_eq!(
            arena.builder().push_item(Item::default()),
            Err("no room for item")
        );
        let blank = Entry::default();
        assert_eq!(arena.builder().push_entry(blank), Err("no room for entry"));
        assert!(arena.completed().is_some());
        assert_eq!(0, arena.item_slots());
        assert_eq!(0, arena.entry_slots());
    }

    #[test]
    #[should_panic]
    fn panic_first_error() {
        let mut arena = StackArena::wrap(NO_ITEMS, NO_ENTRIES);
        arena.panic_first_error("invalid");
    }
    #[test]
    fn intern() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        assert_eq!(Ok("x"), arena.builder().intern("x"));
    }
    #[test]
    fn format_errors() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        let content = "\tx\n\tx\nk=v";
        let errors = arena.format_errors("", content, 1).unwrap_err();
        assert_eq!(errors, ":1: error: (thru line 2) excess indentation\n");
        let errors = arena.format_errors("", content, usize::MAX).unwrap_err();
        assert_eq!(errors, ":1: error: (thru line 2) excess indentation\n");
        // decided to become more flexible about empties, so these aren't errors any more...
        // let content = "\n\n\tx\nk=v";
        // let errors = arena.format_errors("", content, 1).unwrap_err();
        // assert_eq!(errors, ":1: error: consecutive empty lines\n");
        // let errors = arena.format_errors("", content, usize::MAX).unwrap_err();
        // assert_eq!(
        //     errors,
        //     ":1: error: consecutive empty lines\n:2: error: (thru line 3) excess indentation\n"
        // );
    }
    #[test]
    fn excess_indent() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        let content = "\tx\n\tx\nk=v";
        let errors = arena
            .collect_errors(&content, usize::MAX)
            .expect_err("invalid");
        assert_eq!(errors, vec!(ParseError::new(1, 3, "excess indentation")));
    }
    #[test]
    fn consecutive_empty() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        let content = "\n\n\nk=v\n\n";
        let file = arena.collect_errors(&content, usize::MAX).unwrap();
        assert_eq!(file.entries.len(), 1);
        assert_eq!(file.trailing, 2);
        let entry = file.entries[0].get();
        assert_eq!(entry.name.comment.gap, 3);
    }
    #[test]
    fn list_shortcut() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        let content = "[data]\n\t\n";
        let file = arena.collect_errors(&content, usize::MAX).unwrap();
        let cell = path!({"data"}List).walk_file(&file).unwrap();
        let list = &[Cell::new(Item::default())];
        assert_eq!(cell.get().item, Item::List((&list[..]).into()));
    }
    #[test]
    fn list_errors() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        let content = "[data]\n\t/\n\t//\n\t///\n\t<_\n\t[_\n\t{_\n\t<>\n\t[]\n\t{}";
        let errors = arena
            .collect_errors(&content, usize::MAX)
            .expect_err("invalid");
        assert_eq!(
            errors,
            vec!(
                ParseError::at(4, "stray comment"),
                ParseError::at(5, "malformed `<>` in list"),
                ParseError::at(6, "malformed `[]` in list"),
                ParseError::at(7, "malformed `{}` in list"),
            )
        );
    }
    // #[test]
    // fn dict_gap_error() {
    //     let mut arena = StackArena::wrap(NO_ITEMS, NO_ENTRIES);
    //     assert_eq!(
    //         arena.first_error("///"), seen as prolog, only worked with # and //
    //         Err(ParseError::at(2, "gap/comment but no key"))
    //     );
    // }
    #[test]
    fn dict_errors() {
        let bump = Bump::new();
        let mut arena = HeapArena::new(&bump);
        let content = "{data}\n\t/\n\t//\n\t<_\n\t[_\n\t{_\n\t<t>\n\t[l]\n\t{d}";
        let errors = arena
            .collect_errors(&content, usize::MAX)
            .expect_err("invalid");
        assert_eq!(
            errors,
            vec!(
                ParseError::at(2, "missing `=` in dict"),
                ParseError::at(3, "stray comment"),
                ParseError::at(4, "malformed `<key>` in dict"),
                ParseError::at(5, "malformed `[key]` in dict"),
                ParseError::at(6, "malformed `{key}` in dict"),
            )
        );
    }
}
