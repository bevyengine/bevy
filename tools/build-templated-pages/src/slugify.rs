use std::collections::HashMap;

use tera::{to_value, try_get_value, Result, Value};

// A copy-and-paste from Tera. It is the only builtin we use.
// https://github.com/Keats/tera/blob/290889e61e9fda317f42f284e7a875342424d646/src/builtins/filters/string.rs#L240-L243
pub(crate) fn slugify(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("slugify", "value", String, value);
    Ok(to_value(slug::slugify(s)).unwrap())
}
