//! Display utilities for component data

use serde_json::Value;

/// Format values for inline display (summaries)
pub fn format_value_inline(value: &Value) -> String {
    match value {
        Value::Object(obj) => {
            // Special cases for common Bevy types
            if let (Some(x), Some(y), Some(z)) = (obj.get("x"), obj.get("y"), obj.get("z")) {
                if all_numbers(&[x, y, z]) {
                    if let Some(w) = obj.get("w") {
                        if w.is_number() {
                            return format!("({:.2}, {:.2}, {:.2}, {:.2})", 
                                x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0), 
                                z.as_f64().unwrap_or(0.0), w.as_f64().unwrap_or(0.0));
                        }
                    }
                    return format!("({:.2}, {:.2}, {:.2})", 
                        x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0), z.as_f64().unwrap_or(0.0));
                }
            } else if let (Some(x), Some(y)) = (obj.get("x"), obj.get("y")) {
                if all_numbers(&[x, y]) {
                    return format!("({:.2}, {:.2})", x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
                }
            }
            format!("{{ {} fields }}", obj.len())
        }
        Value::Array(arr) => {
            if arr.len() <= 4 && arr.iter().all(|v| v.is_number()) {
                let nums: Vec<String> = arr.iter()
                    .map(|v| format!("{:.2}", v.as_f64().unwrap_or(0.0)))
                    .collect();
                format!("({})", nums.join(", "))
            } else {
                format!("[{} items]", arr.len())
            }
        }
        _ => format_simple_value(value)
    }
}

/// Format simple values (non-expandable)
pub fn format_simple_value(value: &Value) -> String {
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f >= 0.0 && f <= u32::MAX as f64 {
                    // Check if this looks like an Entity ID
                    if f > 0.0 && f < 1000000.0 {
                        format!("Entity({})", f as u64)
                    } else {
                        format!("{}", f as u64)
                    }
                } else {
                    format!("{:.3}", f)
                }
            } else {
                format!("{}", n)
            }
        }
        Value::String(s) => {
            // Truncate very long strings
            if s.len() > 50 {
                format!("\"{}...\"", &s[..47])
            } else if s.contains("::") && s.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '_') {
                // Looks like a type path - clean it up
                let clean = s.split("::").last().unwrap_or(s);
                format!("{}", clean)
            } else {
                format!("\"{}\"", s)
            }
        }
        Value::Bool(b) => format!("{}", b),
        Value::Null => "null".to_string(),
        _ => format!("{}", value)
    }
}

/// Create readable component names
pub fn humanize_component_name(name: &str) -> String {
    // Convert CamelCase to "Camel Case"
    let mut result = String::new();
    let mut chars = name.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && !result.is_empty() {
            result.push(' ');
        }
        result.push(ch);
    }
    
    result
}

/// Check if a value is simple (non-expandable)
pub fn is_simple_value(value: &Value) -> bool {
    matches!(value, Value::Number(_) | Value::String(_) | Value::Bool(_) | Value::Null)
}

/// Helper function to check if all values are numbers
pub fn all_numbers(values: &[&Value]) -> bool {
    values.iter().all(|v| v.is_number())
}
