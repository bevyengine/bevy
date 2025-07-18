//! # Component Data Formatting
//!
//! This module provides utilities for formatting and displaying component data
//! in a human-readable format. It handles various data types including JSON
//! objects, arrays, and primitive values with special formatting for common
//! Bevy types.
//!
//! ## Formatting Categories
//!
//! - **Parser**: JSON parsing and data structure extraction
//! - **Display**: Human-readable formatting for UI display
//! - **Bevy Types**: Specialized formatting for Bevy-specific types
//!
//! ## Supported Types
//!
//! The formatter recognizes and specially handles:
//! - **Transforms**: Position, rotation, and scale formatting
//! - **Vectors**: Vec2, Vec3 with decimal precision control
//! - **Colors**: RGBA color values with proper formatting
//! - **Primitives**: Numbers, strings, booleans with consistent styling
//! - **Collections**: Arrays and objects with proper indentation
//!
//! ## Usage
//!
//! ```rust,no_run
//! use bevy_editor::formatting::{format_value_inline, format_simple_value, is_simple_value};
//! use serde_json::Value;
//!
//! let json_value = Value::String("example".to_string());
//! 
//! // Format a JSON value for display
//! let formatted = format_value_inline(&json_value);
//! 
//! // Check if a value can be displayed simply
//! if is_simple_value(&json_value) {
//!     let simple = format_simple_value(&json_value);
//! }
//! ```

pub mod parser;
pub mod display;
pub mod bevy_types;

// Re-export key functions, avoiding conflicts
pub use parser::*;
pub use display::{format_value_inline, format_simple_value, humanize_component_name, is_simple_value, all_numbers};
pub use bevy_types::{format_bevy_type, format_transform, format_vec3, format_vec2, format_quat, format_color};
