use crate::{Size, UiRect};
use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Vec2};
use bevy_reflect::prelude::*;
use bevy_render::{
    color::Color,
    texture::{Image, DEFAULT_IMAGE_HANDLE},
};
use bevy_transform::prelude::GlobalTransform;
use serde::{Deserialize, Serialize};
use std::ops::{Div, DivAssign, Mul, MulAssign};
use thiserror::Error;

/// Describes the size of a UI node
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Node {
    /// The size of the node as width and height in logical pixels
    /// automatically calculated by [`super::flex::flex_node_system`]
    pub(crate) calculated_size: Vec2,
}

impl Node {
    /// The calculated node size as width and height in logical pixels
    /// automatically calculated by [`super::flex::flex_node_system`]
    pub fn size(&self) -> Vec2 {
        self.calculated_size
    }

    /// Returns the logical pixel coordinates of the UI node, based on its `GlobalTransform`.
    #[inline]
    pub fn logical_rect(&self, transform: &GlobalTransform) -> Rect {
        Rect::from_center_size(transform.translation().truncate(), self.size())
    }

    /// Returns the physical pixel coordinates of the UI node, based on its `GlobalTransform` and the scale factor.
    #[inline]
    pub fn physical_rect(&self, transform: &GlobalTransform, scale_factor: f32) -> Rect {
        let rect = self.logical_rect(transform);
        Rect {
            min: rect.min / scale_factor,
            max: rect.max / scale_factor,
        }
    }
}

impl Node {
    pub const DEFAULT: Self = Self {
        calculated_size: Vec2::ZERO,
    };
}

impl Default for Node {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Represents the possible value types for layout properties.
///
/// This enum allows specifying values for various [`Style`] properties in different units,
/// such as logical pixels, percentages, or automatically determined values.
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Val {
    /// Automatically determine the value based on the context and other `Style` properties.
    Auto,
    /// Set this value in logical pixels.
    Px(f32),
    /// Set the value as a percentage of its parent node's length along a specific axis.
    ///
    /// If the UI node has no parent, the percentage is calculated based on the window's length
    /// along the corresponding axis.
    ///
    /// The chosen axis depends on the `Style` field set:
    /// * For `flex_basis`, the percentage is relative to the main-axis length determined by the `flex_direction`.
    /// * For `gap`, `min_size`, `size`, and `max_size`:
    ///   - `width` is relative to the parent's width.
    ///   - `height` is relative to the parent's height.
    /// * For `margin`, `padding`, and `border` values: the percentage is relative to the parent node's width.
    /// * For positions, `left` and `right` are relative to the parent's width, while `bottom` and `top` are relative to the parent's height.
    Percent(f32),
    /// Set this value in percent of the viewport width
    Vw(f32),
    /// Set this value in percent of the viewport height
    Vh(f32),
    /// Set this value in percent of the viewport's smaller dimension.
    VMin(f32),
    /// Set this value in percent of the viewport's larger dimension.
    VMax(f32),
}

impl Val {
    pub const DEFAULT: Self = Self::Auto;
}

impl Default for Val {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Mul<f32> for Val {
    type Output = Val;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value * rhs),
            Val::Percent(value) => Val::Percent(value * rhs),
            Val::Vw(value) => Val::Vw(value * rhs),
            Val::Vh(value) => Val::Vh(value * rhs),
            Val::VMin(value) => Val::VMin(value * rhs),
            Val::VMax(value) => Val::VMax(value * rhs),
        }
    }
}

impl MulAssign<f32> for Val {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Val::Auto => {}
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value) => *value *= rhs,
        }
    }
}

impl Div<f32> for Val {
    type Output = Val;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value / rhs),
            Val::Percent(value) => Val::Percent(value / rhs),
            Val::Vw(value) => Val::Vw(value / rhs),
            Val::Vh(value) => Val::Vh(value / rhs),
            Val::VMin(value) => Val::VMin(value / rhs),
            Val::VMax(value) => Val::VMax(value / rhs),
        }
    }
}

