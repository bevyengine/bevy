use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    PositionType, Size, Style, UiRect, Val,
};

use super::LayoutContext;

impl Val {
    fn into_length_percentage_auto(
        self,
        context: &LayoutContext,
    ) -> taffy::style::LengthPercentageAuto {
        match self {
            Val::Auto => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.),
            Val::Px(value) => taffy::style::LengthPercentageAuto::Points(
                (context.scale_factor * value as f64) as f32,
            ),
            Val::VMin(value) => {
                taffy::style::LengthPercentageAuto::Points(context.min_size * value / 100.)
            }
            Val::VMax(value) => {
                taffy::style::LengthPercentageAuto::Points(context.max_size * value / 100.)
            }
            Val::Vw(value) => {
                taffy::style::LengthPercentageAuto::Points(context.physical_size.x * value / 100.)
            }
            Val::Vh(value) => {
                taffy::style::LengthPercentageAuto::Points(context.physical_size.y * value / 100.)
            }
        }
    }

    fn into_length_percentage(self, context: &LayoutContext) -> taffy::style::LengthPercentage {
        match self.into_length_percentage_auto(context) {
            taffy::style::LengthPercentageAuto::Auto => taffy::style::LengthPercentage::Points(0.0),
            taffy::style::LengthPercentageAuto::Percent(value) => {
                taffy::style::LengthPercentage::Percent(value)
            }
            taffy::style::LengthPercentageAuto::Points(value) => {
                taffy::style::LengthPercentage::Points(value)
            }
        }
    }

    fn into_dimension(self, context: &LayoutContext) -> taffy::style::Dimension {
        self.into_length_percentage_auto(context).into()
    }
}

impl UiRect {
    fn map_to_taffy_rect<T>(self, map_fn: impl Fn(Val) -> T) -> taffy::geometry::Rect<T> {
        taffy::geometry::Rect {
            left: map_fn(self.left),
            right: map_fn(self.right),
            top: map_fn(self.top),
            bottom: map_fn(self.bottom),
        }
    }
}

impl Size {
    fn map_to_taffy_size<T>(self, map_fn: impl Fn(Val) -> T) -> taffy::geometry::Size<T> {
        taffy::geometry::Size {
            width: map_fn(self.width),
            height: map_fn(self.height),
        }
    }
}

pub fn from_style(context: &LayoutContext, style: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: style.display.into(),
        position: style.position_type.into(),
        flex_direction: style.flex_direction.into(),
        flex_wrap: style.flex_wrap.into(),
        align_items: Some(style.align_items.into()),
        align_self: style.align_self.into(),
        align_content: Some(style.align_content.into()),
        justify_content: Some(style.justify_content.into()),
        inset: taffy::prelude::Rect {
            left: style.left.into_length_percentage_auto(context),
            right: style.right.into_length_percentage_auto(context),
            top: style.top.into_length_percentage_auto(context),
            bottom: style.bottom.into_length_percentage_auto(context),
        },
        margin: style
            .margin
            .map_to_taffy_rect(|m| m.into_length_percentage_auto(context)),
        padding: style
            .padding
            .map_to_taffy_rect(|m| m.into_length_percentage(context)),
        border: style
            .border
            .map_to_taffy_rect(|m| m.into_length_percentage(context)),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis.into_dimension(context),
        size: style.size.map_to_taffy_size(|s| s.into_dimension(context)),
        min_size: style
            .min_size
            .map_to_taffy_size(|s| s.into_dimension(context)),
        max_size: style
            .max_size
            .map_to_taffy_size(|s| s.into_dimension(context)),
        aspect_ratio: style.aspect_ratio,
        gap: style
            .gap
            .map_to_taffy_size(|s| s.into_length_percentage(context)),
        justify_self: None,
    }
}

impl From<AlignItems> for taffy::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::Start => taffy::style::AlignItems::Start,
            AlignItems::End => taffy::style::AlignItems::End,
            AlignItems::FlexStart => taffy::style::AlignItems::FlexStart,
            AlignItems::FlexEnd => taffy::style::AlignItems::FlexEnd,
            AlignItems::Center => taffy::style::AlignItems::Center,
            AlignItems::Baseline => taffy::style::AlignItems::Baseline,
            AlignItems::Stretch => taffy::style::AlignItems::Stretch,
        }
    }
}

