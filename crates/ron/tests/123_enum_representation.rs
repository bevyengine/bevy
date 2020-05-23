use ron::{de::from_str, ser::to_string};
use serde::{Deserialize, Serialize};
use std::{cmp::PartialEq, fmt::Debug};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Inner {
    Foo,
    Bar,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum EnumStructExternally {
    VariantA { foo: u32, bar: u32, different: u32 },
    VariantB { foo: u32, bar: u32 },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum EnumStructInternally {
    VariantA { foo: u32, bar: u32, different: u32 },
    VariantB { foo: u32, bar: u32 },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
enum EnumStructAdjacently {
    VariantA {
        foo: u32,
        bar: u32,
        different: Inner,
    },
    VariantB {
        foo: u32,
        bar: u32,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum EnumStructUntagged {
    VariantA { foo: u32, bar: u32, different: u32 },
    VariantB { foo: u32, bar: u32 },
}

fn test_ser<T: Serialize>(value: &T, expected: &str) {
    let actual = to_string(value).expect("Failed to serialize");
    assert_eq!(actual, expected);
}

fn test_de<T>(s: &str, expected: T)
where
    T: for<'a> Deserialize<'a> + Debug + PartialEq,
{
    let actual: Result<T, _> = from_str(s);
    assert_eq!(actual, Ok(expected));
}

fn test_roundtrip<T>(value: T)
where
    T: Serialize + for<'a> Deserialize<'a> + Debug + PartialEq,
{
    let s = to_string(&value).expect("Failed to serialize");
    let actual: Result<T, _> = from_str(&s);
    assert_eq!(actual, Ok(value));
}

#[test]
fn test_externally_a_ser() {
    let v = EnumStructExternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    let e = "VariantA(foo:1,bar:2,different:3)";
    test_ser(&v, e);
}

#[test]
fn test_externally_b_ser() {
    let v = EnumStructExternally::VariantB { foo: 1, bar: 2 };
    let e = "VariantB(foo:1,bar:2)";
    test_ser(&v, e);
}

#[test]
fn test_internally_a_ser() {
    let v = EnumStructInternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    let e = "(type:\"VariantA\",foo:1,bar:2,different:3)";
    test_ser(&v, e);
}

#[test]
fn test_internally_b_ser() {
    let v = EnumStructInternally::VariantB { foo: 1, bar: 2 };
    let e = "(type:\"VariantB\",foo:1,bar:2)";
    test_ser(&v, e);
}

#[test]
fn test_adjacently_a_ser() {
    let v = EnumStructAdjacently::VariantA {
        foo: 1,
        bar: 2,
        different: Inner::Foo,
    };
    let e = "(type:\"VariantA\",content:(foo:1,bar:2,different:Foo))";
    test_ser(&v, e);
}

#[test]
fn test_adjacently_b_ser() {
    let v = EnumStructAdjacently::VariantB { foo: 1, bar: 2 };
    let e = "(type:\"VariantB\",content:(foo:1,bar:2))";
    test_ser(&v, e);
}

#[test]
fn test_untagged_a_ser() {
    let v = EnumStructUntagged::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    let e = "(foo:1,bar:2,different:3)";
    test_ser(&v, e);
}

#[test]
fn test_untagged_b_ser() {
    let v = EnumStructUntagged::VariantB { foo: 1, bar: 2 };
    let e = "(foo:1,bar:2)";
    test_ser(&v, e);
}

#[test]
fn test_externally_a_de() {
    let s = "VariantA(foo:1,bar:2,different:3)";
    let e = EnumStructExternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_de(s, e);
}

#[test]
fn test_externally_b_de() {
    let s = "VariantB(foo:1,bar:2)";
    let e = EnumStructExternally::VariantB { foo: 1, bar: 2 };
    test_de(s, e);
}

#[test]
fn test_internally_a_de() {
    let s = "(type:\"VariantA\",foo:1,bar:2,different:3)";
    let e = EnumStructInternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_de(s, e);
}

#[test]
fn test_internally_b_de() {
    let s = "(type:\"VariantB\",foo:1,bar:2)";
    let e = EnumStructInternally::VariantB { foo: 1, bar: 2 };
    test_de(s, e);
}

#[test]
fn test_adjacently_a_de() {
    let s = "(type:\"VariantA\",content:(foo:1,bar:2,different:Foo))";
    let e = EnumStructAdjacently::VariantA {
        foo: 1,
        bar: 2,
        different: Inner::Foo,
    };
    test_de(s, e);
}

#[test]
fn test_adjacently_b_de() {
    let s = "(type:\"VariantB\",content:(foo:1,bar:2))";
    let e = EnumStructAdjacently::VariantB { foo: 1, bar: 2 };
    test_de(s, e);
}

#[test]
fn test_untagged_a_de() {
    let s = "(foo:1,bar:2,different:3)";
    let e = EnumStructUntagged::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_de(s, e);
}

#[test]
fn test_untagged_b_de() {
    let s = "(foo:1,bar:2)";
    let e = EnumStructUntagged::VariantB { foo: 1, bar: 2 };
    test_de(s, e);
}

#[test]
fn test_externally_a_roundtrip() {
    let v = EnumStructExternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_roundtrip(v);
}

#[test]
fn test_externally_b_roundtrip() {
    let v = EnumStructExternally::VariantB { foo: 1, bar: 2 };
    test_roundtrip(v);
}

#[test]
fn test_internally_a_roundtrip() {
    let v = EnumStructInternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_roundtrip(v);
}

#[test]
fn test_internally_b_roundtrip() {
    let v = EnumStructInternally::VariantB { foo: 1, bar: 2 };
    test_roundtrip(v);
}

#[test]
fn test_adjacently_a_roundtrip() {
    let v = EnumStructAdjacently::VariantA {
        foo: 1,
        bar: 2,
        different: Inner::Foo,
    };
    test_roundtrip(v);
}

#[test]
fn test_adjacently_b_roundtrip() {
    let v = EnumStructAdjacently::VariantB { foo: 1, bar: 2 };
    test_roundtrip(v);
}

#[test]
fn test_untagged_a_roundtrip() {
    let v = EnumStructUntagged::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_roundtrip(v);
}

#[test]
fn test_untagged_b_roundtrip() {
    let v = EnumStructUntagged::VariantB { foo: 1, bar: 2 };
    test_roundtrip(v);
}
