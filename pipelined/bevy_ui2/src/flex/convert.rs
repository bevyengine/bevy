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
            AlignItems::FlexStart => stretch::style::AlignItems::FlexStart,
            AlignItems::FlexEnd => stretch::style::AlignItems::FlexEnd,
            AlignItems::Center => stretch::style::AlignItems::Center,
            AlignItems::Baseline => stretch::style::AlignItems::Baseline,
            AlignItems::Stretch => stretch::style::AlignItems::Stretch,
        }
    }
}

impl From<AlignSelf> for stretch::style::AlignSelf {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => stretch::style::AlignSelf::Auto,
            AlignSelf::FlexStart => stretch::style::AlignSelf::FlexStart,
            AlignSelf::FlexEnd => stretch::style::AlignSelf::FlexEnd,
            AlignSelf::Center => stretch::style::AlignSelf::Center,
            AlignSelf::Baseline => stretch::style::AlignSelf::Baseline,
            AlignSelf::Stretch => stretch::style::AlignSelf::Stretch,
        }
    }
}

impl From<AlignContent> for stretch::style::AlignContent {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::FlexStart => stretch::style::AlignContent::FlexStart,
            AlignContent::FlexEnd => stretch::style::AlignContent::FlexEnd,
            AlignContent::Center => stretch::style::AlignContent::Center,
            AlignContent::Stretch => stretch::style::AlignContent::Stretch,
            AlignContent::SpaceBetween => stretch::style::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => stretch::style::AlignContent::SpaceAround,
        }
    }
}

impl From<Direction> for stretch::style::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Inherit => stretch::style::Direction::Inherit,
            Direction::LeftToRight => stretch::style::Direction::LTR,
            Direction::RightToLeft => stretch::style::Direction::RTL,
        }
    }
}

impl From<Display> for stretch::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => stretch::style::Display::Flex,
            Display::None => stretch::style::Display::None,
        }
    }
}

impl From<FlexDirection> for stretch::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => stretch::style::FlexDirection::Row,
            FlexDirection::Column => stretch::style::FlexDirection::Column,
            FlexDirection::RowReverse => stretch::style::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => stretch::style::FlexDirection::ColumnReverse,
        }
    }
}

impl From<JustifyContent> for stretch::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::FlexStart => stretch::style::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => stretch::style::JustifyContent::FlexEnd,
            JustifyContent::Center => stretch::style::JustifyContent::Center,
            JustifyContent::SpaceBetween => stretch::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => stretch::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => stretch::style::JustifyContent::SpaceEvenly,
        }
    }
}

impl From<PositionType> for stretch::style::PositionType {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => stretch::style::PositionType::Relative,
            PositionType::Absolute => stretch::style::PositionType::Absolute,
        }
    }
}

impl From<FlexWrap> for stretch::style::FlexWrap {
    fn from(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => stretch::style::FlexWrap::NoWrap,
            FlexWrap::Wrap => stretch::style::FlexWrap::Wrap,
            FlexWrap::WrapReverse => stretch::style::FlexWrap::WrapReverse,
        }
    }
}
