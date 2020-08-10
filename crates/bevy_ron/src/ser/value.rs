use serde::ser::{Serialize, Serializer};

use crate::value::{Number, Value};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Map(ref m) => Serialize::serialize(m, serializer),
            Value::Number(Number::Float(ref f)) => serializer.serialize_f64(f.get()),
            Value::Number(Number::Integer(i)) => serializer.serialize_i64(i),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Seq(ref s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
        }
    }
}
