#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Well-known reflection annotations for tools such as inspectors and editors.
//!
//! The mechanism lives in [`bevy_reflect`]: custom attributes are attached with
//! `#[reflect(@...)]` on types, fields and variants, and read back with
//! `get_attribute::<T>()`. This crate only defines a shared vocabulary of
//! attribute types, so that tools can agree on what to look for.
//!
//! Several annotations can be attached to the same item.
//!
//! ```
//! use bevy_annotations::{DisplayName, Range, ReadOnly, Step, Tooltip, Unit};
//! use bevy_reflect::{Reflect, TypeInfo, Typed};
//!
//! #[derive(Reflect)]
//! #[reflect(@Tooltip("Movement settings for a character"))]
//! struct Movement {
//!     #[reflect(@Range(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
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
//! assert_eq!(speed.get_attribute::<Range>(), Some(&Range(0.0..=20.0)));
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

/// Limits a numeric field to an inclusive range, such as `@Range(0.0..=20.0)`.
///
/// One type covers all numeric fields, so the bounds are stored as [`f64`].
/// Tools should also accept a plain [`RangeInclusive`] attribute, see
/// [`Range::from_attributes`].
#[derive(Reflect, Clone, Debug, PartialEq)]
#[reflect(Clone, Debug, PartialEq)]
pub struct Range(pub RangeInclusive<f64>);

