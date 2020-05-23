use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Borrowed<'a> {
    value: &'a str,
}

const BORROWED: &str = "Borrowed(value: \"test\")";

#[test]
fn borrowed_str() {
    assert_eq!(
        ron::de::from_str(BORROWED).ok(),
        Some(Borrowed { value: "test" })
    );
}