impl DivAssign<f32> for Val {
    fn div_assign(&mut self, rhs: f32) {
        match self {
            Val::Auto => {}
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value) => *value /= rhs,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Error)]
pub enum ValArithmeticError {
    #[error("the variants of the Vals don't match")]
    NonIdenticalVariants,
    #[error("the given variant of Val is not evaluateable (non-numeric)")]
    NonEvaluateable,
}

impl Val {
    /// Tries to add the values of two [`Val`]s.
    /// Returns [`ValArithmeticError::NonIdenticalVariants`] if two [`Val`]s are of different variants.
    /// When adding non-numeric [`Val`]s, it returns the value unchanged.
    pub fn try_add(&self, rhs: Val) -> Result<Val, ValArithmeticError> {
        match (self, rhs) {
            (Val::Auto, Val::Auto) => Ok(*self),
            (Val::Px(value), Val::Px(rhs_value)) => Ok(Val::Px(value + rhs_value)),
            (Val::Percent(value), Val::Percent(rhs_value)) => Ok(Val::Percent(value + rhs_value)),
            _ => Err(ValArithmeticError::NonIdenticalVariants),
        }
    }

    /// Adds `rhs` to `self` and assigns the result to `self` (see [`Val::try_add`])
    pub fn try_add_assign(&mut self, rhs: Val) -> Result<(), ValArithmeticError> {
        *self = self.try_add(rhs)?;
        Ok(())
    }

    /// Tries to subtract the values of two [`Val`]s.
    /// Returns [`ValArithmeticError::NonIdenticalVariants`] if two [`Val`]s are of different variants.
    /// When adding non-numeric [`Val`]s, it returns the value unchanged.
    pub fn try_sub(&self, rhs: Val) -> Result<Val, ValArithmeticError> {
        match (self, rhs) {
            (Val::Auto, Val::Auto) => Ok(*self),
            (Val::Px(value), Val::Px(rhs_value)) => Ok(Val::Px(value - rhs_value)),
            (Val::Percent(value), Val::Percent(rhs_value)) => Ok(Val::Percent(value - rhs_value)),
            _ => Err(ValArithmeticError::NonIdenticalVariants),
        }
    }

    /// Subtracts `rhs` from `self` and assigns the result to `self` (see [`Val::try_sub`])
    pub fn try_sub_assign(&mut self, rhs: Val) -> Result<(), ValArithmeticError> {
        *self = self.try_sub(rhs)?;
        Ok(())
    }

    /// A convenience function for simple evaluation of [`Val::Percent`] variant into a concrete [`Val::Px`] value.
    /// Returns a [`ValArithmeticError::NonEvaluateable`] if the [`Val`] is impossible to evaluate into [`Val::Px`].
    /// Otherwise it returns an [`f32`] containing the evaluated value in pixels.
    ///
    /// **Note:** If a [`Val::Px`] is evaluated, it's inner value returned unchanged.
    pub fn evaluate(&self, size: f32) -> Result<f32, ValArithmeticError> {
        match self {
            Val::Percent(value) => Ok(size * value / 100.0),
            Val::Px(value) => Ok(*value),
            _ => Err(ValArithmeticError::NonEvaluateable),
        }
    }

    /// Similar to [`Val::try_add`], but performs [`Val::evaluate`] on both values before adding.
    /// Returns an [`f32`] value in pixels.
    pub fn try_add_with_size(&self, rhs: Val, size: f32) -> Result<f32, ValArithmeticError> {
        let lhs = self.evaluate(size)?;
        let rhs = rhs.evaluate(size)?;

        Ok(lhs + rhs)
    }

    /// Similar to [`Val::try_add_assign`], but performs [`Val::evaluate`] on both values before adding.
    /// The value gets converted to [`Val::Px`].
    pub fn try_add_assign_with_size(
        &mut self,
        rhs: Val,
        size: f32,
    ) -> Result<(), ValArithmeticError> {
        *self = Val::Px(self.evaluate(size)? + rhs.evaluate(size)?);
        Ok(())
    }

