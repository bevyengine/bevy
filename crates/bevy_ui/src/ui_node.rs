use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Size, Vec2};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_render::{
    color::Color,
    texture::{Image, DEFAULT_IMAGE_HANDLE},
};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign};

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct Node {
    pub size: Vec2,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Val {
    Undefined,
    Auto,
    Px(f32),
    Percent(f32),
}

impl Default for Val {
    fn default() -> Self {
        Val::Undefined
    }
}

impl Add<f32> for Val {
    type Output = Val;

    fn add(self, rhs: f32) -> Self::Output {
        match self {
            Val::Undefined => Val::Undefined,
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value + rhs),
            Val::Percent(value) => Val::Percent(value + rhs),
        }
    }
}

impl AddAssign<f32> for Val {
    fn add_assign(&mut self, rhs: f32) {
        match self {
            Val::Undefined | Val::Auto => {}
            Val::Px(value) => *value += rhs,
            Val::Percent(value) => *value += rhs,
        }
    }
}

#[derive(Component, Clone, PartialEq, Debug, Reflect)]
#[reflect(Component, PartialEq)]
pub struct Style {
    pub display: Display,
    pub position_type: PositionType,
    pub direction: Direction,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub align_content: AlignContent,
    pub justify_content: JustifyContent,
    pub position: Rect<Val>,
    pub margin: Rect<Val>,
    pub padding: Rect<Val>,
    pub border: Rect<Val>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Val,
    pub size: Size<Val>,
    pub min_size: Size<Val>,
    pub max_size: Size<Val>,
    pub aspect_ratio: Option<f32>,
    pub overflow: Overflow,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Default::default(),
            position_type: Default::default(),
            direction: Default::default(),
            flex_direction: Default::default(),
            flex_wrap: Default::default(),
            align_items: Default::default(),
            align_self: Default::default(),
            align_content: Default::default(),
            justify_content: Default::default(),
            position: Default::default(),
            margin: Default::default(),
            padding: Default::default(),
            border: Default::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Val::Auto,
            size: Size::new(Val::Auto, Val::Auto),
            min_size: Size::new(Val::Auto, Val::Auto),
            max_size: Size::new(Val::Auto, Val::Auto),
            aspect_ratio: Default::default(),
            overflow: Default::default(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

impl Default for AlignItems {
    fn default() -> AlignItems {
        AlignItems::Stretch
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum AlignSelf {
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

impl Default for AlignSelf {
    fn default() -> AlignSelf {
        AlignSelf::Auto
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum AlignContent {
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
}

impl Default for AlignContent {
    fn default() -> AlignContent {
        AlignContent::Stretch
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Inherit,
    LeftToRight,
    RightToLeft,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Inherit
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Display {
    Flex,
    None,
}

impl Default for Display {
    fn default() -> Display {
        Display::Flex
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum FlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl Default for FlexDirection {
    fn default() -> FlexDirection {
        FlexDirection::Row
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for JustifyContent {
    fn default() -> JustifyContent {
        JustifyContent::FlexStart
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Overflow {
    Visible,
    Hidden,
    // Scroll,
}

impl Default for Overflow {
    fn default() -> Overflow {
        Overflow::Visible
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum PositionType {
    Relative,
    Absolute,
}

impl Default for PositionType {
    fn default() -> PositionType {
        PositionType::Relative
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl Default for FlexWrap {
    fn default() -> FlexWrap {
        FlexWrap::NoWrap
    }
}

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedSize {
    pub size: Size,
}

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct UiColor(pub Color);

impl From<Color> for UiColor {
    fn from(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct UiImage(pub Handle<Image>);

impl Default for UiImage {
    fn default() -> Self {
        Self(DEFAULT_IMAGE_HANDLE.typed())
    }
}

impl From<Handle<Image>> for UiImage {
    fn from(handle: Handle<Image>) -> Self {
        Self(handle)
    }
}

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedClip {
    pub clip: bevy_sprite::Rect,
}
