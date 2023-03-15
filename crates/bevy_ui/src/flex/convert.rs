use crate::{
    AlignContent, AlignItems, AlignSelf, AutoVal, Border, Display, FlexDirection, FlexWrap,
    JustifyContent, Margin, Padding, PositionType, Size, Style, Val,
};

impl Val {
    fn scaled(self, scale_factor: f64) -> Self {
        match self {
            Val::Percent(value) => Val::Percent(value),
            Val::Px(value) => Val::Px((scale_factor * value as f64) as f32),
        }
    }
    fn into_length_percentage(self, scale_factor: f64) -> taffy::style::LengthPercentage {
        match self.scaled(scale_factor) {
            Val::Percent(value) => taffy::style::LengthPercentage::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentage::Points(value),
        }
    }
}

impl AutoVal {
    fn scaled(self, scale_factor: f64) -> Self {
        match self {
            AutoVal::Auto => AutoVal::Auto,
            AutoVal::Percent(value) => AutoVal::Percent(value),
            AutoVal::Px(value) => AutoVal::Px((scale_factor * value as f64) as f32),
        }
    }
    fn into_dimension(self, scale_factor: f64) -> taffy::style::Dimension {
        match self.scaled(scale_factor) {
            AutoVal::Auto => taffy::style::Dimension::Auto,
            AutoVal::Percent(value) => taffy::style::Dimension::Percent(value / 100.0),
            AutoVal::Px(value) => taffy::style::Dimension::Points(value),
        }
    }
    fn into_length_percentage(self, scale_factor: f64) -> taffy::style::LengthPercentage {
        match self.scaled(scale_factor) {
            AutoVal::Auto => taffy::style::LengthPercentage::Points(0.0),
            AutoVal::Percent(value) => taffy::style::LengthPercentage::Percent(value / 100.0),
            AutoVal::Px(value) => taffy::style::LengthPercentage::Points(value),
        }
    }
    fn into_length_percentage_auto(self, scale_factor: f64) -> taffy::style::LengthPercentageAuto {
        match self.scaled(scale_factor) {
            AutoVal::Auto => taffy::style::LengthPercentageAuto::Auto,
            AutoVal::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
            AutoVal::Px(value) => taffy::style::LengthPercentageAuto::Points(value),
        }
    }
}

macro_rules! impl_map_to_taffy_rect {
    ($t:ty, $v:ty) => {
        impl $t {
            fn map_to_taffy_rect<T>(self, map_fn: impl Fn($v) -> T) -> taffy::geometry::Rect<T> {
                taffy::geometry::Rect {
                    left: map_fn(self.left),
                    right: map_fn(self.right),
                    top: map_fn(self.top),
                    bottom: map_fn(self.bottom),
                }
            }
        }
    };
}

impl_map_to_taffy_rect!(Margin, AutoVal);
impl_map_to_taffy_rect!(Border, Val);
impl_map_to_taffy_rect!(Padding, Val);

impl Size {
    fn map_to_taffy_size<T>(self, map_fn: impl Fn(AutoVal) -> T) -> taffy::geometry::Size<T> {
        taffy::geometry::Size {
            width: map_fn(self.width),
            height: map_fn(self.height),
        }
    }
}

pub fn from_style(scale_factor: f64, style: &Style) -> taffy::style::Style {
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
            left: style.left.into_length_percentage_auto(scale_factor),
            right: style.right.into_length_percentage_auto(scale_factor),
            top: style.top.into_length_percentage_auto(scale_factor),
            bottom: style.bottom.into_length_percentage_auto(scale_factor),
        },
        margin: style
            .margin
            .map_to_taffy_rect(|m| m.into_length_percentage_auto(scale_factor)),
        padding: style
            .padding
            .map_to_taffy_rect(|p| p.into_length_percentage(scale_factor)),
        border: style
            .border
            .map_to_taffy_rect(|b| b.into_length_percentage(scale_factor)),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis.into_dimension(scale_factor),
        size: style
            .size
            .map_to_taffy_size(|s| s.into_dimension(scale_factor)),
        min_size: style
            .min_size
            .map_to_taffy_size(|s| s.into_dimension(scale_factor)),
        max_size: style
            .max_size
            .map_to_taffy_size(|s| s.into_dimension(scale_factor)),
        aspect_ratio: style.aspect_ratio,
        gap: style
            .gap
            .map_to_taffy_size(|s| s.into_length_percentage(scale_factor)),
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
            left: AutoVal::Px(0.),
            right: AutoVal::Percent(0.),
            top: AutoVal::Auto,
            bottom: AutoVal::Auto,
            direction: crate::Direction::Inherit,
            flex_direction: FlexDirection::ColumnReverse,
            flex_wrap: FlexWrap::WrapReverse,
            align_items: AlignItems::Baseline,
            align_self: AlignSelf::Start,
            align_content: AlignContent::SpaceAround,
            justify_content: JustifyContent::SpaceEvenly,
            margin: Margin {
                left: AutoVal::Percent(0.),
                right: AutoVal::Px(0.),
                top: AutoVal::Auto,
                bottom: AutoVal::Auto,
            },
            padding: Padding {
                left: Val::Percent(0.),
                right: Val::Px(0.),
                top: Val::Percent(0.),
                bottom: Val::Percent(0.),
            },
            border: Border {
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
            },
            flex_grow: 1.,
            flex_shrink: 0.,
            flex_basis: AutoVal::Px(0.),
            size: Size {
                width: AutoVal::Px(0.),
                height: AutoVal::Auto,
            },
            min_size: Size {
                width: AutoVal::Px(0.),
                height: AutoVal::Percent(0.),
            },
            max_size: Size {
                width: AutoVal::Auto,
                height: AutoVal::Px(0.),
            },
            aspect_ratio: None,
            overflow: crate::Overflow::Hidden,
            gap: Size {
                width: AutoVal::Px(0.),
                height: AutoVal::Percent(0.),
            },
        };
        let taffy_style = from_style(1.0, &bevy_style);
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
}
