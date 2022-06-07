use crate::{
    AlignContent, AlignItems, AlignSelf, Direction, Display, FlexDirection, FlexWrap,
    JustifyContent, PositionType, Size, Style, UiRect, Val,
};

pub fn from_rect(
    scale_factor: f64,
    rect: UiRect<Val>,
) -> sprawl::geometry::Rect<sprawl::style::Dimension> {
    sprawl::geometry::Rect {
        start: from_val(scale_factor, rect.left),
        end: from_val(scale_factor, rect.right),
        // NOTE: top and bottom are intentionally flipped. stretch has a flipped y-axis
        top: from_val(scale_factor, rect.bottom),
        bottom: from_val(scale_factor, rect.top),
    }
}

pub fn from_f32_size(scale_factor: f64, size: Size<f32>) -> sprawl::geometry::Size<f32> {
    sprawl::geometry::Size {
        width: (scale_factor * size.width as f64) as f32,
        height: (scale_factor * size.height as f64) as f32,
    }
}

pub fn from_val_size(
    scale_factor: f64,
    size: Size<Val>,
) -> sprawl::geometry::Size<sprawl::style::Dimension> {
    sprawl::geometry::Size {
        width: from_val(scale_factor, size.width),
        height: from_val(scale_factor, size.height),
    }
}

pub fn from_style(scale_factor: f64, value: &Style) -> sprawl::style::Style {
    sprawl::style::Style {
        overflow: sprawl::style::Overflow::Visible,
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
            Some(value) => sprawl::number::Number::Defined(value),
            None => sprawl::number::Number::Undefined,
        },
    }
}

pub fn from_val(scale_factor: f64, val: Val) -> sprawl::style::Dimension {
    match val {
        Val::Auto => sprawl::style::Dimension::Auto,
        Val::Percent(value) => sprawl::style::Dimension::Percent(value / 100.0),
        Val::Px(value) => sprawl::style::Dimension::Points((scale_factor * value as f64) as f32),
        Val::Undefined => sprawl::style::Dimension::Undefined,
    }
}

impl From<AlignItems> for sprawl::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::FlexStart => sprawl::style::AlignItems::FlexStart,
            AlignItems::FlexEnd => sprawl::style::AlignItems::FlexEnd,
            AlignItems::Center => sprawl::style::AlignItems::Center,
            AlignItems::Baseline => sprawl::style::AlignItems::Baseline,
            AlignItems::Stretch => sprawl::style::AlignItems::Stretch,
        }
    }
}

impl From<AlignSelf> for sprawl::style::AlignSelf {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => sprawl::style::AlignSelf::Auto,
            AlignSelf::FlexStart => sprawl::style::AlignSelf::FlexStart,
            AlignSelf::FlexEnd => sprawl::style::AlignSelf::FlexEnd,
            AlignSelf::Center => sprawl::style::AlignSelf::Center,
            AlignSelf::Baseline => sprawl::style::AlignSelf::Baseline,
            AlignSelf::Stretch => sprawl::style::AlignSelf::Stretch,
        }
    }
}

impl From<AlignContent> for sprawl::style::AlignContent {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::FlexStart => sprawl::style::AlignContent::FlexStart,
            AlignContent::FlexEnd => sprawl::style::AlignContent::FlexEnd,
            AlignContent::Center => sprawl::style::AlignContent::Center,
            AlignContent::Stretch => sprawl::style::AlignContent::Stretch,
            AlignContent::SpaceBetween => sprawl::style::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => sprawl::style::AlignContent::SpaceAround,
        }
    }
}

impl From<Direction> for sprawl::style::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Inherit => sprawl::style::Direction::Inherit,
            Direction::LeftToRight => sprawl::style::Direction::LTR,
            Direction::RightToLeft => sprawl::style::Direction::RTL,
        }
    }
}

impl From<Display> for sprawl::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => sprawl::style::Display::Flex,
            Display::None => sprawl::style::Display::None,
        }
    }
}

impl From<FlexDirection> for sprawl::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => sprawl::style::FlexDirection::Row,
            FlexDirection::Column => sprawl::style::FlexDirection::Column,
            FlexDirection::RowReverse => sprawl::style::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => sprawl::style::FlexDirection::ColumnReverse,
        }
    }
}

impl From<JustifyContent> for sprawl::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::FlexStart => sprawl::style::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => sprawl::style::JustifyContent::FlexEnd,
            JustifyContent::Center => sprawl::style::JustifyContent::Center,
            JustifyContent::SpaceBetween => sprawl::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => sprawl::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => sprawl::style::JustifyContent::SpaceEvenly,
        }
    }
}

impl From<PositionType> for sprawl::style::PositionType {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => sprawl::style::PositionType::Relative,
            PositionType::Absolute => sprawl::style::PositionType::Absolute,
        }
    }
}

impl From<FlexWrap> for sprawl::style::FlexWrap {
    fn from(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => sprawl::style::FlexWrap::NoWrap,
            FlexWrap::Wrap => sprawl::style::FlexWrap::Wrap,
            FlexWrap::WrapReverse => sprawl::style::FlexWrap::WrapReverse,
        }
    }
}
