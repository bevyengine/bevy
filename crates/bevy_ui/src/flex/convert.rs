use crate::{
    AlignContent, AlignItems, AlignSelf, Direction, Display, FlexDirection, FlexWrap,
    JustifyContent, PositionType, Size, Style, UiRect, Val,
};

pub fn from_rect(
    scale_factor: f64,
    rect: UiRect<Val>,
) -> stretch2::geometry::Rect<stretch2::style::Dimension> {
    stretch2::geometry::Rect {
        start: from_val(scale_factor, rect.left),
        end: from_val(scale_factor, rect.right),
        // NOTE: top and bottom are intentionally flipped. stretch has a flipped y-axis
        top: from_val(scale_factor, rect.bottom),
        bottom: from_val(scale_factor, rect.top),
    }
}

pub fn from_f32_size(scale_factor: f64, size: Size<f32>) -> stretch2::geometry::Size<f32> {
    stretch2::geometry::Size {
        width: (scale_factor * size.width as f64) as f32,
        height: (scale_factor * size.height as f64) as f32,
    }
}

pub fn from_val_size(
    scale_factor: f64,
    size: Size<Val>,
) -> stretch2::geometry::Size<stretch2::style::Dimension> {
    stretch2::geometry::Size {
        width: from_val(scale_factor, size.width),
        height: from_val(scale_factor, size.height),
    }
}

pub fn from_style(scale_factor: f64, value: &Style) -> stretch2::style::Style {
    stretch2::style::Style {
        overflow: stretch2::style::Overflow::Visible,
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
            Some(value) => stretch2::number::Number::Defined(value),
            None => stretch2::number::Number::Undefined,
        },
    }
}

pub fn from_val(scale_factor: f64, val: Val) -> stretch2::style::Dimension {
    match val {
        Val::Auto => stretch2::style::Dimension::Auto,
        Val::Percent(value) => stretch2::style::Dimension::Percent(value / 100.0),
        Val::Px(value) => stretch2::style::Dimension::Points((scale_factor * value as f64) as f32),
        Val::Undefined => stretch2::style::Dimension::Undefined,
    }
}

impl From<AlignItems> for stretch2::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::FlexStart => stretch2::style::AlignItems::FlexStart,
            AlignItems::FlexEnd => stretch2::style::AlignItems::FlexEnd,
            AlignItems::Center => stretch2::style::AlignItems::Center,
            AlignItems::Baseline => stretch2::style::AlignItems::Baseline,
            AlignItems::Stretch => stretch2::style::AlignItems::Stretch,
        }
    }
}

impl From<AlignSelf> for stretch2::style::AlignSelf {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => stretch2::style::AlignSelf::Auto,
            AlignSelf::FlexStart => stretch2::style::AlignSelf::FlexStart,
            AlignSelf::FlexEnd => stretch2::style::AlignSelf::FlexEnd,
            AlignSelf::Center => stretch2::style::AlignSelf::Center,
            AlignSelf::Baseline => stretch2::style::AlignSelf::Baseline,
            AlignSelf::Stretch => stretch2::style::AlignSelf::Stretch,
        }
    }
}

impl From<AlignContent> for stretch2::style::AlignContent {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::FlexStart => stretch2::style::AlignContent::FlexStart,
            AlignContent::FlexEnd => stretch2::style::AlignContent::FlexEnd,
            AlignContent::Center => stretch2::style::AlignContent::Center,
            AlignContent::Stretch => stretch2::style::AlignContent::Stretch,
            AlignContent::SpaceBetween => stretch2::style::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => stretch2::style::AlignContent::SpaceAround,
        }
    }
}

impl From<Direction> for stretch2::style::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Inherit => stretch2::style::Direction::Inherit,
            Direction::LeftToRight => stretch2::style::Direction::LTR,
            Direction::RightToLeft => stretch2::style::Direction::RTL,
        }
    }
}

impl From<Display> for stretch2::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => stretch2::style::Display::Flex,
            Display::None => stretch2::style::Display::None,
        }
    }
}

impl From<FlexDirection> for stretch2::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => stretch2::style::FlexDirection::Row,
            FlexDirection::Column => stretch2::style::FlexDirection::Column,
            FlexDirection::RowReverse => stretch2::style::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => stretch2::style::FlexDirection::ColumnReverse,
        }
    }
}

impl From<JustifyContent> for stretch2::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::FlexStart => stretch2::style::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => stretch2::style::JustifyContent::FlexEnd,
            JustifyContent::Center => stretch2::style::JustifyContent::Center,
            JustifyContent::SpaceBetween => stretch2::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => stretch2::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => stretch2::style::JustifyContent::SpaceEvenly,
        }
    }
}

impl From<PositionType> for stretch2::style::PositionType {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => stretch2::style::PositionType::Relative,
            PositionType::Absolute => stretch2::style::PositionType::Absolute,
        }
    }
}

impl From<FlexWrap> for stretch2::style::FlexWrap {
    fn from(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => stretch2::style::FlexWrap::NoWrap,
            FlexWrap::Wrap => stretch2::style::FlexWrap::Wrap,
            FlexWrap::WrapReverse => stretch2::style::FlexWrap::WrapReverse,
        }
    }
}
