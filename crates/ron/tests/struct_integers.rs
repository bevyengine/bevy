use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct S {
    a: i8,
    b: i16,
    c: i32,
    d: i64,
    e: i128,
    f: u8,
    g: u16,
    h: u32,
    i: u64,
    j: u128,
}

#[test]
fn roundtrip() {
    let s = S {
        a: std::i8::MIN,
        b: std::i16::MIN,
        c: std::i32::MIN,
        d: std::i64::MIN,
        e: std::i128::MIN,
        f: std::u8::MAX,
        g: std::u16::MAX,
        h: std::u32::MAX,
        i: std::u64::MAX,
        j: std::u128::MAX,
    };
    let serialized = ron::ser::to_string(&s).unwrap();
    dbg!(&serialized);
    let deserialized = ron::de::from_str(&serialized).unwrap();
    assert_eq!(s, deserialized,);
}
