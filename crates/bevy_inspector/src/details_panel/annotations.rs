//! The annotations the details panel reads from reflected types and fields.

use alloc::vec::Vec;

use bevy_annotations::{DisplayName, ReadOnly, Step, Unit, ValueRange};
use bevy_reflect::{attributes::CustomAttributes, TypeInfo};
use bevy_ui_widgets::{NumericRange, NumericValue};

use super::FieldValue;

/// The [`bevy_annotations`] of a field or type that the details panel honors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldAnnotations {
    /// The label shown in place of the field or type name.
    pub display_name: Option<&'static str>,
    /// Whether the value is shown without an editable widget, also set on the fields of a
    /// read-only value.
    pub read_only: bool,
    /// The range a number is limited to.
    pub range: Option<ValueRange>,
    /// The increment of a number input.
    pub step: Option<f64>,
    /// The unit shown with a number.
    pub unit: Option<&'static str>,
    /// The display names of the variants of an enum, as pairs of variant and display name.
    pub variant_names: Vec<(&'static str, &'static str)>,
}

impl FieldAnnotations {
    /// Reads the annotations from the custom attributes of a field or type.
    pub fn from_attributes(attributes: &CustomAttributes) -> Self {
        Self {
            display_name: attributes.get::<DisplayName>().map(|name| name.0),
            read_only: attributes.contains::<ReadOnly>(),
            range: attributes.value_range(),
            step: attributes.get::<Step>().map(|step| step.0),
            unit: attributes.get::<Unit>().map(|unit| unit.0),
            variant_names: Vec::new(),
        }
    }

    /// Reads the annotations of a struct, tuple struct or enum type, and of the variants of an
    /// enum.
    pub(super) fn of_type(info: Option<&TypeInfo>) -> Self {
        match info {
            Some(TypeInfo::Struct(info)) => Self::from_attributes(info.custom_attributes()),
            Some(TypeInfo::TupleStruct(info)) => Self::from_attributes(info.custom_attributes()),
            Some(TypeInfo::Enum(info)) => Self {
                variant_names: info
                    .iter()
                    .filter_map(|variant| {
                        let name = variant.get_attribute::<DisplayName>()?;
                        Some((variant.name(), name.0))
                    })
                    .collect(),
                ..Self::from_attributes(info.custom_attributes())
            },
            _ => Self::default(),
        }
    }

    /// Reads the annotations a field inherits from its type, which are those of the type except
    /// its display name.
    pub(super) fn of_field_type(info: Option<&TypeInfo>) -> Self {
        Self {
            display_name: None,
            ..Self::of_type(info)
        }
    }

    /// The annotations that also constrain the items of a list, array, map or set, and the
    /// payload of an `Option`: the range, step and unit.
    pub(super) fn value_constraints(&self) -> Self {
        Self {
            range: self.range.clone(),
            step: self.step,
            unit: self.unit,
            ..Self::default()
        }
    }

    /// The name shown for the enum variant `name`.
    pub(super) fn variant_label<'a>(&self, name: &'a str) -> &'a str {
        self.variant_names
            .iter()
            .find(|(variant, _)| *variant == name)
            .map_or(name, |(_, label)| label)
    }

    /// Fills the annotations missing from `self` with those of `other`.
    pub(super) fn or(self, other: Self) -> Self {
        Self {
            display_name: self.display_name.or(other.display_name),
            read_only: self.read_only || other.read_only,
            range: self.range.or(other.range),
            step: self.step.or(other.step),
            unit: self.unit.or(other.unit),
            variant_names: if self.variant_names.is_empty() {
                other.variant_names
            } else {
                self.variant_names
            },
        }
    }
}

/// Clamps a number to `range`, leaving other values and empty ranges untouched.
pub(super) fn clamp_to_range(value: &FieldValue, range: Option<&ValueRange>) -> FieldValue {
    match (value, range) {
        (FieldValue::Number(number), Some(range)) => match numeric_range(range, *number) {
            Some(range) => FieldValue::Number(range.clamp(*number)),
            None => value.clone(),
        },
        _ => value.clone(),
    }
}

/// `range` in the format of `number`, rounded inward for integers, or `None` if it is empty.
fn numeric_range(range: &ValueRange, number: NumericValue) -> Option<NumericRange> {
    let (min, max) = (range.min(), range.max());
    if min.is_nan() || max.is_nan() || min > max {
        return None;
    }
    let (min, max) = match number {
        NumericValue::F32(_) => return Some(NumericRange::F32(min as f32..=max as f32)),
        NumericValue::F64(_) => return Some(NumericRange::F64(min..=max)),
        _ => (min.ceil(), max.floor()),
    };
    if min > max {
        return None;
    }
    Some(match number {
        NumericValue::I32(_) => NumericRange::I32(min as i32..=max as i32),
        _ => NumericRange::I64(min as i64..=max as i64),
    })
}

/// The range a number input showing `value` is limited to: `limit`, the range of the field type,
/// narrowed to the annotated `range` where they overlap.
pub(super) fn narrowed_limit(
    limit: Option<NumericRange>,
    range: Option<&ValueRange>,
    value: NumericValue,
) -> Option<NumericRange> {
    let Some(range) = range.and_then(|range| numeric_range(range, value)) else {
        return limit;
    };
    match (limit, range) {
        (Some(NumericRange::I32(limit)), NumericRange::I32(range)) => {
            let start = *limit.start().max(range.start());
            let end = *limit.end().min(range.end());
            Some(NumericRange::I32(if start <= end {
                start..=end
            } else {
                limit
            }))
        }
        (Some(NumericRange::I64(limit)), NumericRange::I64(range)) => {
            let start = *limit.start().max(range.start());
            let end = *limit.end().min(range.end());
            Some(NumericRange::I64(if start <= end {
                start..=end
            } else {
                limit
            }))
        }
        (_, range) => Some(range),
    }
}

/// The annotations read from `attributes`, or none if there are no attributes.
pub(super) fn annotations_of(attributes: Option<&CustomAttributes>) -> FieldAnnotations {
    attributes
        .map(FieldAnnotations::from_attributes)
        .unwrap_or_default()
}
