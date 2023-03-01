use taffy::prelude::Rect;
use taffy::prelude::LengthPercentage;
use taffy::prelude::LengthPercentageAuto;
use taffy::style::Dimension;
use crate::Breadth;
use crate::JustifySelf;
use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    PositionType, Style, Val,
};

fn from_val(scale_factor: f64, val: Val) -> LengthPercentageAuto {
    match val {
        Val::Auto => LengthPercentageAuto::Auto,
        Val::Percent(value) => LengthPercentageAuto::Percent(value / 100.0),
        Val::Px(value) => LengthPercentageAuto::Points((scale_factor * value as f64) as f32),
    }
}

fn from_breadth(scale_factor: f64, breadth: Breadth) -> LengthPercentage {
    match breadth {
        Breadth::Percent(value) => LengthPercentage::Percent(value / 100.0),
        Breadth::Px(value) => LengthPercentage::Points((scale_factor * value as f64) as f32),
    }
}

fn dim_from_val(scale_factor: f64, val: Val) -> Dimension {
    match val {
        Val::Auto => Dimension::Auto,
        Val::Percent(value) => Dimension::Percent(value / 100.0),
        Val::Px(value) => Dimension::Points((scale_factor * value as f64) as f32),
    }
}

pub fn from_style(scale_factor: f64, style: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: style.display.into(),
        position: style.position_type.into(),
        inset: Rect {
            left: from_val(scale_factor, style.left),
            right: from_val(scale_factor, style.right),
            top: from_val(scale_factor, style.top),
            bottom: from_val(scale_factor, style.bottom),
        },
        flex_direction: style.flex_direction.into(),
        flex_wrap: style.flex_wrap.into(),
        align_items: Some(style.align_items.into()),
        align_self: style.align_self.into(),
        align_content: Some(style.align_content.into()),
        justify_content: Some(style.justify_content.into()),
        justify_self: style.justify_self.into(),  
        margin: Rect {
            left: from_val(scale_factor, style.margin.left),
            right: from_val(scale_factor, style.margin.right),
            top: from_val(scale_factor, style.margin.top),
            bottom: from_val(scale_factor, style.margin.bottom),
        },
        padding: Rect {
            left: from_breadth(scale_factor, style.padding.left),
            right: from_breadth(scale_factor, style.padding.right),
            top: from_breadth(scale_factor, style.padding.top),
            bottom: from_breadth(scale_factor, style.padding.bottom),
        },
        border: Rect {
            left: from_breadth(scale_factor, style.border.left),
            right: from_breadth(scale_factor, style.border.right),
            top: from_breadth(scale_factor, style.border.top),
            bottom: from_breadth(scale_factor, style.border.bottom),
        },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: dim_from_val(scale_factor, style.flex_basis),
        size: taffy::prelude::Size {
            width: dim_from_val(scale_factor, style.size.width),
            height: dim_from_val(scale_factor, style.size.height),
        },
        min_size: taffy::prelude::Size {
            width: dim_from_val(scale_factor, style.min_size.width),
            height: dim_from_val(scale_factor, style.min_size.height),
        },
        max_size: taffy::prelude::Size {
            width: dim_from_val(scale_factor, style.max_size.width),
            height: dim_from_val(scale_factor, style.max_size.height),
        },
        aspect_ratio: style.aspect_ratio,
        gap: taffy::prelude::Size {
            width: from_breadth(scale_factor, style.gap.width),
            height: from_breadth(scale_factor, style.gap.height),
        },
    }
}

impl From<AlignItems> for taffy::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::Start => taffy::style::AlignSelf::Start,
            AlignItems::End => taffy::style::AlignSelf::End,
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
            AlignSelf::Start => Some(taffy::style::AlignSelf::Start),
            AlignSelf::End => Some(taffy::style::AlignSelf::End),
            AlignSelf::FlexStart => Some(taffy::style::AlignSelf::FlexStart),
            AlignSelf::FlexEnd => Some(taffy::style::AlignSelf::FlexEnd),
            AlignSelf::Center => Some(taffy::style::AlignSelf::Center),
            AlignSelf::Baseline => Some(taffy::style::AlignSelf::Baseline),
            AlignSelf::Stretch => Some(taffy::style::AlignSelf::Stretch),
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

impl From<JustifySelf> for Option<taffy::style::AlignItems> {
    fn from(value: JustifySelf) -> Self {
        match value {
            JustifySelf::Auto => None,
            JustifySelf::Start => taffy::style::AlignItems::Start.into(),
            JustifySelf::End => taffy::style::AlignItems::End.into(),
            JustifySelf::FlexStart => taffy::style::AlignItems::FlexStart.into(),
            JustifySelf::FlexEnd => taffy::style::AlignItems::FlexEnd.into(),
            JustifySelf::Center => taffy::style::AlignItems::Center.into(),
            JustifySelf::Baseline => taffy::style::AlignItems::Baseline.into(),
            JustifySelf::Stretch => taffy::style::AlignItems::Stretch.into(),
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
