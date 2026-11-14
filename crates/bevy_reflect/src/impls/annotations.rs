use crate::{attributes::CustomAttributes, std_traits::ReflectDefault};
use bevy_annotations::{DisplayName, ReadOnly, Step, Tooltip, Unit, ValueRange};
use bevy_reflect_derive::{impl_reflect, impl_reflect_opaque};
use core::ops::RangeInclusive;

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq)]
    #[type_path = "bevy_annotations"]
    struct ValueRange(RangeInclusive<f64>);
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq)]
    #[type_path = "bevy_annotations"]
    struct Step(f64);
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq)]
    #[type_path = "bevy_annotations"]
    struct Unit(&'static str);
);
impl_reflect!(
    #[reflect(Clone, Debug, Default, PartialEq)]
    #[type_path = "bevy_annotations"]
    struct ReadOnly;
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq)]
    #[type_path = "bevy_annotations"]
    struct Tooltip(&'static str);
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq)]
    #[type_path = "bevy_annotations"]
    struct DisplayName(&'static str);
);

impl_reflect_opaque!(::bevy_annotations::SerializedAs(Clone, Debug, PartialEq));

impl CustomAttributes {
    /// Reads a numeric range from these attributes.
    ///
    /// Accepts a [`ValueRange`], or a [`RangeInclusive`] of any primitive integer
    /// or float type, converted to [`f64`]. Returns `None` if neither is present.
    pub fn value_range(&self) -> Option<ValueRange> {
        if let Some(range) = self.get::<ValueRange>() {
            return Some(range.clone());
        }
        macro_rules! try_range_inclusive {
            ($($ty:ty),*) => {
                $(
                    if let Some(range) = self.get::<RangeInclusive<$ty>>() {
                        return Some(ValueRange(*range.start() as f64..=*range.end() as f64));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Reflect, TypeInfo, Typed};
    use bevy_annotations::SerializedAs;

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

    #[derive(Reflect)]
    #[reflect(@Tooltip("Movement settings for a character"))]
    struct Movement {
        #[reflect(@ValueRange(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
        speed: f32,
        #[reflect(@DisplayName("Jump Height"), @Tooltip("Peak height of a jump"))]
        jump: f32,
        #[reflect(@ReadOnly)]
        grounded: bool,
    }

    #[derive(Reflect)]
    #[reflect(@SerializedAs::Seq)]
    struct Span {
        min: f32,
        max: f32,
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque, @SerializedAs::STRING)]
    struct Tag;

    #[test]
    fn annotations_on_types_and_fields() {
        let TypeInfo::Struct(info) = Movement::type_info() else {
            panic!("expected struct info");
        };
        let speed = info.field("speed").unwrap();
        assert_eq!(
            speed.get_attribute::<ValueRange>(),
            Some(&ValueRange(0.0..=20.0))
        );
        assert_eq!(speed.get_attribute::<Step>(), Some(&Step(0.5)));
        assert_eq!(speed.get_attribute::<Unit>(), Some(&Unit("m/s")));
        let jump = info.field("jump").unwrap();
        assert_eq!(
            jump.get_attribute::<DisplayName>(),
            Some(&DisplayName("Jump Height"))
        );
        assert!(info.field("grounded").unwrap().has_attribute::<ReadOnly>());
        assert!(info.has_attribute::<Tooltip>());
    }

    #[test]
    fn serialized_as_on_struct_and_opaque() {
        let TypeInfo::Struct(span) = Span::type_info() else {
            panic!("expected struct info");
        };
        assert_eq!(
            span.get_attribute::<SerializedAs>(),
            Some(&SerializedAs::Seq)
        );
        let TypeInfo::Opaque(tag) = Tag::type_info() else {
            panic!("expected opaque info");
        };
        assert_eq!(
            tag.get_attribute::<SerializedAs>(),
            Some(&SerializedAs::STRING)
        );
    }

    #[test]
    fn range_from_attributes() {
        let TypeInfo::Struct(info) = Named::type_info() else {
            panic!("expected struct info");
        };
        let range = |name| info.field(name).unwrap().custom_attributes().value_range();
        assert_eq!(range("range"), Some(ValueRange(0.0..=1.0)));
        assert_eq!(range("range_f32"), Some(ValueRange(0.0..=2.0)));
        assert_eq!(range("range_i32"), Some(ValueRange(-5.0..=5.0)));
        let range_i32 = range("range_i32").unwrap();
        assert_eq!((range_i32.min(), range_i32.max()), (-5.0, 5.0));
        assert_eq!(range("plain"), None);
        assert_eq!(range("step"), None);
    }
}
