#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Standard ("well-known") reflection annotations for tools such as inspectors and editors.
//!
//! The mechanism lives in `bevy_reflect`: custom attributes are attached with
//! `#[reflect(@...)]` on types, fields and variants, and read back with
//! `get_attribute::<T>()`. This crate only defines a shared vocabulary of
//! attribute types, so that tools can agree on what to look for.
//!
//! The types have no dependencies, and `bevy_reflect` implements `Reflect` for them.
//! This lets the reflection impls in `bevy_reflect` itself use them.
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

use core::ops::RangeInclusive;

/// Limits a numeric field to an inclusive range, such as `@ValueRange(0.0..=20.0)`.
///
/// One type covers all numeric fields, so the bounds are stored as [`f64`].
/// Tools should also accept a plain [`RangeInclusive`] attribute, which
/// `CustomAttributes::value_range` in `bevy_reflect` handles.
#[derive(Clone, Debug, PartialEq)]
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
}

/// The increment applied per step or drag on a numeric field, such as `@Step(0.5)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step(pub f64);

/// A unit label shown next to a value, such as `@Unit("m")`, `@Unit("deg")` or `@Unit("s")`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit(pub &'static str);

/// Marks a field or type as not editable by tools.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReadOnly;

/// Help text for a type, field or variant, such as `@Tooltip("Peak height of a jump")`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tooltip(pub &'static str);

/// A friendlier label shown in place of the type, field or variant name, such as
/// `@DisplayName("Jump Height")`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayName(pub &'static str);

/// Describes what a type serializes to, for a `Serialize` implementation that does not follow
/// the type's reflected structure.
///
/// Tools that read serialized values, such as `bevy_remote`, use this to describe them. Types
/// without this annotation serialize as their reflected structure, unless they are opaque, in
/// which case the shape is unknown.
///
/// ```
/// use bevy_annotations::SerializedAs;
/// use bevy_reflect::Reflect;
///
/// #[derive(Reflect)]
/// #[reflect(@SerializedAs::Seq)]
/// struct Span {
///     min: f32,
///     max: f32,
/// }
///
/// #[derive(Reflect, Clone)]
/// #[reflect(opaque, @SerializedAs::STRING)]
/// struct Tag(u64);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SerializedAs {
    /// A flat array of the fields of a struct or tuple struct, in order.
    ///
    /// Fields whose types are also `Seq` are inlined, so `Mat2`, with two `Vec2` fields,
    /// serializes as an array of four numbers.
    Seq,
    /// A fixed-length array with one labeled item per entry, for opaque types.
    Array {
        /// The type path of the items.
        item: &'static str,
        /// The label of each item.
        labels: &'static [&'static str],
    },
    /// A map with exactly these fields, for opaque types.
    Object {
        /// The name and type path of each field.
        fields: &'static [(&'static str, &'static str)],
    },
    /// A string.
    String {
        /// The JSON Schema format of the string, such as `uuid`.
        format: Option<&'static str>,
        /// A regular expression that matches the string.
        pattern: Option<&'static str>,
    },
    /// An integer.
    Integer {
        /// The lowest allowed value.
        minimum: Option<i64>,
        /// Whether zero is excluded.
        non_zero: bool,
    },
}

impl SerializedAs {
    /// A string with no further constraints.
    pub const STRING: Self = Self::String {
        format: None,
        pattern: None,
    };
    /// An integer of at least one.
    pub const POSITIVE_INTEGER: Self = Self::Integer {
        minimum: Some(1),
        non_zero: false,
    };
    /// An integer other than zero.
    pub const NON_ZERO_INTEGER: Self = Self::Integer {
        minimum: None,
        non_zero: true,
    };
}
