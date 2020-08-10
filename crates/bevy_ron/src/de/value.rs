use std::fmt;

use serde::{
    de::{Error, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    de,
    value::{Map, Number, Value},
};

impl std::str::FromStr for Value {
    type Err = de::Error;

    /// Creates a value from a string reference.
    fn from_str(s: &str) -> de::Result<Self> {
        let mut de = super::Deserializer::from_str(s)?;

        let val = Value::deserialize(&mut de)?;
        de.end()?;

        Ok(val)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a RON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Char(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(String::from_utf8(v).map_err(|e| Error::custom(format!("{}", e)))?)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Option(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Value::Option(Some(Box::new(
            deserializer.deserialize_any(ValueVisitor)?,
        ))))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Unit)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        if let Some(cap) = seq.size_hint() {
            vec.reserve_exact(cap);
        }

        while let Some(x) = seq.next_element()? {
            vec.push(x);
        }

        Ok(Value::Seq(vec))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut res: Map = Map::new();

        while let Some(entry) = map.next_entry()? {
            res.insert(entry.0, entry.1);
        }

        Ok(Value::Map(res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn eval(s: &str) -> Value {
        s.parse().expect("Failed to parse")
    }

    #[test]
    fn test_none() {
        assert_eq!(eval("None"), Value::Option(None));
    }

    #[test]
    fn test_some() {
        assert_eq!(eval("Some(())"), Value::Option(Some(Box::new(Value::Unit))));
        assert_eq!(
            eval("Some  (  () )"),
            Value::Option(Some(Box::new(Value::Unit)))
        );
    }

    #[test]
    fn test_tuples_basic() {
        assert_eq!(
            eval("(3, 4.0, 5.0)"),
            Value::Seq(vec![
                Value::Number(Number::new(3)),
                Value::Number(Number::new(4.0)),
                Value::Number(Number::new(5.0)),
            ],),
        );
    }

    #[test]
    fn test_tuples_ident() {
        assert_eq!(
            eval("(true, 3, 4, 5.0)"),
            Value::Seq(vec![
                Value::Bool(true),
                Value::Number(Number::new(3)),
                Value::Number(Number::new(4)),
                Value::Number(Number::new(5.0)),
            ]),
        );
    }

    #[test]
    fn test_tuples_error() {
        use crate::de::{Error, ErrorCode, Position};

        assert_eq!(
            Value::from_str("Foo:").unwrap_err(),
            Error {
                code: ErrorCode::TrailingCharacters,
                position: Position { col: 4, line: 1 }
            },
        );
    }

    #[test]
    fn test_floats() {
        assert_eq!(
            eval("(inf, -inf, NaN)"),
            Value::Seq(vec![
                Value::Number(Number::new(std::f64::INFINITY)),
                Value::Number(Number::new(std::f64::NEG_INFINITY)),
                Value::Number(Number::new(std::f64::NAN)),
            ]),
        );
    }

    #[test]
    fn test_complex() {
        assert_eq!(
            eval(
                "Some([
    Room ( width: 20, height: 5, name: \"The Room\" ),

    (
        width: 10.0,
        height: 10.0,
        name: \"Another room\",
        enemy_levels: {
            \"Enemy1\": 3,
            \"Enemy2\": 5,
            \"Enemy3\": 7,
        },
    ),
])"
            ),
            Value::Option(Some(Box::new(Value::Seq(vec![
                Value::Map(
                    vec![
                        (
                            Value::String("width".to_owned()),
                            Value::Number(Number::new(20)),
                        ),
                        (
                            Value::String("height".to_owned()),
                            Value::Number(Number::new(5)),
                        ),
                        (
                            Value::String("name".to_owned()),
                            Value::String("The Room".to_owned()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
                Value::Map(
                    vec![
                        (
                            Value::String("width".to_owned()),
                            Value::Number(Number::new(10.0)),
                        ),
                        (
                            Value::String("height".to_owned()),
                            Value::Number(Number::new(10.0)),
                        ),
                        (
                            Value::String("name".to_owned()),
                            Value::String("Another room".to_owned()),
                        ),
                        (
                            Value::String("enemy_levels".to_owned()),
                            Value::Map(
                                vec![
                                    (
                                        Value::String("Enemy1".to_owned()),
                                        Value::Number(Number::new(3)),
                                    ),
                                    (
                                        Value::String("Enemy2".to_owned()),
                                        Value::Number(Number::new(5)),
                                    ),
                                    (
                                        Value::String("Enemy3".to_owned()),
                                        Value::Number(Number::new(7)),
                                    ),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            ]))))
        );
    }
}
