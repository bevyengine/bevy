//! This module contains basic node bundles used to build UIs

use crate::{
    layout_components::{SizeConstraints, Spacing},
    prelude::{FlexContainer, FlexItem},
    widget::Button,
    BackgroundColor, CalculatedSize, FocusPolicy, Interaction, LayoutControl, Node, UiImage,
    ZIndex,
};
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    prelude::{Color, ComputedVisibility},
    view::Visibility,
};
use bevy_text::Text;
use bevy_transform::prelude::{GlobalTransform, Transform};

/// The basic UI node
///
/// Useful as a container for a variety of child nodes.
#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Core controls for layouting of this node.
    ///
    /// See: [`Display`](crate::Display), [`Position`](crate::Position), [`Inset`](crate::Inset), [`Overflow`](crate::Overflow).
    pub control: LayoutControl,
    /// Defines how this node's layout should be.
    pub layout: FlexContainer,
    /// Defines how  this node should behave as a child of a node.
    pub child_layout: FlexItem,
    /// The constraints on the size of this node
    pub size_constraints: SizeConstraints,
    /// The margin, padding and border of the UI node
    pub spacing: Spacing,
    /// The background color, which serves as a "fill" for this node
    pub background_color: BackgroundColor,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl Default for NodeBundle {
    fn default() -> Self {
        NodeBundle {
            node: Default::default(),
            control: Default::default(),
            layout: Default::default(),
            child_layout: Default::default(),
            size_constraints: Default::default(),
            spacing: Default::default(),
            // Transparent background
            background_color: Color::NONE.into(),
            focus_policy: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}

/// A UI node that is an image
#[derive(Bundle, Clone, Debug, Default)]
pub struct ImageBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Core controls for layouting of this node.
    ///
    /// See: [`Display`](crate::Display), [`Position`](crate::Position), [`Inset`](crate::Inset), [`Overflow`](crate::Overflow).
    pub control: LayoutControl,
    /// Defines how this node's layout should be.
    pub layout: FlexContainer,
    /// Defines how  this node should behave as a child of a node.
    pub child_layout: FlexItem,
    /// The constraints on the size of this node
    pub size_constraints: SizeConstraints,
    /// The margin, padding and border of the UI node
    pub spacing: Spacing,
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// The background color, which serves as a "fill" for this node
    ///
    /// Combines with `UiImage` to tint the provided image.
    pub background_color: BackgroundColor,
    /// The image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

/// A UI node that is text
#[derive(Bundle, Clone, Debug)]
pub struct TextBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Core controls for layouting of this node.
    ///
    /// See: [`Display`](crate::Display), [`Position`](crate::Position), [`Inset`](crate::Inset), [`Overflow`](crate::Overflow).
    pub control: LayoutControl,
    /// Defines how this node's layout should be.
    pub layout: FlexContainer,
    /// Defines how  this node should behave as a child of a node.
    pub child_layout: FlexItem,
    /// The constraints on the size of this node
    pub size_constraints: SizeConstraints,
    /// The margin, padding and border of the UI node
    pub spacing: Spacing,
    /// Contains the text of the node
    pub text: Text,
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl Default for TextBundle {
    fn default() -> Self {
        TextBundle {
            focus_policy: FocusPolicy::Pass,
            text: Default::default(),
            node: Default::default(),
            calculated_size: Default::default(),
            layout: Default::default(),
            control: Default::default(),
            child_layout: Default::default(),
            size_constraints: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
            spacing: Default::default(),
        }
    }
}

/// A UI node that is a button
#[derive(Bundle, Clone, Debug)]
pub struct ButtonBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Marker component that signals this node is a button
    pub button: Button,
    /// Core controls for layouting of this node.
    ///
    /// See: [`Display`](crate::Display), [`Position`](crate::Position), [`Inset`](crate::Inset), [`Overflow`](crate::Overflow).
    pub control: LayoutControl,
    /// Defines how this node's layout should be.
    pub layout: FlexContainer,
    /// Defines how  this node should behave as a child of a node.
    pub child_layout: FlexItem,
    /// The constraints on the size of this node
    pub size_constraints: SizeConstraints,
    /// The margin, padding and border of the UI node
    pub spacing: Spacing,
    /// Describes whether and how the button has been interacted with by the input
    pub interaction: Interaction,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The background color, which serves as a "fill" for this node
    ///
    /// When combined with `UiImage`, tints the provided image.
    pub background_color: BackgroundColor,
    /// The image of the node
    pub image: UiImage,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of this entity, use the properties of layouting components.
    ///
    /// See: [`LayoutControl`], [`FlexContainer`], [`FlexItem`], [`SizeConstraints`], [`Spacing`].
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl Default for ButtonBundle {
    fn default() -> Self {
        Self {
            focus_policy: FocusPolicy::Block,
            node: Default::default(),
            button: Default::default(),
            interaction: Default::default(),
            background_color: Default::default(),
            image: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
            control: Default::default(),
            layout: Default::default(),
            child_layout: Default::default(),
            size_constraints: Default::default(),
            spacing: Default::default(),
        }
    }
}