impl From<AlignSelf> for Option<taffy::style::AlignSelf> {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => None,
            AlignSelf::Start => taffy::style::AlignSelf::Start.into(),
            AlignSelf::End => taffy::style::AlignSelf::End.into(),
            AlignSelf::FlexStart => taffy::style::AlignSelf::FlexStart.into(),
            AlignSelf::FlexEnd => taffy::style::AlignSelf::FlexEnd.into(),
            AlignSelf::Center => taffy::style::AlignSelf::Center.into(),
            AlignSelf::Baseline => taffy::style::AlignSelf::Baseline.into(),
            AlignSelf::Stretch => taffy::style::AlignSelf::Stretch.into(),
        }
    }
}

impl From<AlignContent> for taffy::style::AlignContent {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::Start => taffy::style::AlignContent::Start,
            AlignContent::End => taffy::style::AlignContent::End,
            AlignContent::FlexStart => taffy::style::AlignContent::FlexStart,
            AlignContent::FlexEnd => taffy::style::AlignContent::FlexEnd,
            AlignContent::Center => taffy::style::AlignContent::Center,
            AlignContent::Stretch => taffy::style::AlignContent::Stretch,
            AlignContent::SpaceBetween => taffy::style::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => taffy::style::AlignContent::SpaceAround,
            AlignContent::SpaceEvenly => taffy::style::AlignContent::SpaceEvenly,
        }
    }
}

impl From<Display> for taffy::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => taffy::style::Display::Flex,
            Display::None => taffy::style::Display::None,
        }
    }
}

impl From<FlexDirection> for taffy::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => taffy::style::FlexDirection::Row,
            FlexDirection::Column => taffy::style::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::style::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::style::FlexDirection::ColumnReverse,
        }
    }
}

impl From<JustifyContent> for taffy::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::Start => taffy::style::JustifyContent::Start,
            JustifyContent::End => taffy::style::JustifyContent::End,
            JustifyContent::FlexStart => taffy::style::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::style::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::style::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::style::JustifyContent::SpaceEvenly,
        }
    }
}

impl From<PositionType> for taffy::style::Position {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => taffy::style::Position::Relative,
            PositionType::Absolute => taffy::style::Position::Absolute,
        }
    }
}

