use taffy::style::LengthPercentageAuto;

use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    PositionType, Size, Style, UiRect, Val,
};

impl Val {
    fn scaled(self, scale_factor: f64) -> Self {
        match self {
            Val::Auto => Val::Auto,
            Val::Percent(value) => Val::Percent(value),
            Val::Px(value) => Val::Px((scale_factor * value as f64) as f32),
            Val::Undefined => Val::Undefined,
        }
    }

    fn to_inset(self) -> LengthPercentageAuto {
        match self {
            Val::Auto | Val::Undefined => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentageAuto::Points(value),
        }
    }
}

impl UiRect {
    fn scaled(self, scale_factor: f64) -> Self {
        Self {
            left: self.left.scaled(scale_factor),
            right: self.right.scaled(scale_factor),
            top: self.top.scaled(scale_factor),
            bottom: self.bottom.scaled(scale_factor),
        }
    }
}

impl Size {
    fn scaled(self, scale_factor: f64) -> Self {
        Self {
            width: self.width.scaled(scale_factor),
            height: self.height.scaled(scale_factor),
        }
    }
}

impl<T: From<Val>> From<UiRect> for taffy::prelude::Rect<T> {
    fn from(value: UiRect) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
            top: value.top.into(),
            bottom: value.bottom.into(),
        }
    }
}

impl<T: From<Val>> From<Size> for taffy::prelude::Size<T> {
    fn from(value: Size) -> Self {
        Self {
            width: value.width.into(),
            height: value.height.into(),
        }
    }
}

impl From<Val> for taffy::style::Dimension {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto | Val::Undefined => taffy::style::Dimension::Auto,
            Val::Percent(value) => taffy::style::Dimension::Percent(value / 100.0),
            Val::Px(value) => taffy::style::Dimension::Points(value),
        }
    }
}

impl From<Val> for taffy::style::LengthPercentage {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto | Val::Undefined => taffy::style::LengthPercentage::Points(0.0),
            Val::Percent(value) => taffy::style::LengthPercentage::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentage::Points(value),
        }
    }
}

impl From<Val> for taffy::style::LengthPercentageAuto {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentageAuto::Points(value),
            Val::Undefined => taffy::style::LengthPercentageAuto::Points(0.),
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
            left: style.position.left.scaled(scale_factor).to_inset(),
            right: style.position.right.scaled(scale_factor).to_inset(),
            top: style.position.top.scaled(scale_factor).to_inset(),
            bottom: style.position.bottom.scaled(scale_factor).to_inset(),
        },
        margin: style.margin.scaled(scale_factor).into(),
        padding: style.padding.scaled(scale_factor).into(),
        border: style.border.scaled(scale_factor).into(),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis.scaled(scale_factor).into(),
        size: style.size.scaled(scale_factor).into(),
        min_size: style.min_size.scaled(scale_factor).into(),
        max_size: style.max_size.scaled(scale_factor).into(),
        aspect_ratio: style.aspect_ratio,
        gap: style.gap.scaled(scale_factor).into(),
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
