//! Bevy-specific type formatting

use serde_json::Value;

/// Helper function to check if all values are numbers
pub fn all_numbers(values: &[&Value]) -> bool {
    values.iter().all(|v| v.is_number())
}

/// Format Transform components
pub fn format_transform(value: &Value) -> String {
    if let Some(obj) = value.as_object() {
        let mut parts = Vec::new();
        
        if let Some(translation) = obj.get("translation") {
            parts.push(format!("translation: {}", format_vec3(translation)));
        }
        if let Some(rotation) = obj.get("rotation") {
            parts.push(format!("rotation: {}", format_quat(rotation)));
        }
        if let Some(scale) = obj.get("scale") {
            parts.push(format!("scale: {}", format_vec3(scale)));
        }
        
        if !parts.is_empty() {
            return format!("Transform {{ {} }}", parts.join(", "));
        }
    }
    
    format!("Transform: {}", value)
}

/// Format Vec3 values
pub fn format_vec3(value: &Value) -> String {
    if let Some(obj) = value.as_object() {
        if let (Some(x), Some(y), Some(z)) = (obj.get("x"), obj.get("y"), obj.get("z")) {
            if all_numbers(&[x, y, z]) {
                return format!("({:.3}, {:.3}, {:.3})", 
                    x.as_f64().unwrap_or(0.0), 
                    y.as_f64().unwrap_or(0.0), 
                    z.as_f64().unwrap_or(0.0));
            }
        }
    }
    
    format!("Vec3: {}", value)
}

/// Format Vec2 values
pub fn format_vec2(value: &Value) -> String {
    if let Some(obj) = value.as_object() {
        if let (Some(x), Some(y)) = (obj.get("x"), obj.get("y")) {
            if all_numbers(&[x, y]) {
                return format!("({:.3}, {:.3})", 
                    x.as_f64().unwrap_or(0.0), 
                    y.as_f64().unwrap_or(0.0));
            }
        }
    }
    
    format!("Vec2: {}", value)
}

/// Format Quat values
pub fn format_quat(value: &Value) -> String {
    if let Some(obj) = value.as_object() {
        if let (Some(x), Some(y), Some(z), Some(w)) = (obj.get("x"), obj.get("y"), obj.get("z"), obj.get("w")) {
            if all_numbers(&[x, y, z, w]) {
                return format!("({:.3}, {:.3}, {:.3}, {:.3})", 
                    x.as_f64().unwrap_or(0.0), 
                    y.as_f64().unwrap_or(0.0), 
                    z.as_f64().unwrap_or(0.0),
                    w.as_f64().unwrap_or(0.0));
            }
        }
    }
    
    format!("Quat: {}", value)
}

/// Format Color values
pub fn format_color(value: &Value) -> String {
    if let Some(obj) = value.as_object() {
        if let (Some(r), Some(g), Some(b)) = (obj.get("r"), obj.get("g"), obj.get("b")) {
            if all_numbers(&[r, g, b]) {
                let mut result = format!("rgb({:.3}, {:.3}, {:.3})", 
                    r.as_f64().unwrap_or(0.0), 
                    g.as_f64().unwrap_or(0.0), 
                    b.as_f64().unwrap_or(0.0));
                
                if let Some(a) = obj.get("a") {
                    if a.is_number() {
                        result = format!("rgba({:.3}, {:.3}, {:.3}, {:.3})", 
                            r.as_f64().unwrap_or(0.0), 
                            g.as_f64().unwrap_or(0.0), 
                            b.as_f64().unwrap_or(0.0),
                            a.as_f64().unwrap_or(0.0));
                    }
                }
                
                return result;
            }
        }
    }
    
    format!("Color: {}", value)
}

/// Format common Bevy types
pub fn format_bevy_type(type_name: &str, value: &Value) -> String {
    match type_name {
        "Transform" => format_transform(value),
        "Vec3" => format_vec3(value),
        "Vec2" => format_vec2(value), 
        "Quat" => format_quat(value),
        "Color" => format_color(value),
        _ => format!("{}: {}", type_name, value)
    }
}
