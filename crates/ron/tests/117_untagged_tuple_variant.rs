use std::borrow::Cow;

use ron::{de::from_str, ser::to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildSystem<'m> {
    version: Cow<'m, str>,
    flags: Vec<Flag<'m>>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Flag<'m> {
    Value(Cow<'m, str>),
    If(Cow<'m, str>, Vec<Cow<'m, str>>),
}

#[test]
fn test_ebkalderon_case() {
    let file = r#"BuildSystem(
    version: "1.0.0",
    flags: [
        "--enable-thing",
        "--enable-other-thing",
        If("some-conditional", ["--enable-third-thing"]),
    ]
)
"#;

    assert_eq!(
        from_str::<BuildSystem>(file).unwrap(),
        BuildSystem {
            version: "1.0.0".into(),
            flags: vec![
                Flag::Value("--enable-thing".into()),
                Flag::Value("--enable-other-thing".into()),
                Flag::If(
                    "some-conditional".into(),
                    vec!["--enable-third-thing".into()]
                )
            ]
        },
    );
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Foo {
    Bar(usize),
}

#[test]
fn test_vessd_case() {
    let foo_vec = vec![Foo::Bar(0); 5];
    let foo_str = to_string(&foo_vec).unwrap();
    assert_eq!(foo_str.as_str(), "[0,0,0,0,0]");
    assert_eq!(from_str::<Vec<Foo>>(&foo_str).unwrap(), foo_vec);
}