    /// Similar to [`Val::try_sub`], but performs [`Val::evaluate`] on both values before subtracting.
    /// Returns an [`f32`] value in pixels.
    pub fn try_sub_with_size(&self, rhs: Val, size: f32) -> Result<f32, ValArithmeticError> {
        let lhs = self.evaluate(size)?;
        let rhs = rhs.evaluate(size)?;

        Ok(lhs - rhs)
    }

    /// Similar to [`Val::try_sub_assign`], but performs [`Val::evaluate`] on both values before adding.
    /// The value gets converted to [`Val::Px`].
    pub fn try_sub_assign_with_size(
        &mut self,
        rhs: Val,
        size: f32,
    ) -> Result<(), ValArithmeticError> {
        *self = Val::Px(self.try_add_with_size(rhs, size)?);
        Ok(())
    }
}

/// Describes the style of a UI node
///
/// It uses the [Flexbox](https://cssreference.io/flexbox/) system.
#[derive(Component, Clone, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct Style {
    /// Whether to arrange this node and its children with flexbox layout
    ///
    /// If this is set to [`Display::None`], this node will be collapsed.
    pub display: Display,
    /// Whether to arrange this node relative to other nodes, or positioned absolutely
    pub position_type: PositionType,
    pub left: Val,
    pub right: Val,
    pub top: Val,
    pub bottom: Val,
    /// Which direction the content of this node should go
    pub direction: Direction,
    /// Whether to use column or row layout
    pub flex_direction: FlexDirection,
    /// How to wrap nodes
    pub flex_wrap: FlexWrap,
    /// How items are aligned according to the cross axis
    pub align_items: AlignItems,
    /// How this item is aligned according to the cross axis.
    /// Overrides [`AlignItems`].
    pub align_self: AlignSelf,
    /// How to align each line, only applies if flex_wrap is set to
    /// [`FlexWrap::Wrap`] and there are multiple lines of items
    pub align_content: AlignContent,
    /// How items align according to the main axis
    pub justify_content: JustifyContent,
    /// The amount of space around a node outside its border.
    ///
    /// If a percentage value is used, the percentage is calculated based on the width of the parent node.
    ///
    /// # Example
    /// ```
    /// # use bevy_ui::{Style, UiRect, Val};
    /// let style = Style {
    ///     margin: UiRect {
    ///         left: Val::Percent(10.),
    ///         right: Val::Percent(10.),
    ///         top: Val::Percent(15.),
    ///         bottom: Val::Percent(15.)
    ///     },
    ///     ..Default::default()
    /// };
    /// ```
    /// A node with this style and a parent with dimensions of 100px by 300px, will have calculated margins of 10px on both left and right edges, and 15px on both top and bottom edges.
    pub margin: UiRect,
    /// The amount of space between the edges of a node and its contents.
    ///
    /// If a percentage value is used, the percentage is calculated based on the width of the parent node.
    ///
    /// # Example
    /// ```
    /// # use bevy_ui::{Style, UiRect, Val};
    /// let style = Style {
    ///     padding: UiRect {
    ///         left: Val::Percent(1.),
    ///         right: Val::Percent(2.),
    ///         top: Val::Percent(3.),
    ///         bottom: Val::Percent(4.)
    ///     },
    ///     ..Default::default()
    /// };
    /// ```
    /// A node with this style and a parent with dimensions of 300px by 100px, will have calculated padding of 3px on the left, 6px on the right, 9px on the top and 12px on the bottom.
    pub padding: UiRect,
    /// The amount of space between the margins of a node and its padding.
    ///
    /// If a percentage value is used, the percentage is calculated based on the width of the parent node.
    ///
    /// The size of the node will be expanded if there are constraints that prevent the layout algorithm from placing the border within the existing node boundary.
    ///
    /// Rendering for borders is not yet implemented.
    pub border: UiRect,
    /// Defines how much a flexbox item should grow if there's space available
    pub flex_grow: f32,
    /// How to shrink if there's not enough space available
    pub flex_shrink: f32,
    /// The initial length of the main axis, before other properties are applied.
    ///
    /// If both are set, `flex_basis` overrides `size` on the main axis but it obeys the bounds defined by `min_size` and `max_size`.
    pub flex_basis: Val,
    /// The ideal size of the flexbox
    ///
    /// `size.width` is used when it is within the bounds defined by `min_size.width` and `max_size.width`.
    /// `size.height` is used when it is within the bounds defined by `min_size.height` and `max_size.height`.
    pub size: Size,
    /// The minimum size of the flexbox
    ///
    /// `min_size.width` is used if it is greater than either `size.width` or `max_size.width`, or both.
    /// `min_size.height` is used if it is greater than either `size.height` or `max_size.height`, or both.
    pub min_size: Size,
    /// The maximum size of the flexbox
    ///
    /// `max_size.width` is used if it is within the bounds defined by `min_size.width` and `size.width`.
    /// `max_size.height` is used if it is within the bounds defined by `min_size.height` and `size.height.
    pub max_size: Size,
    /// The aspect ratio of the flexbox
    pub aspect_ratio: Option<f32>,
    /// How to handle overflow
    pub overflow: Overflow,
    /// The size of the gutters between the rows and columns of the flexbox layout
    ///
    /// A value of `Size::AUTO` is treated as zero.
    pub gap: Size,
}

