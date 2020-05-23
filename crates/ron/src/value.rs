//! Value module.

use serde::{
    de::{
        DeserializeOwned, DeserializeSeed, Deserializer, Error as SerdeError, MapAccess, SeqAccess,
        Visitor,
    },
    forward_to_deserialize_any, Deserialize, Serialize,
};
use std::{
    cmp::{Eq, Ordering},
    hash::{Hash, Hasher},
    iter::FromIterator,
    ops::{Index, IndexMut},
};

use crate::de::{Error as RonError, Result};

/// A `Value` to `Value` map.
///
/// This structure either uses a [BTreeMap](std::collections::BTreeMap) or the
/// [IndexMap](indexmap::IndexMap) internally.
/// The latter can be used by enabling the `indexmap` feature. This can be used
/// to preserve the order of the parsed map.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Map(MapInner);

impl Map {
    /// Creates a new, empty `Map`.
    pub fn new() -> Map {
        Default::default()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if `self.len() == 0`, `false` otherwise.
    pub fn is_empty(&self) -> usize {
        self.0.len()
    }

    /// Inserts a new element, returning the previous element with this `key` if
    /// there was any.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    /// Removes an element by its `key`.
    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        self.0.remove(key)
    }

    /// Iterate all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> + DoubleEndedIterator {
        self.0.iter()
    }

    /// Iterate all key-value pairs mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Value, &mut Value)> + DoubleEndedIterator {
        self.0.iter_mut()
    }

    /// Iterate all keys.
    pub fn keys(&self) -> impl Iterator<Item = &Value> + DoubleEndedIterator {
        self.0.keys()
    }

    /// Iterate all values.
    pub fn values(&self) -> impl Iterator<Item = &Value> + DoubleEndedIterator {
        self.0.values()
    }

    /// Iterate all values mutably.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> + DoubleEndedIterator {
        self.0.values_mut()
    }
}

impl FromIterator<(Value, Value)> for Map {
    fn from_iter<T: IntoIterator<Item = (Value, Value)>>(iter: T) -> Self {
        Map(MapInner::from_iter(iter))
    }
}

/// Note: equality is only given if both values and order of values match
impl Eq for Map {}

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.iter().for_each(|x| x.hash(state));
    }
}

impl Index<&Value> for Map {
    type Output = Value;

