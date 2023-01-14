//! This module contains basic node bundles used to build UIs

use crate::{BackgroundColor, FocusPolicy, Node, Style, ZIndex};
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    prelude::{Color, ComputedVisibility},
    view::Visibility,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// The basic UI node
///
/// Useful as a container for a variety of child nodes.
#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// The background color, which serves as a "fill" for this node
    pub background_color: BackgroundColor,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `nodebundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
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
            // Transparent background
            background_color: Color::NONE.into(),
            node: Default::default(),
            style: Default::default(),
            focus_policy: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}