impl Style {
    pub const DEFAULT: Self = Self {
        display: Display::DEFAULT,
        position_type: PositionType::DEFAULT,
        left: Val::Auto,
        right: Val::Auto,
        top: Val::Auto,
        bottom: Val::Auto,
        direction: Direction::DEFAULT,
        flex_direction: FlexDirection::DEFAULT,
        flex_wrap: FlexWrap::DEFAULT,
        align_items: AlignItems::DEFAULT,
        align_self: AlignSelf::DEFAULT,
        align_content: AlignContent::DEFAULT,
        justify_content: JustifyContent::DEFAULT,
        margin: UiRect::DEFAULT,
        padding: UiRect::DEFAULT,
        border: UiRect::DEFAULT,
        flex_grow: 0.0,
        flex_shrink: 1.0,
        flex_basis: Val::Auto,
        size: Size::AUTO,
        min_size: Size::AUTO,
        max_size: Size::AUTO,
        aspect_ratio: None,
        overflow: Overflow::DEFAULT,
        gap: Size::AUTO,
    };
}

impl Default for Style {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// How items are aligned according to the cross axis
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum AlignItems {
    /// Items are packed towards the start of the axis.
    Start,
    /// Items are packed towards the end of the axis.
    End,
    /// Items are packed towards the start of the axis, unless the flex direction is reversed;
    /// then they are packed towards the end of the axis.
    FlexStart,
    /// Items are packed towards the end of the axis, unless the flex direction is reversed;
    /// then they are packed towards the start of the axis.
    FlexEnd,
    /// Items are aligned at the center.
    Center,
    /// Items are aligned at the baseline.
    Baseline,
    /// Items are stretched across the whole cross axis.
    Stretch,
}

impl AlignItems {
    pub const DEFAULT: Self = Self::Stretch;
}

impl Default for AlignItems {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// How this item is aligned according to the cross axis.
/// Overrides [`AlignItems`].
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum AlignSelf {
    /// Use the parent node's [`AlignItems`] value to determine how this item should be aligned.
    Auto,
    /// This item will be aligned with the start of the axis.
    Start,
    /// This item will be aligned with the end of the axis.
    End,
    /// This item will be aligned with the start of the axis, unless the flex direction is reversed;
    /// then it will be aligned with the end of the axis.
    FlexStart,
    /// This item will be aligned with the end of the axis, unless the flex direction is reversed;
    /// then it will be aligned with the start of the axis.
    FlexEnd,
    /// This item will be aligned at the center.
    Center,
    /// This item will be aligned at the baseline.
    Baseline,
    /// This item will be stretched across the whole cross axis.
    Stretch,
}

impl AlignSelf {
    pub const DEFAULT: Self = Self::Auto;
}

impl Default for AlignSelf {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines how each line is aligned within the flexbox.
///
/// It only applies if [`FlexWrap::Wrap`] is present and if there are multiple lines of items.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum AlignContent {
    /// Each line moves towards the start of the cross axis.
    Start,
    /// Each line moves towards the end of the cross axis.
    End,
    /// Each line moves towards the start of the cross axis, unless the flex direction is reversed; then the line moves towards the end of the cross axis.
    FlexStart,
    /// Each line moves towards the end of the cross axis, unless the flex direction is reversed; then the line moves towards the start of the cross axis.
    FlexEnd,
    /// Each line moves towards the center of the cross axis.
    Center,
    /// Each line will stretch to fill the remaining space.
    Stretch,
    /// Each line fills the space it needs, putting the remaining space, if any
    /// in-between the lines.
    SpaceBetween,
    /// The gap between the first and last items is exactly THE SAME as the gap between items.
    /// The gaps are distributed evenly.
    SpaceEvenly,
    /// Each line fills the space it needs, putting the remaining space, if any
    /// around the lines.
    SpaceAround,
}

impl AlignContent {
    pub const DEFAULT: Self = Self::Stretch;
}

impl Default for AlignContent {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines the text direction
///
/// For example English is written LTR (left-to-right) while Arabic is written RTL (right-to-left).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Direction {
    /// Inherit from parent node.
    Inherit,
    /// Text is written left to right.
    LeftToRight,
    /// Text is written right to left.
    RightToLeft,
}

impl Direction {
    pub const DEFAULT: Self = Self::Inherit;
}

impl Default for Direction {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Whether to use a Flexbox layout model.
///
/// Part of the [`Style`] component.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Display {
    /// Use Flexbox layout model to determine the position of this [`Node`].
    Flex,
    /// Use no layout, don't render this node and its children.
    ///
    /// If you want to hide a node and its children,
    /// but keep its layout in place, set its [`Visibility`](bevy_render::view::Visibility) component instead.
    None,
}

impl Display {
    pub const DEFAULT: Self = Self::Flex;
}

impl Default for Display {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines how flexbox items are ordered within a flexbox
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum FlexDirection {
    /// Same way as text direction along the main axis.
    Row,
    /// Flex from top to bottom.
    Column,
    /// Opposite way as text direction along the main axis.
    RowReverse,
    /// Flex from bottom to top.
    ColumnReverse,
}

impl FlexDirection {
    pub const DEFAULT: Self = Self::Row;
}

impl Default for FlexDirection {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines how items are aligned according to the main axis
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum JustifyContent {
    /// Items are packed toward the start of the axis.
    Start,
    /// Items are packed toward the end of the axis.
    End,
    /// Pushed towards the start, unless the flex direction is reversed; then pushed towards the end.
    FlexStart,
    /// Pushed towards the end, unless the flex direction is reversed; then pushed towards the start.
    FlexEnd,
    /// Centered along the main axis.
    Center,
    /// Remaining space is distributed between the items.
    SpaceBetween,
    /// Remaining space is distributed around the items.
    SpaceAround,
    /// Like [`JustifyContent::SpaceAround`] but with even spacing between items.
    SpaceEvenly,
}

impl JustifyContent {
    pub const DEFAULT: Self = Self::FlexStart;
}

impl Default for JustifyContent {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Whether to show or hide overflowing items
#[derive(Copy, Clone, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Overflow {
    /// Show overflowing items.
    Visible,
    /// Hide overflowing items.
    Hidden,
}

impl Overflow {
    pub const DEFAULT: Self = Self::Visible;
}

impl Default for Overflow {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The strategy used to position this node
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum PositionType {
    /// Relative to all other nodes with the [`PositionType::Relative`] value.
    Relative,
    /// Independent of all other nodes.
    ///
    /// As usual, the `Style.position` field of this node is specified relative to its parent node.
    Absolute,
}

impl PositionType {
    const DEFAULT: Self = Self::Relative;
}

impl Default for PositionType {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines if flexbox items appear on a single line or on multiple lines
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum FlexWrap {
    /// Single line, will overflow if needed.
    NoWrap,
    /// Multiple lines, if needed.
    Wrap,
    /// Same as [`FlexWrap::Wrap`] but new lines will appear before the previous one.
    WrapReverse,
}

impl FlexWrap {
    const DEFAULT: Self = Self::NoWrap;
}

impl Default for FlexWrap {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The calculated size of the node
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedSize {
    /// The size of the node in logical pixels
    pub size: Vec2,
    /// Whether to attempt to preserve the aspect ratio when determining the layout for this item
    pub preserve_aspect_ratio: bool,
}

impl CalculatedSize {
    const DEFAULT: Self = Self {
        size: Vec2::ZERO,
        preserve_aspect_ratio: false,
    };
}

impl Default for CalculatedSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The background color of the node
///
/// This serves as the "fill" color.
/// When combined with [`UiImage`], tints the provided texture.
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct BackgroundColor(pub Color);

impl BackgroundColor {
    pub const DEFAULT: Self = Self(Color::WHITE);
}

impl Default for BackgroundColor {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<Color> for BackgroundColor {
    fn from(color: Color) -> Self {
        Self(color)
    }
}

/// The 2D texture displayed for this UI node
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct UiImage {
    /// Handle to the texture
    pub texture: Handle<Image>,
    /// Whether the image should be flipped along its x-axis
    pub flip_x: bool,
    /// Whether the image should be flipped along its y-axis
    pub flip_y: bool,
}

impl Default for UiImage {
    fn default() -> UiImage {
        UiImage {
            texture: DEFAULT_IMAGE_HANDLE.typed(),
            flip_x: false,
            flip_y: false,
        }
    }
}

impl UiImage {
    pub fn new(texture: Handle<Image>) -> Self {
        Self {
            texture,
            ..Default::default()
        }
    }

