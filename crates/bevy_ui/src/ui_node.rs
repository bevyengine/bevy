use crate::{Size, UiRect};
use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use bevy_render::{
    color::Color,
    texture::{Image, DEFAULT_IMAGE_HANDLE},
};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign};

/// Describes the size of a UI node
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Node {
    /// The size of the node as width and height in pixels
    pub size: Vec2,
}

/// An enum that describes possible types of value in flexbox layout options
#[derive(Copy, Clone, PartialEq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Val {
    /// No value defined
    #[default]
    Undefined,
    /// Automatically determine this value
    Auto,
    /// Set this value in pixels
    Px(f32),
    /// Set this value in percent
    Percent(f32),
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
            Val::Px(value) | Val::Percent(value) => *value += rhs,
        }
    }
}

/// Describes the style of a UI node
///
/// It uses the [Flexbox](https://cssreference.io/flexbox/) system.
///
/// **Note:** Bevy's UI is upside down compared to how Flexbox normally works, to stay consistent with engine paradigms about layouting from
/// the upper left corner of the display
#[derive(Component, Clone, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct Style {
    /// Whether to arrange this node and its children with flexbox layout
    ///
    /// If this is set to [`Display::None`], this node will be collapsed.
    pub display: Display,
    /// Whether to arrange this node relative to other nodes, or positioned absolutely
    pub position_type: PositionType,
    /// Which direction the content of this node should go
    pub direction: Direction,
    /// Whether to use column or row layout
    pub flex_direction: FlexDirection,
    /// How to wrap nodes
    pub flex_wrap: FlexWrap,
    /// How items are aligned according to the cross axis
    pub align_items: AlignItems,
    /// Like align_items but for only this item
    pub align_self: AlignSelf,
    /// How to align each line, only applies if flex_wrap is set to
    /// [`FlexWrap::Wrap`] and there are multiple lines of items
    pub align_content: AlignContent,
    /// How items align according to the main axis
    pub justify_content: JustifyContent,
    /// The position of the node as descrided by its Rect
    pub position: UiRect<Val>,
    /// The margin of the node
    pub margin: UiRect<Val>,
    /// The padding of the node
    pub padding: UiRect<Val>,
    /// The border of the node
    pub border: UiRect<Val>,
    /// Defines how much a flexbox item should grow if there's space available
    pub flex_grow: f32,
    /// How to shrink if there's not enough space available
    pub flex_shrink: f32,
    /// The initial size of the item
    pub flex_basis: Val,
    /// The size of the flexbox
    pub size: Size<Val>,
    /// The minimum size of the flexbox
    pub min_size: Size<Val>,
    /// The maximum size of the flexbox
    pub max_size: Size<Val>,
    /// The aspect ratio of the flexbox
    pub aspect_ratio: Option<f32>,
    /// How to handle overflow
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

/// How items are aligned according to the cross axis
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum AlignItems {
    /// Items are aligned at the start
    FlexStart,
    /// Items are aligned at the end
    FlexEnd,
    /// Items are aligned at the center
    Center,
    /// Items are aligned at the baseline
    Baseline,
    /// Items are stretched across the whole cross axis
    #[default]
    Stretch,
}

/// Works like [`AlignItems`] but applies only to a single item
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum AlignSelf {
    /// Use the value of [`AlignItems`]
    #[default]
    Auto,
    /// If the parent has [`AlignItems::Center`] only this item will be at the start
    FlexStart,
    /// If the parent has [`AlignItems::Center`] only this item will be at the end
    FlexEnd,
    /// If the parent has [`AlignItems::FlexStart`] only this item will be at the center
    Center,
    /// If the parent has [`AlignItems::Center`] only this item will be at the baseline
    Baseline,
    /// If the parent has [`AlignItems::Center`] only this item will stretch along the whole cross axis
    Stretch,
}

/// Defines how each line is aligned within the flexbox.
///
/// It only applies if [`FlexWrap::Wrap`] is present and if there are multiple lines of items.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum AlignContent {
    /// Each line moves towards the start of the cross axis
    FlexStart,
    /// Each line moves towards the end of the cross axis
    FlexEnd,
    /// Each line moves towards the center of the cross axis
    Center,
    /// Each line will stretch to fill the remaining space
    #[default]
    Stretch,
    /// Each line fills the space it needs, putting the remaining space, if any
    /// inbetween the lines
    SpaceBetween,
    /// Each line fills the space it needs, putting the remaining space, if any
    /// around the lines
    SpaceAround,
}

/// Defines the text direction
///
/// For example English is written LTR (left-to-right) while Arabic is written RTL (right-to-left).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Direction {
    /// Inherit from parent node
    #[default]
    Inherit,
    /// Text is written left to right
    LeftToRight,
    /// Text is written right to left
    RightToLeft,
}

/// Whether to use a Flexbox layout model.
///
/// Part of the [`Style`] component.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Display {
    /// Use Flexbox layout model to determine the position of this [`Node`].
    #[default]
    Flex,
    /// Use no layout, don't render this node and its children.
    ///
    /// If you want to hide a node and its children,
    /// but keep its layout in place, set its [`Visibility`](bevy_render::view::Visibility) component instead.
    None,
}

/// Defines how flexbox items are ordered within a flexbox
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum FlexDirection {
    /// Same way as text direction along the main axis
    #[default]
    Row,
    /// Flex from bottom to top
    Column,
    /// Opposite way as text direction along the main axis
    RowReverse,
    /// Flex from top to bottom
    ColumnReverse,
}

/// Defines how items are aligned according to the main axis
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum JustifyContent {
    /// Pushed towards the start
    #[default]
    FlexStart,
    /// Pushed towards the end
    FlexEnd,
    /// Centered along the main axis
    Center,
    /// Remaining space is distributed between the items
    SpaceBetween,
    /// Remaining space is distributed around the items
    SpaceAround,
    /// Like [`JustifyContent::SpaceAround`] but with even spacing between items
    SpaceEvenly,
}

/// Whether to show or hide overflowing items
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Overflow {
    /// Show overflowing items
    #[default]
    Visible,
    /// Hide overflowing items
    Hidden,
}

/// The strategy used to position this node
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum PositionType {
    /// Relative to all other nodes with the [`PositionType::Relative`] value
    #[default]
    Relative,
    /// Independent of all other nodes
    ///
    /// As usual, the `Style.position` field of this node is specified relative to its parent node
    Absolute,
}

/// Defines if flexbox items appear on a single line or on multiple lines
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum FlexWrap {
    /// Single line, will overflow if needed
    #[default]
    NoWrap,
    /// Multiple lines, if needed
    Wrap,
    /// Same as [`FlexWrap::Wrap`] but new lines will appear before the previous one
    WrapReverse,
}

/// The calculated size of the node
#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedSize {
    /// The size of the node
    pub size: Size,
}

/// The color of the node
#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct UiColor(pub Color);

impl From<Color> for UiColor {
    fn from(color: Color) -> Self {
        Self(color)
    }
}

/// The image of the node
#[derive(Component, Clone, Debug, Reflect, Deref, DerefMut)]
#[reflect(Component, Default)]
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

/// The calculated clip of the node
#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedClip {
    /// The rect of the clip
    pub clip: bevy_sprite::Rect,
}
