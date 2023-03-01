use taffy::prelude::Rect;
use taffy::prelude::LengthPercentage;
use taffy::prelude::LengthPercentageAuto;
use taffy::style::Dimension;
use crate::JustifySelf;
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

pub fn lpa_from_rect(
    scale_factor: f64,
    rect: UiRect,
) -> taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    taffy::geometry::Rect {
        left: lpa_from_val(scale_factor, rect.left),
        right: lpa_from_val(scale_factor, rect.right),
        top: lpa_from_val(scale_factor, rect.top),
        bottom: lpa_from_val(scale_factor, rect.bottom),
    }
}

pub fn lp_from_rect(
    scale_factor: f64,
    rect: UiRect,
) -> taffy::geometry::Rect<taffy::style::LengthPercentage> {
    taffy::geometry::Rect {
        left: lp_from_val(scale_factor, rect.left),
        right: lp_from_val(scale_factor, rect.right),
        top: lp_from_val(scale_factor, rect.top),
        bottom: lp_from_val(scale_factor, rect.bottom),
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

pub fn lp_from_val_size(
    scale_factor: f64,
    size: Size,
) -> taffy::geometry::Size<taffy::style::LengthPercentage> {
    taffy::geometry::Size {
        width: lp_from_val(scale_factor, size.width),
        height: lp_from_val(scale_factor, size.height),
    }
}



pub fn from_style(scale_factor: f64, value: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: value.display.into(),
        position: value.position_type.into(),
        inset: Rect {
            left: lpa_from_val(scale_factor, value.left),
            right: lpa_from_val(scale_factor, value.right),
            top: lpa_from_val(scale_factor, value.top),
            bottom: lpa_from_val(scale_factor, value.bottom),
        },
        flex_direction: value.flex_direction.into(),
        flex_wrap: value.flex_wrap.into(),
        align_items: Some(value.align_items.into()),
        align_self: value.align_self.into(),
        align_content: Some(value.align_content.into()),
        justify_content: Some(value.justify_content.into()),
        justify_self: value.justify_self.into(),  
        margin: lpa_from_rect(scale_factor, value.margin),
        padding: lp_from_rect(scale_factor, value.padding),
        border: lp_from_rect(scale_factor, value.border),
        flex_grow: value.flex_grow,
        flex_shrink: value.flex_shrink,
        flex_basis: from_val(scale_factor, value.flex_basis),
        size: from_val_size(scale_factor, value.size),
        min_size: from_val_size(scale_factor, value.min_size),
        max_size: from_val_size(scale_factor, value.max_size),
        aspect_ratio: value.aspect_ratio,
        gap: lp_from_val_size(scale_factor, value.gap),        
    }
}

fn lp_from_val(scale_factor: f64, val: Val) -> LengthPercentage {
    match val {
        Val::Auto => LengthPercentage::Points(0.),
        Val::Percent(value) => LengthPercentage::Percent(value / 100.0),
        Val::Px(value) => LengthPercentage::Points((scale_factor * value as f64) as f32),
    }
}

fn from_val(scale_factor: f64, val: Val) -> Dimension {
    match val {
        Val::Auto => Dimension::Auto,
        Val::Percent(value) => Dimension::Percent(value / 100.0),
        Val::Px(value) => Dimension::Points((scale_factor * value as f64) as f32),
    }
}

fn lpa_from_val(scale_factor: f64, val: Val) -> LengthPercentageAuto {
    match val {
        Val::Auto => LengthPercentageAuto::Auto,
        Val::Percent(value) => LengthPercentageAuto::Percent(value / 100.0),
        Val::Px(value) => LengthPercentageAuto::Points((scale_factor * value as f64) as f32),
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