    /// flip the image along its x-axis
    #[must_use]
    pub const fn with_flip_x(mut self) -> Self {
        self.flip_x = true;
        self
    }

    /// flip the image along its y-axis
    #[must_use]
    pub const fn with_flip_y(mut self) -> Self {
        self.flip_y = true;
        self
    }
}

impl From<Handle<Image>> for UiImage {
    fn from(texture: Handle<Image>) -> Self {
        Self::new(texture)
    }
}

/// The calculated clip of the node
#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CalculatedClip {
    /// The rect of the clip
    pub clip: Rect,
}

/// Indicates that this [`Node`] entity's front-to-back ordering is not controlled solely
/// by its location in the UI hierarchy. A node with a higher z-index will appear on top
/// of other nodes with a lower z-index.
///
/// UI nodes that have the same z-index will appear according to the order in which they
/// appear in the UI hierarchy. In such a case, the last node to be added to its parent
/// will appear in front of this parent's other children.
///
/// Internally, nodes with a global z-index share the stacking context of root UI nodes
/// (nodes that have no parent). Because of this, there is no difference between using
/// [`ZIndex::Local(n)`] and [`ZIndex::Global(n)`] for root nodes.
///
/// Nodes without this component will be treated as if they had a value of [`ZIndex::Local(0)`].
#[derive(Component, Copy, Clone, Debug, Reflect)]
pub enum ZIndex {
    /// Indicates the order in which this node should be rendered relative to its siblings.
    Local(i32),
    /// Indicates the order in which this node should be rendered relative to root nodes and
    /// all other nodes that have a global z-index.
    Global(i32),
}

impl Default for ZIndex {
    fn default() -> Self {
        Self::Local(0)
    }
}

#[cfg(test)]
mod tests {
    use crate::ValArithmeticError;