impl Range {
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
    /// Accepts a [`Range`], or a [`RangeInclusive`] of any primitive integer
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
    #[reflect(@Tooltip("A named struct"), @DisplayName("Named"), @ReadOnly)]
    struct Named {
        #[reflect(@Range(0.0..=1.0))]
        range: f32,
        #[reflect(@Step(0.25))]
        step: f32,
        #[reflect(@Unit("m"))]
        unit: f32,
        #[reflect(@ReadOnly)]
        read_only: f32,
        #[reflect(@Tooltip("help"))]
        tooltip: f32,
        #[reflect(@DisplayName("Nice"))]
        display_name: f32,
        #[reflect(
            @Range(-1.0..=1.0),
            @Step(0.1),
            @Unit("deg"),
            @ReadOnly,
            @Tooltip("all"),
            @DisplayName("All")
        )]
        all: f32,
        #[reflect(@0.0_f32..=2.0_f32)]
        range_f32: f32,
        #[reflect(@-5_i32..=5_i32)]
        range_i32: i32,
        plain: f32,
    }

    #[derive(Reflect)]
    struct Tuple(
        #[reflect(@Range(0.0..=1.0))] f32,
        #[reflect(@Step(0.25))] f32,
        #[reflect(@Unit("m"))] f32,
        #[reflect(@ReadOnly)] f32,
        #[reflect(@Tooltip("help"))] f32,
        #[reflect(@DisplayName("Nice"))] f32,
    );

    #[derive(Reflect)]
    enum Shape {
        #[reflect(@Tooltip("variant"), @DisplayName("Round"))]
        Circle {
            #[reflect(@Range(0.0..=1.0))]
            range: f32,
            #[reflect(@Step(0.25))]
            step: f32,
            #[reflect(@Unit("m"))]
            unit: f32,
            #[reflect(@ReadOnly)]
            read_only: f32,
            #[reflect(@Tooltip("help"))]
            tooltip: f32,
            #[reflect(@DisplayName("Nice"))]
            display_name: f32,
        },
    }

    fn named_info() -> &'static bevy_reflect::structs::StructInfo {
        let TypeInfo::Struct(info) = Named::type_info() else {
            panic!("expected struct info");
        };
        info
    }

    #[test]
    fn named_fields() {
        let info = named_info();
        let field = |name| info.field(name).unwrap();
        assert_eq!(
            field("range").get_attribute::<Range>(),
            Some(&Range(0.0..=1.0))
        );
        assert_eq!(field("step").get_attribute::<Step>(), Some(&Step(0.25)));
        assert_eq!(field("unit").get_attribute::<Unit>(), Some(&Unit("m")));
        assert_eq!(
            field("read_only").get_attribute::<ReadOnly>(),
            Some(&ReadOnly)
        );
        assert_eq!(
            field("tooltip").get_attribute::<Tooltip>(),
            Some(&Tooltip("help"))
        );
        assert_eq!(
            field("display_name").get_attribute::<DisplayName>(),
            Some(&DisplayName("Nice"))
        );
        assert!(field("plain").custom_attributes().is_empty());
    }

    #[test]
    fn tuple_fields() {
        let TypeInfo::TupleStruct(info) = Tuple::type_info() else {
            panic!("expected tuple struct info");
        };
        let field = |index| info.field_at(index).unwrap();
        assert_eq!(field(0).get_attribute::<Range>(), Some(&Range(0.0..=1.0)));
        assert_eq!(field(1).get_attribute::<Step>(), Some(&Step(0.25)));
        assert_eq!(field(2).get_attribute::<Unit>(), Some(&Unit("m")));
        assert_eq!(field(3).get_attribute::<ReadOnly>(), Some(&ReadOnly));
        assert_eq!(field(4).get_attribute::<Tooltip>(), Some(&Tooltip("help")));
        assert_eq!(
            field(5).get_attribute::<DisplayName>(),
            Some(&DisplayName("Nice"))
        );
    }

    #[test]
    fn enum_variant_fields() {
        let TypeInfo::Enum(info) = Shape::type_info() else {
            panic!("expected enum info");
        };
        let variant = info.variant("Circle").unwrap();
        assert_eq!(
            variant.get_attribute::<Tooltip>(),
            Some(&Tooltip("variant"))
        );
        assert_eq!(
            variant.get_attribute::<DisplayName>(),
            Some(&DisplayName("Round"))
        );

        let bevy_reflect::enums::VariantInfo::Struct(variant) = variant else {
            panic!("expected struct variant");
        };
        let field = |name| variant.field(name).unwrap();
        assert_eq!(
            field("range").get_attribute::<Range>(),
            Some(&Range(0.0..=1.0))
        );
        assert_eq!(field("step").get_attribute::<Step>(), Some(&Step(0.25)));
        assert_eq!(field("unit").get_attribute::<Unit>(), Some(&Unit("m")));
        assert_eq!(
            field("read_only").get_attribute::<ReadOnly>(),
            Some(&ReadOnly)
        );
        assert_eq!(
            field("tooltip").get_attribute::<Tooltip>(),
            Some(&Tooltip("help"))
        );
        assert_eq!(
            field("display_name").get_attribute::<DisplayName>(),
            Some(&DisplayName("Nice"))
        );
    }

    #[test]
    fn type_level() {
        let info = named_info();
        assert_eq!(
            info.get_attribute::<Tooltip>(),
            Some(&Tooltip("A named struct"))
        );
        assert_eq!(
            info.get_attribute::<DisplayName>(),
            Some(&DisplayName("Named"))
        );
        assert_eq!(info.get_attribute::<ReadOnly>(), Some(&ReadOnly));
    }

    #[test]
    fn several_on_one_field() {
        let field = named_info().field("all").unwrap();
        assert_eq!(field.custom_attributes().len(), 6);
        assert_eq!(field.get_attribute::<Range>(), Some(&Range(-1.0..=1.0)));
        assert_eq!(field.get_attribute::<Step>(), Some(&Step(0.1)));
        assert_eq!(field.get_attribute::<Unit>(), Some(&Unit("deg")));
        assert!(field.has_attribute::<ReadOnly>());
        assert_eq!(field.get_attribute::<Tooltip>(), Some(&Tooltip("all")));
        assert_eq!(
            field.get_attribute::<DisplayName>(),
            Some(&DisplayName("All"))
        );
    }

    #[test]
    fn range_from_attributes() {
        let info = named_info();
        let range = |name| Range::from_attributes(info.field(name).unwrap().custom_attributes());
        assert_eq!(range("range"), Some(Range(0.0..=1.0)));
        assert_eq!(range("range_f32"), Some(Range(0.0..=2.0)));
        assert_eq!(range("range_i32"), Some(Range(-5.0..=5.0)));
        let range_i32 = range("range_i32").unwrap();
        assert_eq!((range_i32.min(), range_i32.max()), (-5.0, 5.0));
        assert_eq!(range("plain"), None);
        assert_eq!(range("step"), None);
    }
}