    fn index(&self, index: &Value) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<&Value> for Map {
    fn index_mut(&mut self, index: &Value) -> &mut Self::Output {
        self.0.get_mut(index).expect("no entry found for key")
    }
}

impl Ord for Map {
    fn cmp(&self, other: &Map) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

/// Note: equality is only given if both values and order of values match
impl PartialEq for Map {
    fn eq(&self, other: &Map) -> bool {
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl PartialOrd for Map {
    fn partial_cmp(&self, other: &Map) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

#[cfg(not(feature = "indexmap"))]
type MapInner = std::collections::BTreeMap<Value, Value>;
#[cfg(feature = "indexmap")]
type MapInner = indexmap::IndexMap<Value, Value>;

/// A wrapper for a number, which can be either `f64` or `i64`.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Hash, Ord)]
pub enum Number {
    Integer(i64),
    Float(Float),
}

/// A wrapper for `f64`, which guarantees that the inner value
/// is finite and thus implements `Eq`, `Hash` and `Ord`.
#[derive(Copy, Clone, Debug)]
pub struct Float(f64);

impl Float {
    /// Construct a new `Float`.
    pub fn new(v: f64) -> Self {
        Float(v)
    }

    /// Returns the wrapped float.
    pub fn get(self) -> f64 {
        self.0
    }
}

impl Number {
    /// Construct a new number.
    pub fn new(v: impl Into<Number>) -> Self {
        v.into()
    }

    /// Returns the `f64` representation of the number regardless of whether the number is stored
    /// as a float or integer.
    ///
    /// # Example
    ///
    /// ```
    /// # use ron::value::Number;
    /// let i = Number::new(5);
    /// let f = Number::new(2.0);
    /// assert_eq!(i.into_f64(), 5.0);
    /// assert_eq!(f.into_f64(), 2.0);
    /// ```
    pub fn into_f64(self) -> f64 {
        self.map_to(|i| i as f64, |f| f)
    }

    /// If the `Number` is a float, return it. Otherwise return `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use ron::value::Number;
    /// let i = Number::new(5);
    /// let f = Number::new(2.0);
    /// assert_eq!(i.as_f64(), None);
    /// assert_eq!(f.as_f64(), Some(2.0));
    /// ```
    pub fn as_f64(self) -> Option<f64> {
        self.map_to(|_| None, Some)
    }

    /// If the `Number` is an integer, return it. Otherwise return `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use ron::value::Number;
    /// let i = Number::new(5);
    /// let f = Number::new(2.0);
    /// assert_eq!(i.as_i64(), Some(5));
    /// assert_eq!(f.as_i64(), None);
    /// ```
    pub fn as_i64(self) -> Option<i64> {
        self.map_to(Some, |_| None)
    }

    /// Map this number to a single type using the appropriate closure.
    ///
    /// # Example
    ///
    /// ```
    /// # use ron::value::Number;
    /// let i = Number::new(5);
    /// let f = Number::new(2.0);
    /// assert!(i.map_to(|i| i > 3, |f| f > 3.0));
    /// assert!(!f.map_to(|i| i > 3, |f| f > 3.0));
    /// ```
    pub fn map_to<T>(
        self,
        integer_fn: impl FnOnce(i64) -> T,
        float_fn: impl FnOnce(f64) -> T,
    ) -> T {
        match self {
            Number::Integer(i) => integer_fn(i),
            Number::Float(Float(f)) => float_fn(f),
        }
    }
}

impl From<f64> for Number {
    fn from(f: f64) -> Number {
        Number::Float(Float(f))
    }
}

impl From<i64> for Number {
    fn from(i: i64) -> Number {
        Number::Integer(i)
    }
}

impl From<i32> for Number {
    fn from(i: i32) -> Number {
        Number::Integer(i as i64)
    }
}

// The following number conversion checks if the integer fits losslessly into an i64, before
// constructing a Number::Integer variant. If not, the conversion defaults to float.

impl From<u64> for Number {
    fn from(i: u64) -> Number {
        if i as i64 as u64 == i {
            Number::Integer(i as i64)
        } else {
            Number::new(i as f64)
        }
    }
}

/// Partial equality comparison
/// In order to be able to use `Number` as a mapping key, NaN floating values
/// wrapped in `Float` are equals to each other. It is not the case for
/// underlying `f64` values itself.
impl PartialEq for Float {
    fn eq(&self, other: &Self) -> bool {
        self.0.is_nan() && other.0.is_nan() || self.0 == other.0
    }
}

/// Equality comparison
/// In order to be able to use `Float` as a mapping key, NaN floating values
/// wrapped in `Float` are equals to each other. It is not the case for
/// underlying `f64` values itself.
impl Eq for Float {}

impl Hash for Float {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0 as u64);
    }
}

/// Partial ordering comparison
/// In order to be able to use `Number` as a mapping key, NaN floating values
/// wrapped in `Number` are equals to each other and are less then any other
/// floating value. It is not the case for the underlying `f64` values themselves.
/// ```
/// use ron::value::Number;
/// assert!(Number::new(std::f64::NAN) < Number::new(std::f64::NEG_INFINITY));
/// assert_eq!(Number::new(std::f64::NAN), Number::new(std::f64::NAN));
/// ```
impl PartialOrd for Float {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.0.is_nan(), other.0.is_nan()) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            _ => self.0.partial_cmp(&other.0),
        }
    }
}

