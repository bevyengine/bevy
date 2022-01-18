use crate::{
    AlignContent, AlignItems, AlignSelf, Direction, Display, FlexDirection, FlexWrap,
    JustifyContent, PositionType, Style, Val,
};
use bevy_math::{Rect, Size};

pub fn from_rect(
    scale_factor: f64,
    rect: Rect<Val>,
) -> stretch::geometry::Rect<stretch::style::Dimension> {
    stretch::geometry::Rect {
        start: from_val(scale_factor, rect.left),
        end: from_val(scale_factor, rect.right),
        // NOTE: top and bottom are intentionally flipped. stretch has a flipped y-axis
        top: from_val(scale_factor, rect.bottom),
        bottom: from_val(scale_factor, rect.top),
    }
}

pub fn from_f32_size(scale_factor: f64, size: Size<f32>) -> stretch::geometry::Size<f32> {
    stretch::geometry::Size {
        width: (scale_factor * size.width as f64) as f32,
        height: (scale_factor * size.height as f64) as f32,
    }
}

pub fn from_val_size(
    scale_factor: f64,
    size: Size<Val>,
) -> stretch::geometry::Size<stretch::style::Dimension> {
    stretch::geometry::Size {
        width: from_val(scale_factor, size.width),
        height: from_val(scale_factor, size.height),
    }
}

pub fn from_style(scale_factor: f64, value: &Style) -> stretch::style::Style {
    stretch::style::Style {
        overflow: stretch::style::Overflow::Visible,
        display: value.display.into(),
        position_type: value.position_type.into(),
        direction: value.direction.into(),
        flex_direction: value.flex_direction.into(),
        flex_wrap: value.flex_wrap.into(),
        align_items: value.align_items.into(),
        align_self: value.align_self.into(),
        align_content: value.align_content.into(),
        justify_content: value.justify_content.into(),
        position: from_rect(scale_factor, value.position),
        margin: from_rect(scale_factor, value.margin),
        padding: from_rect(scale_factor, value.padding),
        border: from_rect(scale_factor, value.border),
        flex_grow: value.flex_grow,
        flex_shrink: value.flex_shrink,
        flex_basis: from_val(scale_factor, value.flex_basis),
        size: from_val_size(scale_factor, value.size),
        min_size: from_val_size(scale_factor, value.min_size),
        max_size: from_val_size(scale_factor, value.max_size),
        aspect_ratio: match value.aspect_ratio {
            Some(value) => stretch::number::Number::Defined(value),
            None => stretch::number::Number::Undefined,
        },
    }
}

pub fn from_val(scale_factor: f64, val: Val) -> stretch::style::Dimension {
    match val {
        Val::Auto => stretch::style::Dimension::Auto,
        Val::Percent(value) => stretch::style::Dimension::Percent(value / 100.0),
        Val::Px(value) => stretch::style::Dimension::Points((scale_factor * value as f64) as f32),
        Val::Undefined => stretch::style::Dimension::Undefined,
    }
}

impl From<AlignItems> for stretch::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::FlexStart => Self::FlexStart,
            AlignItems::FlexEnd => Self::FlexEnd,
            AlignItems::Center => Self::Center,
            AlignItems::Baseline => Self::Baseline,
            AlignItems::Stretch => Self::Stretch,
        }
    }
}

impl From<AlignSelf> for stretch::style::AlignSelf {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => Self::Auto,
            AlignSelf::FlexStart => Self::FlexStart,
            AlignSelf::FlexEnd => Self::FlexEnd,
            AlignSelf::Center => Self::Center,
            AlignSelf::Baseline => Self::Baseline,
            AlignSelf::Stretch => Self::Stretch,
        }
    }
}

impl From<AlignContent> for stretch::style::AlignContent {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::FlexStart => Self::FlexStart,
            AlignContent::FlexEnd => Self::FlexEnd,
            AlignContent::Center => Self::Center,
            AlignContent::Stretch => Self::Stretch,
            AlignContent::SpaceBetween => Self::SpaceBetween,
            AlignContent::SpaceAround => Self::SpaceAround,
        }
    }
}

impl From<Direction> for stretch::style::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Inherit => Self::Inherit,
            Direction::LeftToRight => Self::LTR,
            Direction::RightToLeft => Self::RTL,
        }
    }
}

impl From<Display> for stretch::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => Self::Flex,
            Display::None => Self::None,
        }
    }
}

impl From<FlexDirection> for stretch::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => Self::Row,
            FlexDirection::Column => Self::Column,
            FlexDirection::RowReverse => Self::RowReverse,
            FlexDirection::ColumnReverse => Self::ColumnReverse,
        }
    }
}

impl From<JustifyContent> for stretch::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::FlexStart => Self::FlexStart,
            JustifyContent::FlexEnd => Self::FlexEnd,
            JustifyContent::Center => Self::Center,
            JustifyContent::SpaceBetween => Self::SpaceBetween,
            JustifyContent::SpaceAround => Self::SpaceAround,
            JustifyContent::SpaceEvenly => Self::SpaceEvenly,
        }
    }
}

impl From<PositionType> for stretch::style::PositionType {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => Self::Relative,
            PositionType::Absolute => Self::Absolute,
        }
    }
}

impl From<FlexWrap> for stretch::style::FlexWrap {
    fn from(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => Self::NoWrap,
            FlexWrap::Wrap => Self::Wrap,
            FlexWrap::WrapReverse => Self::WrapReverse,
        }
    }
}
