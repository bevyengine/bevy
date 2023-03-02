use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    PositionType, Size, Style, UiRect, Val,
};

pub fn from_style(scale_factor: f64, value: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: value.display.into(),
        position: value.position_type.into(),
        flex_direction: value.flex_direction.into(),
        flex_wrap: value.flex_wrap.into(),
        align_items: Some(value.align_items.into()),
        align_self: value.align_self.into(),
        align_content: Some(value.align_content.into()),
        justify_content: Some(value.justify_content.into()),
        inset: from_rect(scale_factor, value.position),
        margin: taffy::geometry::Rect {
            left: margin(scale_factor, value.margin.left),
            right: margin(scale_factor, value.margin.right),
            top: margin(scale_factor, value.margin.top),
            bottom: margin(scale_factor, value.margin.bottom),
        },
        padding: taffy::geometry::Rect {
            left: length_percent(scale_factor, value.padding.left),
            right: length_percent(scale_factor, value.padding.right),
            top: length_percent(scale_factor, value.padding.top),
            bottom: length_percent(scale_factor, value.padding.bottom),
        },
        border: taffy::geometry::Rect {
            left: length_percent(scale_factor, value.border.left),
            right: length_percent(scale_factor, value.border.right),
            top: length_percent(scale_factor, value.border.top),
            bottom: length_percent(scale_factor, value.border.bottom),
        },
        flex_grow: value.flex_grow,
        flex_shrink: value.flex_shrink,
        flex_basis: dimension(scale_factor, value.flex_basis),
        size: from_size(scale_factor, value.size),
        min_size: from_size(scale_factor, value.min_size),
        max_size: from_size(scale_factor, value.max_size),
        aspect_ratio: value.aspect_ratio,
        gap: taffy::geometry::Size {
            width: length_percent(scale_factor, value.gap.width),
            height: length_percent(scale_factor, value.gap.height),
        },
        justify_self: None,
    }
}

fn dimension(scale_factor: f64, val: Val) -> taffy::style::Dimension {
    match val {
        Val::Auto | Val::Undefined => taffy::style::Dimension::Auto,
        Val::Percent(value) => taffy::style::Dimension::Percent(value / 100.0),
        Val::Px(value) => taffy::style::Dimension::Points((scale_factor * value as f64) as f32),
    }
}

fn length_percent(scale_factor: f64, val: Val) -> taffy::style::LengthPercentage {
    match val {
        Val::Auto | Val::Undefined => taffy::style::LengthPercentage::Points(0.0),
        Val::Percent(value) => taffy::style::LengthPercentage::Percent(value / 100.0),
        Val::Px(value) => {
            taffy::style::LengthPercentage::Points((scale_factor * value as f64) as f32)
        }
    }
}

fn length_percent_auto(scale_factor: f64, val: Val) -> taffy::style::LengthPercentageAuto {
    match val {
        Val::Auto | Val::Undefined => taffy::style::LengthPercentageAuto::Auto,
        Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
        Val::Px(value) => {
            taffy::style::LengthPercentageAuto::Points((scale_factor * value as f64) as f32)
        }
    }
}

fn margin(scale_factor: f64, val: Val) -> taffy::style::LengthPercentageAuto {
    match val {
        Val::Auto => taffy::style::LengthPercentageAuto::Auto,
        Val::Undefined => taffy::style::LengthPercentageAuto::Points(0.),
        Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
        Val::Px(value) => {
            taffy::style::LengthPercentageAuto::Points((scale_factor * value as f64) as f32)
        }
    }
}

fn from_rect(
    scale_factor: f64,
    rect: UiRect,
) -> taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    taffy::geometry::Rect {
        left: length_percent_auto(scale_factor, rect.left),
        right: length_percent_auto(scale_factor, rect.right),
        top: length_percent_auto(scale_factor, rect.top),
        bottom: length_percent_auto(scale_factor, rect.bottom),
    }
}

fn from_size(scale_factor: f64, size: Size) -> taffy::geometry::Size<taffy::style::Dimension> {
    taffy::geometry::Size {
        width: dimension(scale_factor, size.width),
        height: dimension(scale_factor, size.height),
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
            JustifyContent::Stretch => taffy::style::JustifyContent::Stretch,
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