/// Ordering comparison
/// In order to be able to use `Float` as a mapping key, NaN floating values
/// wrapped in `Float` are equals to each other and are less then any other
/// floating value. It is not the case for underlying `f64` values itself. See
/// the `PartialEq` implementation.
impl Ord for Float {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("Bug: Contract violation")
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Char(char),
    Map(Map),
    Number(Number),
    Option(Option<Box<Value>>),
    String(String),
    Seq(Vec<Value>),
    Unit,
}

impl Value {
    /// Tries to deserialize this `Value` into `T`.
    pub fn into_rust<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(self)
    }
}

/// Deserializer implementation for RON `Value`.
/// This does not support enums (because `Value` doesn't store them).
impl<'de> Deserializer<'de> for Value {
    type Error = RonError;

    forward_to_deserialize_any! {
        bool f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Char(c) => visitor.visit_char(c),
            Value::Map(m) => visitor.visit_map(MapAccessor {
                keys: m.keys().cloned().rev().collect(),
                values: m.values().cloned().rev().collect(),
            }),
            Value::Number(Number::Float(ref f)) => visitor.visit_f64(f.get()),
            Value::Number(Number::Integer(i)) => visitor.visit_i64(i),
            Value::Option(Some(o)) => visitor.visit_some(*o),
            Value::Option(None) => visitor.visit_none(),
            Value::String(s) => visitor.visit_string(s),
            Value::Seq(mut seq) => {
                seq.reverse();
                visitor.visit_seq(Seq { seq })
            }
            Value::Unit => visitor.visit_unit(),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Number(Number::Integer(i)) => visitor.visit_i64(i),
            v => Err(RonError::custom(format!("Expected a number, got {:?}", v))),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Number(Number::Integer(i)) => visitor.visit_u64(i as u64),
            v => Err(RonError::custom(format!("Expected a number, got {:?}", v))),
        }
    }
}

struct MapAccessor {
    keys: Vec<Value>,
    values: Vec<Value>,
}

impl<'de> MapAccess<'de> for MapAccessor {
    type Error = RonError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // The `Vec` is reversed, so we can pop to get the originally first element
        self.keys
            .pop()
            .map_or(Ok(None), |v| seed.deserialize(v).map(Some))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // The `Vec` is reversed, so we can pop to get the originally first element
        self.values
            .pop()
            .map(|v| seed.deserialize(v))
            .expect("Contract violation")
    }
}

struct Seq {
    seq: Vec<Value>,
}

impl<'de> SeqAccess<'de> for Seq {
    type Error = RonError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // The `Vec` is reversed, so we can pop to get the originally first element
        self.seq
            .pop()
            .map_or(Ok(None), |v| seed.deserialize(v).map(Some))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::{collections::BTreeMap, fmt::Debug};

    fn assert_same<'de, T>(s: &'de str)
    where
        T: Debug + Deserialize<'de> + PartialEq,
    {
        use crate::de::from_str;

        let direct: T = from_str(s).unwrap();
        let value: Value = from_str(s).unwrap();
        let value = T::deserialize(value).unwrap();

        assert_eq!(direct, value, "Deserialization for {:?} is not the same", s);
    }

    #[test]
    fn boolean() {
        assert_same::<bool>("true");
        assert_same::<bool>("false");
    }

    #[test]
    fn float() {
        assert_same::<f64>("0.123");
        assert_same::<f64>("-4.19");
    }

    #[test]
    fn int() {
        assert_same::<u32>("626");
        assert_same::<i32>("-50");
    }

    #[test]
    fn char() {
        assert_same::<char>("'4'");
        assert_same::<char>("'c'");
    }

    #[test]
    fn map() {
        assert_same::<BTreeMap<char, String>>(
            "{
'a': \"Hello\",
'b': \"Bye\",
        }",
        );
    }

    #[test]
    fn option() {
        assert_same::<Option<char>>("Some('a')");
        assert_same::<Option<char>>("None");
    }

    #[test]
    fn seq() {
        assert_same::<Vec<f64>>("[1.0, 2.0, 3.0, 4.0]");
    }

    #[test]
    fn unit() {
        assert_same::<()>("()");
    }
}