impl From<FlexWrap> for taffy::style::FlexWrap {
    fn from(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => taffy::style::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::style::FlexWrap::Wrap,
            FlexWrap::WrapReverse => taffy::style::FlexWrap::WrapReverse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_from() {
        let bevy_style = crate::Style {
            display: Display::Flex,
            position_type: PositionType::Absolute,
            left: Val::Px(0.),
            right: Val::Percent(0.),
            top: Val::Auto,
            bottom: Val::Auto,
            direction: crate::Direction::Inherit,
            flex_direction: FlexDirection::ColumnReverse,
            flex_wrap: FlexWrap::WrapReverse,
            align_items: AlignItems::Baseline,
            align_self: AlignSelf::Start,
            align_content: AlignContent::SpaceAround,
            justify_content: JustifyContent::SpaceEvenly,
            margin: UiRect {
                left: Val::Percent(0.),
                right: Val::Px(0.),
                top: Val::Auto,
                bottom: Val::Auto,
            },
            padding: UiRect {
                left: Val::Percent(0.),
                right: Val::Px(0.),
                top: Val::Percent(0.),
                bottom: Val::Percent(0.),
            },
            border: UiRect {
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Auto,
                bottom: Val::Px(0.),
            },
            flex_grow: 1.,
            flex_shrink: 0.,
            flex_basis: Val::Px(0.),
            size: Size {
                width: Val::Px(0.),
                height: Val::Auto,
            },
            min_size: Size {
                width: Val::Px(0.),
                height: Val::Percent(0.),
            },
            max_size: Size {
                width: Val::Auto,
                height: Val::Px(0.),
            },
            aspect_ratio: None,
            overflow: crate::Overflow::Hidden,
            gap: Size {
                width: Val::Px(0.),
                height: Val::Percent(0.),
            },
        };
        let viewport_values = LayoutContext::new(1.0, bevy_math::Vec2::new(800., 600.));
        let taffy_style = from_style(&viewport_values, &bevy_style);
        assert_eq!(taffy_style.display, taffy::style::Display::Flex);
        assert_eq!(taffy_style.position, taffy::style::Position::Absolute);
        assert!(matches!(
            taffy_style.inset.left,
            taffy::style::LengthPercentageAuto::Points(_)
        ));
        assert!(matches!(
            taffy_style.inset.right,
            taffy::style::LengthPercentageAuto::Percent(_)
        ));
        assert!(matches!(
            taffy_style.inset.top,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert!(matches!(
            taffy_style.inset.bottom,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert_eq!(
            taffy_style.flex_direction,
            taffy::style::FlexDirection::ColumnReverse
        );
        assert_eq!(taffy_style.flex_wrap, taffy::style::FlexWrap::WrapReverse);
        assert_eq!(
            taffy_style.align_items,
            Some(taffy::style::AlignItems::Baseline)
        );
        assert_eq!(taffy_style.align_self, Some(taffy::style::AlignSelf::Start));
        assert_eq!(
            taffy_style.align_content,
            Some(taffy::style::AlignContent::SpaceAround)
        );
        assert_eq!(
            taffy_style.justify_content,
            Some(taffy::style::JustifyContent::SpaceEvenly)
        );
        assert!(matches!(
            taffy_style.margin.left,
            taffy::style::LengthPercentageAuto::Percent(_)
        ));
        assert!(matches!(
            taffy_style.margin.right,
            taffy::style::LengthPercentageAuto::Points(_)
        ));
        assert!(matches!(
            taffy_style.margin.top,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert!(matches!(
            taffy_style.margin.bottom,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert!(matches!(
            taffy_style.padding.left,
            taffy::style::LengthPercentage::Percent(_)
        ));
        assert!(matches!(
            taffy_style.padding.right,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.padding.top,
            taffy::style::LengthPercentage::Percent(_)
        ));
        assert!(matches!(
            taffy_style.padding.bottom,
            taffy::style::LengthPercentage::Percent(_)
        ));
        assert!(matches!(
            taffy_style.border.left,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.border.right,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.border.top,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.border.bottom,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert_eq!(taffy_style.flex_grow, 1.);
        assert_eq!(taffy_style.flex_shrink, 0.);
        assert!(matches!(
            taffy_style.flex_basis,
            taffy::style::Dimension::Points(_)
        ));
        assert!(matches!(
            taffy_style.size.width,
            taffy::style::Dimension::Points(_)
        ));
        assert!(matches!(
            taffy_style.size.height,
            taffy::style::Dimension::Auto
        ));
        assert!(matches!(
            taffy_style.min_size.width,
            taffy::style::Dimension::Points(_)
        ));
        assert!(matches!(
            taffy_style.min_size.height,
            taffy::style::Dimension::Percent(_)
        ));
        assert!(matches!(
            taffy_style.max_size.width,
            taffy::style::Dimension::Auto
        ));
        assert!(matches!(
            taffy_style.max_size.height,
            taffy::style::Dimension::Points(_)
        ));
        assert_eq!(taffy_style.aspect_ratio, None);
        assert_eq!(
            taffy_style.gap.width,
            taffy::style::LengthPercentage::Points(0.)
        );
        assert_eq!(
            taffy_style.gap.height,
            taffy::style::LengthPercentage::Percent(0.)
        );
    }

    #[test]
    fn test_into_length_percentage() {
        use taffy::style::LengthPercentage;
        let context = LayoutContext::new(2.0, bevy_math::Vec2::new(800., 600.));
        let cases = [
            (Val::Auto, LengthPercentage::Points(0.)),
            (Val::Percent(1.), LengthPercentage::Percent(0.01)),
            (Val::Px(1.), LengthPercentage::Points(2.)),
            (Val::Vw(1.), LengthPercentage::Points(8.)),
            (Val::Vh(1.), LengthPercentage::Points(6.)),
            (Val::VMin(2.), LengthPercentage::Points(12.)),
            (Val::VMax(2.), LengthPercentage::Points(16.)),
        ];
        for (val, length) in cases {
            assert!(match (val.into_length_percentage(&context), length) {
                (LengthPercentage::Points(a), LengthPercentage::Points(b))
                | (LengthPercentage::Percent(a), LengthPercentage::Percent(b)) =>
                    (a - b).abs() < 0.0001,
                _ => false,
            },);
        }
    }
}
