use bevy_a11y::accesskit::{NodeBuilder, Role};
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::{Bundle, Component, Entity};
use bevy_ecs::query::Changed;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::system::{Commands, Query};
use bevy_hierarchy::Children;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::{FromReflect, Reflect, ReflectFromReflect};
use bevy_render::view::{ComputedVisibility, Visibility};
use bevy_text::Text;
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_ui::{BackgroundColor, FocusPolicy, Interaction, Node, Style, UiImage, ZIndex};

use crate::calc_name;

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, Reflect, FromReflect)]
#[reflect(Component, FromReflect, Default)]
pub struct Button;

fn button_changed(
    mut commands: Commands,
    mut query: Query<(Entity, &Children, Option<&mut AccessibilityNode>), Changed<Button>>,
    texts: Query<&Text>,
) {
    for (entity, children, accessible) in &mut query {
        let name = calc_name(&texts, children);
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Button);
            if let Some(name) = name {
                accessible.set_name(name);
            } else {
                accessible.clear_name();
            }
        } else {
            let mut node = NodeBuilder::new(Role::Button);
            if let Some(name) = name {
                node.set_name(name);
            }
            commands
                .entity(entity)
                .insert(AccessibilityNode::from(node));
        }
    }
}

/// A plugin for button widgets
#[derive(Default)]
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Button>()
            .add_systems(Update, button_changed);
    }
}

/// A UI node that is a button
#[derive(Bundle, Clone, Debug)]
pub struct ButtonBundle {
    /// Describes the logical size of the node
    pub node: Node,
    /// Marker component that signals this node is a button
    pub button: Button,
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
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
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
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

impl Default for ButtonBundle {
    fn default() -> Self {
        Self {
            focus_policy: FocusPolicy::Block,
            node: Default::default(),
            button: Default::default(),
            style: Default::default(),
            interaction: Default::default(),
            background_color: Default::default(),
            image: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}