    use super::Val;

    #[test]
    fn val_try_add() {
        let auto_sum = Val::Auto.try_add(Val::Auto).unwrap();
        let px_sum = Val::Px(20.).try_add(Val::Px(22.)).unwrap();
        let percent_sum = Val::Percent(50.).try_add(Val::Percent(50.)).unwrap();

        assert_eq!(auto_sum, Val::Auto);
        assert_eq!(px_sum, Val::Px(42.));
        assert_eq!(percent_sum, Val::Percent(100.));
    }

    #[test]
    fn val_try_add_to_self() {
        let mut val = Val::Px(5.);

        val.try_add_assign(Val::Px(3.)).unwrap();

        assert_eq!(val, Val::Px(8.));
    }

    #[test]
    fn val_try_sub() {
        let auto_sum = Val::Auto.try_sub(Val::Auto).unwrap();
        let px_sum = Val::Px(72.).try_sub(Val::Px(30.)).unwrap();
        let percent_sum = Val::Percent(100.).try_sub(Val::Percent(50.)).unwrap();

        assert_eq!(auto_sum, Val::Auto);
        assert_eq!(px_sum, Val::Px(42.));
        assert_eq!(percent_sum, Val::Percent(50.));
    }

    #[test]
    fn different_variant_val_try_add() {
        let different_variant_sum_1 = Val::Px(50.).try_add(Val::Percent(50.));
        let different_variant_sum_2 = Val::Percent(50.).try_add(Val::Auto);

        assert_eq!(
            different_variant_sum_1,
            Err(ValArithmeticError::NonIdenticalVariants)
        );
        assert_eq!(
            different_variant_sum_2,
            Err(ValArithmeticError::NonIdenticalVariants)
        );
    }

