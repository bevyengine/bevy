//! This module contains basic node bundles used to build UIs
#![expect(deprecated)]

use crate::{
    widget::{Button, ImageNodeSize},
    BackgroundColor, BorderColor, BorderRadius, ComputedNode, ContentSize, FocusPolicy, ImageNode,
    Interaction, MaterialNode, Node, ScrollPosition, UiMaterial, ZIndex,
};
use bevy_ecs::bundle::Bundle;
use bevy_render::view::{InheritedVisibility, ViewVisibility, Visibility};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// The basic UI node.
///
/// Contains the [`Node`] component and other components required to make a container.
///
/// See [`node_bundles`](crate::node_bundles) for more specialized bundles like [`ImageBundle`].
#[derive(Bundle, Clone, Debug, Default)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `Node` component instead. Inserting `Node` will also insert the other components required automatically."
)]
pub struct NodeBundle {
    /// Controls the layout (size and position) of the node and its children
    /// This also affect how the node is drawn/painted.
    pub node: Node,
    /// Describes the logical size of the node
    pub computed_node: ComputedNode,
    /// The background color, which serves as a "fill" for this node
    pub background_color: BackgroundColor,
    /// The color of the Node's border
    pub border_color: BorderColor,
    /// The border radius of the node
    pub border_radius: BorderRadius,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The scroll position of the node,
    pub scroll_position: ScrollPosition,
    /// The transform of the node
    ///
    /// This component is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Node`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This component is automatically updated by the [`TransformPropagate`](`bevy_transform::TransformSystem::TransformPropagate`) systems.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Node`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

/// A UI node that is an image
#[derive(Bundle, Debug, Default)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `ImageNode` component instead. Inserting `ImageNode` will also insert the other components required automatically."
)]
pub struct ImageBundle {
    /// Describes the logical size of the node
    pub computed_node: ComputedNode,
    /// Controls the layout (size and position) of the node and its children
    /// This also affects how the node is drawn/painted.
    pub node: Node,
    /// The calculated size based on the given image
    pub calculated_size: ContentSize,
    /// The image of the node.
    ///
    /// To tint the image, change the `color` field of this component.
    pub image: ImageNode,
    /// The color of the background that will fill the containing node.
    pub background_color: BackgroundColor,
    /// The border radius of the node
    pub border_radius: BorderRadius,
    /// The size of the image in pixels
    ///
    /// This component is set automatically
    pub image_size: ImageNodeSize,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This component is automatically managed by the UI layout system.
    /// To alter the position of the `ImageBundle`, use the properties of the [`Node`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This component is automatically updated by the [`TransformPropagate`](`bevy_transform::TransformSystem::TransformPropagate`) systems.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

/// A UI node that is a button
#[derive(Bundle, Clone, Debug)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `Button` component instead. Inserting `Button` will also insert the other components required automatically."
)]
pub struct ButtonBundle {
    /// Describes the logical size of the node
    pub computed_node: ComputedNode,
    /// Marker component that signals this node is a button
    pub button: Button,
    /// Controls the layout (size and position) of the node and its children
    /// Also affect how the node is drawn/painted.
    pub node: Node,
    /// Describes whether and how the button has been interacted with by the input
    pub interaction: Interaction,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The color of the Node's border
    pub border_color: BorderColor,
    /// The border radius of the node
    pub border_radius: BorderRadius,
    /// The image of the node
    pub image: ImageNode,
    /// The background color that will fill the containing node
    pub background_color: BackgroundColor,
    /// The transform of the node
    ///
    /// This component is automatically managed by the UI layout system.
    /// To alter the position of the `ButtonBundle`, use the properties of the [`Node`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This component is automatically updated by the [`TransformPropagate`](`bevy_transform::TransformSystem::TransformPropagate`) systems.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl Default for ButtonBundle {
    fn default() -> Self {
        Self {
            node: Default::default(),
            computed_node: Default::default(),
            button: Default::default(),
            interaction: Default::default(),
            focus_policy: FocusPolicy::Block,
            border_color: Default::default(),
            border_radius: Default::default(),
            image: Default::default(),
            background_color: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}

/// A UI node that is rendered using a [`UiMaterial`]
///
/// Adding a `BackgroundColor` component to an entity with this bundle will ignore the custom
/// material and use the background color instead.
#[derive(Bundle, Clone, Debug)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `MaterialNode` component instead. Inserting `MaterialNode` will also insert the other components required automatically."
)]
pub struct MaterialNodeBundle<M: UiMaterial> {
    /// Describes the logical size of the node
    pub computed_node: ComputedNode,
    /// Controls the layout (size and position) of the node and its children
    /// Also affects how the node is drawn/painted.
    pub node: Node,
    /// The [`UiMaterial`] used to render the node.
    pub material: MaterialNode<M>,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Node`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Node`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl<M: UiMaterial> Default for MaterialNodeBundle<M> {
    fn default() -> Self {
        Self {
            node: Default::default(),
            computed_node: Default::default(),
            material: Default::default(),
            focus_policy: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}
