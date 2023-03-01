use taffy::prelude::Rect;
use taffy::prelude::LengthPercentage;
use taffy::prelude::LengthPercentageAuto;
use taffy::style::Dimension;
use crate::Breadth;
use crate::JustifySelf;
use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    PositionType, Size, Style, UiRect, Val,
};

trait IntoScaled {
    type Output;
    fn into_scaled(self, scalar: f64) -> Self::Output;
}

impl IntoScaled for Val {
    type Output = LengthPercentageAuto;
    fn into_scaled(self: Val, scalar: f64) -> LengthPercentageAuto {
        match self {
            Val::Auto => LengthPercentageAuto::Auto,
            Val::Percent(value) => LengthPercentageAuto::Percent(value / 100.0),
            Val::Px(value) => LengthPercentageAuto::Points((scalar * value as f64) as f32),
        }
    }
}

impl IntoScaled for Breadth {
    type Output = LengthPercentage;
    fn into_scaled(self, scalar: f64) -> LengthPercentage {
        match self {
            Breadth::Percent(value) => LengthPercentage::Percent(value / 100.0),
            Breadth::Px(value) => LengthPercentage::Points((scalar * value as f64) as f32),
        }
    }
}

fn from_val(scale_factor: f64, val: Val) -> Dimension {
    match val {
        Val::Auto => Dimension::Auto,
        Val::Percent(value) => Dimension::Percent(value / 100.0),
        Val::Px(value) => Dimension::Points((scale_factor * value as f64) as f32),
    }
}

impl IntoScaled for Size {
    type Output = taffy::geometry::Size<Dimension>;
    fn into_scaled(self, scalar: f64) -> taffy::geometry::Size<Dimension> {
        taffy::geometry::Size::<Dimension> {
            width: from_val(scalar, self.width),
            height:from_val(scalar, self.height),
        }
    }
}

impl IntoScaled for UiRect<Breadth> {
    type Output = taffy::geometry::Rect<LengthPercentage>;
    fn into_scaled(self, scalar: f64) -> taffy::geometry::Rect<LengthPercentage> {
        taffy::geometry::Rect::<LengthPercentage> {
            left: self.left.into_scaled(scalar),
            right: self.right.into_scaled(scalar),
            top: self.top.into_scaled(scalar),
            bottom: self.bottom.into_scaled(scalar),
        }
    }
}

impl IntoScaled for UiRect<Val> {
    type Output = taffy::geometry::Rect<LengthPercentageAuto>;
    fn into_scaled(self, scalar: f64) -> taffy::geometry::Rect<LengthPercentageAuto> {
        taffy::geometry::Rect::<LengthPercentageAuto> {
            left: self.left.into_scaled(scalar),
            right: self.right.into_scaled(scalar),
            top: self.top.into_scaled(scalar),
            bottom: self.bottom.into_scaled(scalar),
        }
    }
}

fn from_breadth_size(
    scale_factor: f64,
    size: Size<Breadth>,
) -> taffy::geometry::Size<taffy::style::LengthPercentage> {
    taffy::geometry::Size::<taffy::style::LengthPercentage> {
        width: size.width.into_scaled(scale_factor),
        height: size.height.into_scaled(scale_factor),
    }
}

pub fn from_style(scale_factor: f64, style: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: style.display.into(),
        position: style.position_type.into(),
        inset: Rect {
            left: style.left.into_scaled(scale_factor),
            right: style.right.into_scaled(scale_factor),
            top: style.top.into_scaled(scale_factor),
            bottom: style.bottom.into_scaled(scale_factor),
        },
        flex_direction: style.flex_direction.into(),
        flex_wrap: style.flex_wrap.into(),
        align_items: Some(style.align_items.into()),
        align_self: style.align_self.into(),
        align_content: Some(style.align_content.into()),
        justify_content: Some(style.justify_content.into()),
        justify_self: style.justify_self.into(),  
        margin: style.margin.into_scaled(scale_factor),
        padding: style.padding.into_scaled(scale_factor),
        border: style.border.into_scaled(scale_factor),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: from_val(scale_factor, style.flex_basis),
        size: style.size.into_scaled(scale_factor),
        min_size: style.min_size.into_scaled(scale_factor),
        max_size: style.max_size.into_scaled(scale_factor),
        aspect_ratio: style.aspect_ratio,
        gap: from_breadth_size(scale_factor, style.gap),        
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