    #[test]
    fn different_variant_val_try_sub() {
        let different_variant_diff_1 = Val::Px(50.).try_sub(Val::Percent(50.));
        let different_variant_diff_2 = Val::Percent(50.).try_sub(Val::Auto);

        assert_eq!(
            different_variant_diff_1,
            Err(ValArithmeticError::NonIdenticalVariants)
        );
        assert_eq!(
            different_variant_diff_2,
            Err(ValArithmeticError::NonIdenticalVariants)
        );
    }

    #[test]
    fn val_evaluate() {
        let size = 250.;
        let result = Val::Percent(80.).evaluate(size).unwrap();

        assert_eq!(result, size * 0.8);
    }

    #[test]
    fn val_evaluate_px() {
        let size = 250.;
        let result = Val::Px(10.).evaluate(size).unwrap();

        assert_eq!(result, 10.);
    }

    #[test]
    fn val_invalid_evaluation() {
        let size = 250.;
        let evaluate_auto = Val::Auto.evaluate(size);

        assert_eq!(evaluate_auto, Err(ValArithmeticError::NonEvaluateable));
    }

    #[test]
    fn val_try_add_with_size() {
        let size = 250.;

        let px_sum = Val::Px(21.).try_add_with_size(Val::Px(21.), size).unwrap();
        let percent_sum = Val::Percent(20.)
            .try_add_with_size(Val::Percent(30.), size)
            .unwrap();
        let mixed_sum = Val::Px(20.)
            .try_add_with_size(Val::Percent(30.), size)
            .unwrap();

        assert_eq!(px_sum, 42.);
        assert_eq!(percent_sum, 0.5 * size);
        assert_eq!(mixed_sum, 20. + 0.3 * size);
    }

    #[test]
    fn val_try_sub_with_size() {
        let size = 250.;

        let px_sum = Val::Px(60.).try_sub_with_size(Val::Px(18.), size).unwrap();
        let percent_sum = Val::Percent(80.)
            .try_sub_with_size(Val::Percent(30.), size)
            .unwrap();
        let mixed_sum = Val::Percent(50.)
            .try_sub_with_size(Val::Px(30.), size)
            .unwrap();

        assert_eq!(px_sum, 42.);
        assert_eq!(percent_sum, 0.5 * size);
        assert_eq!(mixed_sum, 0.5 * size - 30.);
    }

    #[test]
    fn val_try_add_non_numeric_with_size() {
        let size = 250.;

        let percent_sum = Val::Auto.try_add_with_size(Val::Auto, size);

        assert_eq!(percent_sum, Err(ValArithmeticError::NonEvaluateable));
    }

    #[test]
    fn val_arithmetic_error_messages() {
        assert_eq!(
            format!("{}", ValArithmeticError::NonIdenticalVariants),
            "the variants of the Vals don't match"
        );
        assert_eq!(
            format!("{}", ValArithmeticError::NonEvaluateable),
            "the given variant of Val is not evaluateable (non-numeric)"
        );
    }

    #[test]
    fn default_val_equals_const_default_val() {
        assert_eq!(Val::default(), Val::DEFAULT);
    }
}
