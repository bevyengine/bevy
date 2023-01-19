//! Components used to control the layout of UI-[`Node`](super::Node) entities.
use crate::{Size, UiRect, Val};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Component;
use bevy_reflect::prelude::*;
use serde::{Deserialize, Serialize};

/// Grouping of core control parameter of the layout for this node.
#[derive(Component, Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct LayoutControl {
    /// Defines how the node will partake in the layouting.
    pub display: Display,
    /// The strategy used to position this node.
    pub position: Position,
    /// The inset of this UI node, relative to its default position
    pub inset: Inset,
    /// The behavior in case the node overflows its allocated space
    pub overflow: Overflow,
}

impl LayoutControl {
    pub const DEFAULT: Self = Self {
        display: Display::DEFAULT,
        position: Position::DEFAULT,
        inset: Inset::DEFAULT,
        overflow: Overflow::DEFAULT,
    };
}
impl Default for LayoutControl {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines how the node will partake in the layouting.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Display {
    /// Use Flexbox layout model to determine the position of this [`Node`](crate::ui_node::Node).
    Flex,
    /// The node is completely removed from layouting.
    ///
    /// The final layout will be calculated as if this node never existed.
    /// If you want to hide a node and its children, but keep its layout in place,
    /// set its [`Visibility`](bevy_render::view::Visibility) component instead.
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

/// The strategy used to position this node.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Position {
    /// Positioned in relation to its parent, while considering all sibling-nodes that also have the [`Position::Relative`] value.
    Relative,
    /// Positioned in relation to its parent, without considering any sibling nodes.
    Absolute,
}
impl Position {
    pub const DEFAULT: Self = Self::Relative;
}

impl Default for Position {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The inset of a UI node from its base position.
///
/// `Inset` is the shorthand for the CSS properties `top`, `bottom`, `left`, `right`.
/// It sets the distance between an element and its parent element.
///
/// To check the final position of a UI element, read its [`Transform`](bevy_transform::components::Transform) component.
#[derive(Deref, DerefMut, Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub struct Inset(pub UiRect);
impl Inset {
    pub const DEFAULT: Self = Self(UiRect::DEFAULT);
}

impl Default for Inset {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Controls the size of UI nodes
///
/// Layouting is performed by the Flexbox layout algorithm where the value of [`SizeConstraints`] is considered.
/// To check the actual size of a UI element, read its [`Transform`](bevy_transform::components::Transform) component
#[derive(Component, Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub struct SizeConstraints {
    /// The minimum extent, which cannot be violated by the layouting algorithm.
    ///
    /// Minimum extents override maximums.
    /// That is: if an extent's calculated maximum is less than its calculated minimum then the maximum extent will be ignored.
    pub min: Size,
    /// The suggested extent, which will be used if other constraints can be resolved.
    pub suggested: Size,
    /// The maximum extent, which cannot be violated by the layouting algorithm except when overridden by the minimum extent.
    pub max: Size,
    /// The expected aspect ratio, computed as width / height.
    pub aspect_ratio: Option<f32>,
}

impl SizeConstraints {
    /// Creates a new [`SizeConstraints`] with all values set to default.
    pub const DEFAULT: SizeConstraints = SizeConstraints {
        min: Size::DEFAULT,
        suggested: Size::DEFAULT,
        max: Size::DEFAULT,
        aspect_ratio: None,
    };

    /// Creates a new [`SizeConstraints`] with both `max` and `suggested` constraints set to the largest possible extent (100%).
    pub const FILL_PARENT: SizeConstraints = SizeConstraints {
        min: Size::DEFAULT,
        suggested: Size::FULL,
        max: Size::FULL,
        aspect_ratio: None,
    };

