use crate::{
    layout_components::{
        flex::{
            AlignContent, AlignItems, AlignSelf, Direction, FlexLayoutQueryItem, JustifyContent,
            Wrap,
        },
        Position,
    },
    Display,
};
use crate::{Size, UiRect, Val};

pub fn from_rect(
    scale_factor: f64,
    rect: UiRect,
) -> taffy::geometry::Rect<taffy::style::Dimension> {
    taffy::geometry::Rect {
        left: from_val(scale_factor, rect.left),
        right: from_val(scale_factor, rect.right),
        top: from_val(scale_factor, rect.top),
        bottom: from_val(scale_factor, rect.bottom),
    }
}

pub fn from_f32_size(scale_factor: f64, size: Size) -> taffy::geometry::Size<f32> {
    taffy::geometry::Size {
        width: val_to_f32(scale_factor, size.width),
        height: val_to_f32(scale_factor, size.height),
    }
}

pub fn from_val_size(
    scale_factor: f64,
    size: Size,
) -> taffy::geometry::Size<taffy::style::Dimension> {
    taffy::geometry::Size {
        width: from_val(scale_factor, size.width),
        height: from_val(scale_factor, size.height),
    }
}

pub fn from_flex_layout(scale_factor: f64, value: FlexLayoutQueryItem<'_>) -> taffy::style::Style {
    taffy::style::Style {
        display: (value.control.display).into(),
        position_type: (value.control.position).into(),
        flex_direction: value.layout.direction.into(),
        flex_wrap: value.layout.wrap.into(),
        align_items: value.layout.align_items.into(),
        align_self: value.child_layout.align_self.into(),
        align_content: value.layout.align_content.into(),
        justify_content: value.layout.justify_content.into(),
        position: from_rect(scale_factor, value.control.inset.0),
        margin: from_rect(scale_factor, value.spacing.margin),
        padding: from_rect(scale_factor, value.spacing.padding),
        border: from_rect(scale_factor, value.spacing.border),
        flex_grow: value.child_layout.grow,
        flex_shrink: value.child_layout.shrink,
        flex_basis: from_val(scale_factor, value.child_layout.basis),
        size: from_val_size(scale_factor, value.size_constraints.suggested),
        min_size: from_val_size(scale_factor, value.size_constraints.min),
        max_size: from_val_size(scale_factor, value.size_constraints.max),
        aspect_ratio: value.size_constraints.aspect_ratio,
        gap: from_val_size(scale_factor, value.layout.gap),
    }
}

/// Converts a [`Val`] to a [`f32`] while respecting the scale factor.
pub fn val_to_f32(scale_factor: f64, val: Val) -> f32 {
    match val {
        Val::Undefined | Val::Auto => 0.0,
        Val::Px(value) => (scale_factor * value as f64) as f32,
        Val::Percent(value) => value / 100.0,
    }
}

pub fn from_val(scale_factor: f64, val: Val) -> taffy::style::Dimension {
    match val {
        Val::Auto => taffy::style::Dimension::Auto,
        Val::Percent(value) => taffy::style::Dimension::Percent(value / 100.0),
        Val::Px(value) => taffy::style::Dimension::Points((scale_factor * value as f64) as f32),
        Val::Undefined => taffy::style::Dimension::Undefined,
    }
}

impl From<AlignItems> for taffy::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::FlexStart => taffy::style::AlignItems::FlexStart,
            AlignItems::FlexEnd => taffy::style::AlignItems::FlexEnd,
            AlignItems::Center => taffy::style::AlignItems::Center,
            AlignItems::Baseline => taffy::style::AlignItems::Baseline,
            AlignItems::Stretch => taffy::style::AlignItems::Stretch,
        }
    }
}

impl From<AlignSelf> for taffy::style::AlignSelf {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => taffy::style::AlignSelf::Auto,
            AlignSelf::FlexStart => taffy::style::AlignSelf::FlexStart,
            AlignSelf::FlexEnd => taffy::style::AlignSelf::FlexEnd,
            AlignSelf::Center => taffy::style::AlignSelf::Center,
            AlignSelf::Baseline => taffy::style::AlignSelf::Baseline,
            AlignSelf::Stretch => taffy::style::AlignSelf::Stretch,
        }
    }
}

impl From<AlignContent> for taffy::style::AlignContent {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::FlexStart => taffy::style::AlignContent::FlexStart,
            AlignContent::FlexEnd => taffy::style::AlignContent::FlexEnd,
            AlignContent::Center => taffy::style::AlignContent::Center,
            AlignContent::Stretch => taffy::style::AlignContent::Stretch,
            AlignContent::SpaceBetween => taffy::style::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => taffy::style::AlignContent::SpaceAround,
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

impl From<Direction> for taffy::style::FlexDirection {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Row => taffy::style::FlexDirection::Row,
            Direction::Column => taffy::style::FlexDirection::Column,
            Direction::RowReverse => taffy::style::FlexDirection::RowReverse,
            Direction::ColumnReverse => taffy::style::FlexDirection::ColumnReverse,
        }
    }
}

impl From<JustifyContent> for taffy::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::FlexStart => taffy::style::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::style::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::style::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::style::JustifyContent::SpaceEvenly,
        }
    }
}

impl From<Position> for taffy::style::PositionType {
    fn from(value: Position) -> Self {
        match value {
            Position::Relative => taffy::style::PositionType::Relative,
            Position::Absolute => taffy::style::PositionType::Absolute,
        }
    }
}

impl From<Wrap> for taffy::style::FlexWrap {
    fn from(value: Wrap) -> Self {
        match value {
            Wrap::NoWrap => taffy::style::FlexWrap::NoWrap,
            Wrap::Wrap => taffy::style::FlexWrap::Wrap,
            Wrap::WrapReverse => taffy::style::FlexWrap::WrapReverse,
        }
    }
}
