use bevy_utils::hashbrown::HashMap;
use serde::de::{self, Deserializer, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Deserialize, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Deserialize, Default)]
struct SetGold {
    gold: usize,
}

#[derive(Debug, Deserialize, Default)]
struct ComplexMove {
    target: Vec3,
    name: String,
}

struct CliDeserializer<'a> {
    input: &'a str,
}

impl<'a> CliDeserializer<'a> {
    fn from_str(input: &'a str) -> Result<Self, de::value::Error> {
        Ok(Self { input })
    }
}

impl<'de> Deserializer<'de> for CliDeserializer<'de> {
    type Error = de::value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        unimplemented!("deserialize_any not implemented")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if input == "true" {
            visitor.visit_bool(true)
        } else if input == "false" {
            visitor.visit_bool(false)
        } else {
            Err(de::Error::custom(format!("Invalid boolean value: {}", input)))
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = i8::from_str(input) {
            visitor.visit_i8(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = i16::from_str(input) {
            visitor.visit_i16(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = i32::from_str(input) {
            visitor.visit_i32(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = i64::from_str(input) {
            visitor.visit_i64(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>, {
        let input = self.input;
        if let Ok(i) = i128::from_str(input) {
            visitor.visit_i128(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = u8::from_str(input) {
            visitor.visit_u8(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = u16::from_str(input) {
            visitor.visit_u16(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = u32::from_str(input) {
            visitor.visit_u32(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = u64::from_str(input) {
            visitor.visit_u64(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>, {
        let input = self.input;
        if let Ok(i) = u128::from_str(input) {
            visitor.visit_u128(i)
        } else {
            Err(de::Error::custom(format!("Invalid integer value: {}", input)))
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = f32::from_str(input) {
            visitor.visit_f32(i)
        } else {
            Err(de::Error::custom(format!("Invalid float value: {}", input)))
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(i) = f64::from_str(input) {
            visitor.visit_f64(i)
        } else {
            Err(de::Error::custom(format!("Invalid float value: {}", input)))
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;
        if let Ok(c) = char::from_str(input) {
            visitor.visit_char(c)
        } else {
            Err(de::Error::custom(format!("Invalid char value: {}", input)))
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        visitor.visit_borrowed_str(self.input)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        self.deserialize_str(visitor)
    }


    fn deserialize_struct<V>(
            self,
            name: &'static str,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de> {
        let input = self.input;

        // Collect string values
        // Example: val1 val2 "val val3" val4
        let mut cursor = 0;
        let words = input.split_whitespace().collect::<Vec<&str>>();
        let mut values = HashMap::new();
        let mut key =  None;
        let mut positional_idx = 0;
        let mut can_be_positional = true;

        while cursor < words.len() {
            if words[cursor].contains('"') {
                //collect string value

                let mut end = cursor + 1;
                while !words[end].contains('"') && end < input.len() {
                    end += 1;
                }
                let string = words[cursor + 1..end].join(" ");
                cursor = end + 1;

                if key.is_none() {
                    if can_be_positional {
                        values.insert(fields[positional_idx].to_string(), string);
                        positional_idx += 1;
                    } else {
                        return Err(de::Error::custom(format!("Invalid key: {}", string)));
                    }
                } else {
                    values.insert(key.unwrap(), string);
                    key = None;
                }
            } else if words[cursor].starts_with("--") {
                
                let key_val = &words[cursor][2..];
                key = Some(key_val.to_string());

                can_be_positional = false;
                cursor += 1;
            } else {
                let value = words[cursor];
                let key_val = fields[positional_idx].to_string();

                values.insert(key_val, value.to_string());

                cursor += 1;
                positional_idx += 1;
            }
        }

        println!("values: {:?}", values);

        visitor.visit_map(CliMapVisitor::new(values))
    }

    forward_to_deserialize_any! {
        bytes byte_buf option
        unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

struct CliMapVisitor<'a> {
    values: HashMap<String, String>,

    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> CliMapVisitor<'a> {
    fn new(values: HashMap<String, String>) -> Self {
        Self { values, _marker: Default::default() }
    }
}

impl<'de> de::MapAccess<'de> for CliMapVisitor<'de> {
    type Error = de::value::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de> {

        if self.values.is_empty() {
            return Ok(None);
        }
        let key = self.values.keys().next().unwrap().clone();
        let value = self.values.remove(&key).unwrap();
        seed.deserialize(CliDeserializer::<'de>::from_str(&key).unwrap()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de> {
        todo!()
    }
}

enum CliToken {
    PositionValue(String),
    KeyValue(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, Default, PartialEq)]
    struct SetGold {
        gold: usize,
    }

    #[test]
    fn test_deserialize_int() {
        let input = "100";
        let mut deserializer = CliDeserializer::from_str(input).unwrap();
        let set_gold = i32::deserialize(deserializer).unwrap();
        assert_eq!(set_gold, 100);
    }

    #[test]
    fn test_deserialize_setgold() {
        let input = "100";
        let mut deserializer = CliDeserializer::from_str(input).unwrap();
        let set_gold = SetGold::deserialize(deserializer).unwrap();
        assert_eq!(set_gold, SetGold { gold: 100 });
    }
}