use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    PositionType, Size, Style, UiRect, Val,
};

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

pub fn from_val_size(
    scale_factor: f64,
    size: Size,
) -> taffy::geometry::Size<taffy::style::Dimension> {
    taffy::geometry::Size {
        width: from_val(scale_factor, size.width),
        height: from_val(scale_factor, size.height),
    }
}

pub fn from_style(scale_factor: f64, value: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: value.display.into(),
        position_type: value.position_type.into(),
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
        aspect_ratio: value.aspect_ratio,
        gap: from_val_size(scale_factor, value.gap),
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
            JustifyContent::FlexStart => taffy::style::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::style::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::style::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::style::JustifyContent::SpaceEvenly,
        }
    }
}

impl From<PositionType> for taffy::style::PositionType {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => taffy::style::PositionType::Relative,
            PositionType::Absolute => taffy::style::PositionType::Absolute,
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
