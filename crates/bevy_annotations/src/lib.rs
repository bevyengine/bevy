#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Standard ("well-known") reflection annotations for tools such as inspectors and editors.
//!
//! The mechanism lives in [`bevy_reflect`]: custom attributes are attached with
//! `#[reflect(@...)]` on types, fields and variants, and read back with
//! `get_attribute::<T>()`. This crate only defines a shared vocabulary of
//! attribute types, so that tools can agree on what to look for.
//!
//! Several annotations can be attached to the same item.
//!
//! ```
//! use bevy_annotations::{DisplayName, ValueRange, ReadOnly, Step, Tooltip, Unit};
//! use bevy_reflect::{Reflect, TypeInfo, Typed};
//!
//! #[derive(Reflect)]
//! #[reflect(@Tooltip("Movement settings for a character"))]
//! struct Movement {
//!     #[reflect(@ValueRange(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
//!     speed: f32,
//!     #[reflect(@DisplayName("Jump Height"), @Tooltip("Peak height of a jump"))]
//!     jump: f32,
//!     #[reflect(@ReadOnly)]
//!     grounded: bool,
//! }
//!
//! let TypeInfo::Struct(info) = Movement::type_info() else {
//!     panic!("expected struct info");
//! };
//!
//! let speed = info.field("speed").unwrap();
//! assert_eq!(speed.get_attribute::<ValueRange>(), Some(&ValueRange(0.0..=20.0)));
//! assert_eq!(speed.get_attribute::<Step>(), Some(&Step(0.5)));
//! assert_eq!(speed.get_attribute::<Unit>(), Some(&Unit("m/s")));
//!
//! let jump = info.field("jump").unwrap();
//! assert_eq!(jump.get_attribute::<DisplayName>(), Some(&DisplayName("Jump Height")));
//!
//! assert!(info.field("grounded").unwrap().has_attribute::<ReadOnly>());
//! assert!(info.has_attribute::<Tooltip>());
//! ```

use bevy_reflect::{attributes::CustomAttributes, std_traits::ReflectDefault, Reflect};
use core::ops::RangeInclusive;

/// Limits a numeric field to an inclusive range, such as `@ValueRange(0.0..=20.0)`.
///
/// One type covers all numeric fields, so the bounds are stored as [`f64`].
/// Tools should also accept a plain [`RangeInclusive`] attribute, see
/// [`ValueRange::from_attributes`].
#[derive(Reflect, Clone, Debug, PartialEq)]
#[reflect(Clone, Debug, PartialEq)]
pub struct ValueRange(pub RangeInclusive<f64>);

impl ValueRange {
    /// The lowest allowed value.
    pub fn min(&self) -> f64 {
        *self.0.start()
    }

    /// The highest allowed value.
    pub fn max(&self) -> f64 {
        *self.0.end()
    }

    /// Reads a numeric range from `attributes`.
    ///
    /// Accepts a [`ValueRange`], or a [`RangeInclusive`] of any primitive integer
    /// or float type, converted to [`f64`]. Returns `None` if neither is present.
    pub fn from_attributes(attributes: &CustomAttributes) -> Option<Self> {
        if let Some(range) = attributes.get::<Self>() {
            return Some(range.clone());
        }
        macro_rules! try_range_inclusive {
            ($($ty:ty),*) => {
                $(
                    if let Some(range) = attributes.get::<RangeInclusive<$ty>>() {
                        return Some(Self(*range.start() as f64..=*range.end() as f64));
                    }
                )*
            };
        }
        try_range_inclusive!(
            f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
        );
        None
    }
}

/// The increment applied per step or drag on a numeric field, such as `@Step(0.5)`.
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Clone, Debug, PartialEq)]
pub struct Step(pub f64);

/// A unit label shown next to a value, such as `@Unit("m")`, `@Unit("deg")` or `@Unit("s")`.
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Clone, Debug, PartialEq)]
pub struct Unit(pub &'static str);

/// Marks a field or type as not editable by tools.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq)]
#[reflect(Clone, Debug, Default, PartialEq)]
pub struct ReadOnly;

/// Help text for a type, field or variant, such as `@Tooltip("Peak height of a jump")`.
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Clone, Debug, PartialEq)]
pub struct Tooltip(pub &'static str);

/// A friendlier label shown in place of the type, field or variant name, such as
/// `@DisplayName("Jump Height")`.
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Clone, Debug, PartialEq)]
pub struct DisplayName(pub &'static str);

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_reflect::{TypeInfo, Typed};

    #[derive(Reflect)]
    struct Named {
        #[reflect(@ValueRange(0.0..=1.0))]
        range: f32,
        #[reflect(@Step(0.25))]
        step: f32,
        #[reflect(@0.0_f32..=2.0_f32)]
        range_f32: f32,
        #[reflect(@-5_i32..=5_i32)]
        range_i32: i32,
        plain: f32,
    }

    #[test]
    fn range_from_attributes() {
        let TypeInfo::Struct(info) = Named::type_info() else {
            panic!("expected struct info");
        };
        let range =
            |name| ValueRange::from_attributes(info.field(name).unwrap().custom_attributes());
        assert_eq!(range("range"), Some(ValueRange(0.0..=1.0)));
        assert_eq!(range("range_f32"), Some(ValueRange(0.0..=2.0)));
        assert_eq!(range("range_i32"), Some(ValueRange(-5.0..=5.0)));
        let range_i32 = range("range_i32").unwrap();
        assert_eq!((range_i32.min(), range_i32.max()), (-5.0, 5.0));
        assert_eq!(range("plain"), None);
        assert_eq!(range("step"), None);
    }
}
