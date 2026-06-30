use bumpalo::Bump;
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize, de::DeserializeSeed as _};
use std::collections::BTreeMap;
use std::fmt::Debug;
use tindalwic::bumpalo::Arena;
use tindalwic::parse::Parse as _;
use tindalwic::{File, json};
use tindalwic_serde::Neutered;
use tindalwic_serde::de::from_tindalwic;
use tindalwic_serde::ser::to_tindalwic;

#[test]
fn deserialize_file_from_json() {
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);

    let mut file = serde_json::Deserializer::from_str(r#"{ "key":"one\ntwo" }"#);
    let file: File = Neutered::seed(&mut arena).deserialize(&mut file).unwrap();

    json! {
        let entries = {"key":"one\ntwo"}.unwrap();
    }
    assert_eq!(file.cells, entries);
}

struct Check(bumpalo::Bump);
impl Check {
    fn run<'de, T>(&'de self, value: &T) -> T
    where
        T: Debug + PartialEq + Serialize + Deserialize<'de>,
    {
        let mut arena = tindalwic::bumpalo::Arena::new(&self.0);
        println!("# {value:?}");
        let mut data = BTreeMap::new();
        data.insert("data", &value);
        let string = to_tindalwic(arena.builder(), &data).unwrap();
        let string = arena.builder().intern(&string).unwrap();
        println!("## encoded\n{string}");
        let mut file: BTreeMap<&str, T> = from_tindalwic(&mut arena, &string).unwrap();
        let data: T = file.remove("data").unwrap();
        assert!(file.is_empty());
        data
    }
    fn check<'de, T>(&'de self, value: T)
    where
        T: Debug + PartialEq + Serialize + Deserialize<'de>,
    {
        assert_eq!(value, self.run(&value))
    }
    fn check_f32<'de>(&'de self, value: f32) {
        if value.is_nan() {
            assert!(self.run(&value).is_nan())
        } else {
            assert_eq!(value, self.run(&value))
        }
    }
    fn check_f64<'de>(&'de self, value: f64) {
        if value.is_nan() {
            assert!(self.run(&value).is_nan())
        } else {
            assert_eq!(value, self.run(&value))
        }
    }
}
#[fixture]
fn bump() -> Check {
    Check(bumpalo::Bump::new())
}

#[rstest]
fn boolean(bump: Check, #[values(false, true)] value: bool) {
    bump.check(value);
}
#[rstest]
fn signed_1_byte(bump: Check, #[values(i8::MIN, 0, i8::MAX)] value: i8) {
    bump.check(value);
}
#[rstest]
fn signed_2_bytes(bump: Check, #[values(i16::MIN, 0, i16::MAX)] value: i16) {
    bump.check(value);
}
#[rstest]
fn signed_4_bytes(bump: Check, #[values(i32::MIN, 0, i32::MAX)] value: i32) {
    bump.check(value);
}
#[rstest]
fn signed_8_bytes(bump: Check, #[values(i64::MIN, 0, i64::MAX)] value: i64) {
    bump.check(value);
}
#[rstest]
fn signed_16_bytes(bump: Check, #[values(i128::MIN, 0, i128::MAX)] value: i128) {
    bump.check(value);
}
#[rstest]
fn unsigned_1_byte(bump: Check, #[values(u8::MIN, u8::MAX)] value: u8) {
    bump.check(value);
}
#[rstest]
fn unsigned_2_bytes(bump: Check, #[values(u16::MIN, u16::MAX)] value: u16) {
    bump.check(value);
}
#[rstest]
fn unsigned_4_bytes(bump: Check, #[values(u32::MIN, u32::MAX)] value: u32) {
    bump.check(value);
}
#[rstest]
fn unsigned_8_bytes(bump: Check, #[values(u64::MIN, u64::MAX)] value: u64) {
    bump.check(value);
}
#[rstest]
fn unsigned_16_bytes(bump: Check, #[values(u128::MIN, u128::MAX)] value: u128) {
    bump.check(value);
}
#[rstest]
fn float_4_bytes(
    bump: Check,
    #[values(
            f32::NEG_INFINITY,
            f32::MIN,
            -0.0,
            0.0,
            f32::MIN_POSITIVE,
            f32::EPSILON,
            f32::MAX,
            f32::INFINITY,
            f32::NAN
        )]
    value: f32,
) {
    bump.check_f32(value);
}
#[rstest]
fn float_8_bytes(
    bump: Check,
    #[values(
            f64::NEG_INFINITY,
            f64::MIN,
            -0.0,
            0.0,
            f64::MIN_POSITIVE,
            f64::EPSILON,
            f64::MAX,
            f64::INFINITY,
            f64::NAN
        )]
    value: f64,
) {
    bump.check_f64(value);
}
#[rstest]
fn character(bump: Check, #[values(char::MIN, char::MAX)] value: char) {
    bump.check(value);
}
#[rstest]
fn string(bump: Check) {
    bump.check(String::from(""));
    bump.check(String::from("hello"));
    bump.check("");
    bump.check("world");
}
#[rstest]
fn bytes(bump: Check) {
    use bytes::Bytes;
    bump.check(Bytes::from_static(&[]));
    bump.check(Bytes::from_static(b"hello"));
    bump.check(Bytes::from_static(&[u8::MIN, u8::MAX]));
}
#[rstest]
fn option_and_unit(bump: Check) {
    bump.check(());
    bump.check(Option::<u8>::None);
    bump.check(Some(u8::MAX));
    bump.check(Some(()));
    bump.check(Option::<Option<u8>>::Some(None));
}
#[rstest]
fn tuple(bump: Check) {
    bump.check((false, true));
    bump.check([0, 1, 2, 3, 4, 5]);
}
#[rstest]
fn enums(bump: Check) {
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    enum Enum {
        Unit,
        Newtype(bool),
        Tuple(bool, bool),
        Struct { one: bool, two: bool },
    }
    bump.check(Enum::Unit);
    bump.check(Enum::Newtype(false));
    bump.check(Enum::Tuple(false, true));
    bump.check(Enum::Struct {
        one: false,
        two: true,
    });
}
#[rstest]
fn structs(bump: Check) {
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Unit;
    bump.check(Unit);
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Newtype(bool);
    bump.check(Newtype(false));
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Tuple(bool, bool);
    bump.check(Tuple(false, true));
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Struct {
        one: bool,
        two: bool,
    }
    bump.check(Struct {
        one: false,
        two: true,
    });
}
#[rstest]
fn seq(bump: Check) {
    bump.check(Vec::<bool>::new());
    bump.check(vec!['a', 'b', 'c', 'd', 'e']);
}
#[rstest]
fn map(bump: Check) {
    bump.check(BTreeMap::<String, char>::new());
    let mut map = BTreeMap::<String, char>::new();
    map.insert("zero".into(), '0');
    map.insert("one".into(), '1');
    map.insert("two".into(), '2');
    map.insert("three".into(), '3');
    bump.check(map);
}
