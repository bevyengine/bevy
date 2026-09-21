use std::ops::RangeInclusive;

use bevy_ecs::component::Component;
use bevy_log::warn_once;
use bevy_reflect::Reflect;

/// Used to indicate what format of numbers we are editing. This affects the type
/// of [`crate::ValueChange`] event that is emitted.
#[derive(Default, Clone, Copy, Reflect)]
pub enum NumericFormat {
    /// A 32-bit float
    #[default]
    F32,
    /// A 64-bit float
    F64,
    /// A 32-bit integer
    I32,
    /// A 64-bit integer
    I64,
}

/// Represents numbers in different formats.
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect)]
#[component(immutable)]
pub enum NumericValue {
    /// An `f32` value
    F32(f32),
    /// An `f64` value
    F64(f64),
    /// An `i32` value
    I32(i32),
    /// An `i64` value
    I64(i64),
}

impl core::fmt::Display for NumericValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NumericValue::F32(v) => write!(f, "{}", v),
            NumericValue::F64(v) => write!(f, "{}", v),
            NumericValue::I32(v) => write!(f, "{}", v),
            NumericValue::I64(v) => write!(f, "{}", v),
        }
    }
}

impl NumericValue {
    /// Returns the numeric format represented by this value.
    pub fn format(&self) -> NumericFormat {
        match self {
            Self::F32(_) => NumericFormat::F32,
            Self::F64(_) => NumericFormat::F64,
            Self::I32(_) => NumericFormat::I32,
            Self::I64(_) => NumericFormat::I64,
        }
    }

    /// Parses a string according to the requested numeric format.
    pub fn parse_from(value: &str, fmt: NumericFormat) -> Result<Self, String> {
        match fmt {
            NumericFormat::F32 => value
                .parse::<f32>()
                .map(NumericValue::F32)
                .map_err(|_| format!("Could not parse '{}' as f32", value)),
            NumericFormat::F64 => value
                .parse::<f64>()
                .map(NumericValue::F64)
                .map_err(|_| format!("Could not parse '{}' as f64", value)),
            NumericFormat::I32 => value
                .parse::<i32>()
                .map(NumericValue::I32)
                .map_err(|_| format!("Could not parse '{}' as i32", value)),
            NumericFormat::I64 => value
                .parse::<i64>()
                .map(NumericValue::I64)
                .map_err(|_| format!("Could not parse '{}' as i64", value)),
        }
    }

    /// Offset this value by `delta`, preserving the variant.
    pub fn offset_by(self, delta: f64) -> Self {
        match self {
            NumericValue::F32(v) => NumericValue::F32(v + delta as f32),
            NumericValue::F64(v) => NumericValue::F64(v + delta),
            NumericValue::I32(v) => NumericValue::I32(v.saturating_add(delta.round() as i32)),
            NumericValue::I64(v) => NumericValue::I64(v.saturating_add(delta.round() as i64)),
        }
    }

    /// Scale this value by `scale`, preserving the variant.
    pub fn scale_by(self, scale: f64) -> Self {
        match self {
            NumericValue::F32(v) => NumericValue::F32((v as f64 * scale) as f32),
            NumericValue::F64(v) => NumericValue::F64(v * scale),
            NumericValue::I32(v) => NumericValue::I32((v as f64 * scale).round() as i32),
            NumericValue::I64(v) => NumericValue::I64((v as f64 * scale).round() as i64),
        }
    }

    /// Returns this value as an `f64` for calculations that are format-independent.
    pub fn as_f64(&self) -> f64 {
        match *self {
            NumericValue::F32(v) => v as f64,
            NumericValue::F64(v) => v,
            NumericValue::I32(v) => v as f64,
            NumericValue::I64(v) => v as f64,
        }
    }
}

impl Default for NumericValue {
    fn default() -> Self {
        Self::F32(0.0)
    }
}

/// Represents numeric limits in different number formats.
#[derive(Debug, PartialEq, Clone, Reflect)]
pub enum NumericRange {
    /// An 'f32' range.
    F32(RangeInclusive<f32>),
    /// An 'f64' range.
    F64(RangeInclusive<f64>),
    /// An 'i32' range.
    I32(RangeInclusive<i32>),
    /// An 'i64' range.
    I64(RangeInclusive<i64>),
}

impl NumericRange {
    /// Clamp a numeric value of varying type to be within this range.
    pub fn clamp(&self, n: NumericValue) -> NumericValue {
        match (self, n) {
            (Self::F32(r), NumericValue::F32(v)) => {
                NumericValue::F32(v.clamp(*r.start(), *r.end()))
            }
            (Self::F64(r), NumericValue::F64(v)) => {
                NumericValue::F64(v.clamp(*r.start(), *r.end()))
            }
            (Self::I32(r), NumericValue::I32(v)) => {
                NumericValue::I32(v.clamp(*r.start(), *r.end()))
            }
            (Self::I64(r), NumericValue::I64(v)) => {
                NumericValue::I64(v.clamp(*r.start(), *r.end()))
            }
            (range, value) => {
                warn_once!("Number input range type mismatch: {range:?} {value:?}");
                n
            }
        }
    }

    /// Wrap a numeric value of varying type to be within this range.
    pub fn wrap(&self, n: NumericValue) -> NumericValue {
        match (self, n) {
            (Self::F32(r), NumericValue::F32(v)) => {
                let range = r.end() - r.start();
                NumericValue::F32(r.start() + (v - r.start()).rem_euclid(range))
            }
            (Self::F64(r), NumericValue::F64(v)) => {
                let range = r.end() - r.start();
                NumericValue::F64(r.start() + (v - r.start()).rem_euclid(range))
            }
            (Self::I32(r), NumericValue::I32(v)) => {
                let range = r.end() - r.start();
                NumericValue::I32(r.start() + (v - r.start()).rem_euclid(range))
            }
            (Self::I64(r), NumericValue::I64(v)) => {
                let range = r.end() - r.start();
                NumericValue::I64(r.start() + (v - r.start()).rem_euclid(range))
            }
            (range, value) => {
                warn_once!("Number input range type mismatch: {range:?} {value:?}");
                n
            }
        }
    }

    /// Compute the position of the thumb on the slide bar, as a value between 0 and 1, taking
    /// into account the proportion of the value between the minimum and maximum limits.
    pub fn thumb_position(&self, value: NumericValue) -> f32 {
        match (self, value) {
            (Self::F32(range), NumericValue::F32(n)) => {
                if range.end() > range.start() {
                    (n - range.start()) / (range.end() - range.start())
                } else {
                    0.5
                }
            }

            (Self::F64(range), NumericValue::F64(n)) => {
                if range.end() > range.start() {
                    ((n - range.start()) / (range.end() - range.start())) as f32
                } else {
                    0.5
                }
            }

            (Self::I32(range), NumericValue::I32(n)) => {
                if range.end() > range.start() {
                    (n - range.start()) as f32 / (range.end() - range.start()) as f32
                } else {
                    0.5
                }
            }

            (Self::I64(range), NumericValue::I64(n)) => {
                if range.end() > range.start() {
                    (n - range.start()) as f32 / (range.end() - range.start()) as f32
                } else {
                    0.5
                }
            }

            (range, value) => {
                warn_once!("Number input range type mismatch: {range:?} {value:?}");
                0.5
            }
        }
    }
}

impl Default for NumericRange {
    fn default() -> Self {
        Self::F32(0.0..=0.0)
    }
}