    /// Creates a new [`SizeConstraints`] with the `min`-constraint set to the given values.
    pub const fn min(width: Val, height: Val) -> SizeConstraints {
        SizeConstraints {
            min: Size::new(width, height),
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`SizeConstraints`] with the `suggested`-constraint set to the given values.
    pub const fn suggested(width: Val, height: Val) -> SizeConstraints {
        SizeConstraints {
            suggested: Size::new(width, height),
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`SizeConstraints`] with the `max`-constraint set to the given values.
    pub const fn max(width: Val, height: Val) -> SizeConstraints {
        SizeConstraints {
            max: Size::new(width, height),
            ..Self::DEFAULT
        }
    }
}

impl Default for SizeConstraints {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The space around and inside of a UI node
///
/// ## Margin
///
/// A margin is used to create space around UI elements, outside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val, Spacing};
/// #
/// let margin = Spacing::margin(UiRect {
///     left: Val::Px(30.0),
///     right: Val::Px(25.0),
///     top: Val::Px(10.0),
///     bottom: Val::Px(20.0),
/// });
/// ```
///
/// ## Padding
///
/// A padding is used to create space around UI elements, inside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val, Spacing};
/// #
/// let padding = Spacing::padding(UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// });
/// ```
///
/// ## Borders
///
/// A border is used to define the width of the border of a UI element.
///
/// ```
/// # use bevy_ui::{UiRect, Val, Spacing};
/// #
/// let border = Spacing::border(UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// });
/// ```
#[derive(Component, Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub struct Spacing {
    /// The space around the outside of the UI element
    pub margin: UiRect,
    /// The space around the inside of the UI element
    pub padding: UiRect,
    /// The space around the outside of the UI element that can be colored to create a visible border
    pub border: UiRect,
}

impl Default for Spacing {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Spacing {
    /// Creates a [`Spacing`] with default values in all fields.
    pub const DEFAULT: Spacing = Spacing {
        margin: UiRect::DEFAULT,
        padding: UiRect::DEFAULT,
        border: UiRect::DEFAULT,
    };

    /// Creates a [`Spacing`] with [`Val::Auto`] `margin` on all sides.
    pub const AUTO_MARGIN: Spacing = Spacing::margin_all(Val::Auto);

    /// Creates a [`Spacing`] with [`Val::Auto`] `padding` on all sides.
    pub const AUTO_PADDING: Spacing = Spacing::padding_all(Val::Auto);

    /// Creates a [`Spacing`] with a `margin` of the given size.
    pub const fn margin(rect: UiRect) -> Spacing {
        Spacing {
            margin: rect,
            ..Self::DEFAULT
        }
    }

    /// Creates a [`Spacing`] where all sides of the `margin` is of the given value.
    pub const fn margin_all(val: Val) -> Spacing {
        Spacing {
            margin: UiRect::all(val),
            ..Self::DEFAULT
        }
    }

    /// Creates a [`Spacing`] with a `padding` of the given size.
    pub const fn padding(rect: UiRect) -> Spacing {
        Spacing {
            padding: rect,
            ..Self::DEFAULT
        }
    }

    /// Creates a [`Spacing`] where all sides of the `padding` is of the given value.
    pub const fn padding_all(val: Val) -> Spacing {
        Spacing {
            padding: UiRect::all(val),
            ..Self::DEFAULT
        }
    }

    /// Creates a [`Spacing`] with a `border` of the given size.
    pub const fn border(rect: UiRect) -> Spacing {
        Spacing {
            border: rect,
            ..Self::DEFAULT
        }
    }

    /// Creates a [`Spacing`] where all sides of the `border` is of the given value.
    pub const fn border_all(val: Val) -> Spacing {
        Spacing {
            border: UiRect::all(val),
            ..Self::DEFAULT
        }
    }
}

/// Whether to show or hide overflowing items
#[derive(Copy, Clone, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum Overflow {
    /// Show overflowing items
    Visible,
    /// Hide overflowing items
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

/// Flexbox-specific layout components
pub mod flex {

    use bevy_ecs::query::{Changed, Or, WorldQuery};

    use super::*;

    /// A query for all of the components need for flexbox layout.
    ///
    /// See [`FlexLayoutChanged`] when attempting to use this as a query filter.
    #[derive(WorldQuery)]
    pub struct FlexLayoutQuery {
        pub control: &'static LayoutControl,
        /// Defines how this node's layout should be.
        pub layout: &'static FlexContainer,
        /// Defines how  this node should behave as a child of a node.
        pub child_layout: &'static FlexItem,
        /// The constraints on the size of this node
        pub size_constraints: &'static SizeConstraints,
        /// The margin, padding and border of the UI node
        pub spacing: &'static Spacing,
    }

    /// A type alias for when any of the components in a [`FlexLayoutQuery`] have changed.
    pub type FlexLayoutChanged = Or<(
        Changed<LayoutControl>,
        Changed<FlexContainer>,
        Changed<FlexItem>,
        Changed<SizeConstraints>,
        Changed<Spacing>,
    )>;

    /// The flexbox-specific layout configuration of a UI node
    ///
    /// This follows the web spec closely,
    /// you can use [guides](https://css-tricks.com/snippets/css/a-guide-to-flexbox/) for additional documentation.
    #[derive(Component, Serialize, Deserialize, Reflect, Debug, PartialEq, Clone, Copy)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub struct FlexContainer {
        /// How items are ordered inside a flexbox
        ///
        /// Sets the main and cross-axis: if this is a "row" variant, the main axis will be rows.
        pub direction: Direction,
        /// Aligns this container's contents according to the cross-axis
        pub align_items: AlignItems,
        /// Aligns this containers lines according to the cross-axis
        pub align_content: AlignContent,
        /// Aligns this containers items along the main-axis
        pub justify_content: JustifyContent,
        /// Controls how the content wraps
        pub wrap: Wrap,
        /// The size of the gutters between the rows and columns of the flexbox layout.
        ///
        /// Values of `Size::UNDEFINED` and `Size::AUTO` are treated as zero.
        pub gap: Size,
    }
    impl FlexContainer {
        pub const DEFAULT: Self = Self {
            direction: Direction::DEFAULT,
            align_items: AlignItems::DEFAULT,
            align_content: AlignContent::DEFAULT,
            justify_content: JustifyContent::DEFAULT,
            wrap: Wrap::DEFAULT,
            gap: Size::DEFAULT,
        };
    }
    impl Default for FlexContainer {
        fn default() -> Self {
            Self::DEFAULT
        }
    }

    /// Defines how this node should behave as a child of a parent node.
    #[derive(Component, Serialize, Deserialize, Reflect, Debug, PartialEq, Clone, Copy)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub struct FlexItem {
        /// Overrides the inherited [`AlignItems`] value for this node
        pub align_self: AlignSelf,
        /// Defines how much a flexbox item should grow if there's space available
        pub grow: f32,
        /// How to shrink if there's not enough space available
        pub shrink: f32,
        /// The initial size of the item
        pub basis: Val,
    }
    impl FlexItem {
        pub const DEFAULT: Self = Self {
            align_self: AlignSelf::DEFAULT,
            grow: 0.0,
            shrink: 1.0,
            basis: Val::DEFAULT,
        };
    }
    impl Default for FlexItem {
        fn default() -> Self {
            FlexItem::DEFAULT
        }
    }

    /// Defines how flexbox items are ordered within a flexbox
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub enum Direction {
        /// Same way as text direction along the main axis
        Row,
        /// Flex from bottom to top
        Column,
        /// Opposite way as text direction along the main axis
        RowReverse,
        /// Flex from top to bottom
        ColumnReverse,
    }
    impl Direction {
        pub const DEFAULT: Self = Self::Row;
    }

    impl Default for Direction {
        fn default() -> Self {
            Self::DEFAULT
        }
    }

    /// How items are aligned according to the cross axis
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
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

    /// Works like [`AlignItems`] but applies only to a single item
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub enum AlignSelf {
        /// Use the value of [`AlignItems`]
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
    /// It only applies if [`Wrap::Wrap`] is present and if there are multiple lines of items.
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub enum AlignContent {
        /// Each line moves towards the start of the cross axis
        FlexStart,
        /// Each line moves towards the end of the cross axis
        FlexEnd,
        /// Each line moves towards the center of the cross axis
        Center,
        /// Each line will stretch to fill the remaining space
        Stretch,
        /// Each line fills the space it needs, putting the remaining space, if any
        /// inbetween the lines
        SpaceBetween,
        /// Each line fills the space it needs, putting the remaining space, if any
        /// around the lines
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

    /// Defines how items are aligned according to the main axis
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub enum JustifyContent {
        /// Pushed towards the start
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
    impl JustifyContent {
        pub const DEFAULT: Self = Self::FlexStart;
    }

    impl Default for JustifyContent {
        fn default() -> Self {
            Self::DEFAULT
        }
    }

    /// Defines if flexbox items appear on a single line or on multiple lines
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
    #[reflect_value(PartialEq, Serialize, Deserialize)]
    pub enum Wrap {
        /// Single line, will overflow if needed
        NoWrap,
        /// Multiple lines, if needed
        Wrap,
        /// Same as [`Wrap::Wrap`] but new lines will appear before the previous one
        WrapReverse,
    }
    impl Wrap {
        pub const DEFAULT: Self = Self::NoWrap;
    }

    impl Default for Wrap {
        fn default() -> Self {
            Self::DEFAULT
        }
    }
}
