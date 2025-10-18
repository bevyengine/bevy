use crate::{
    experimental::UiChildren,
    prelude::{Button, Label},
    ui_transform::UiGlobalTransform,
    widget::{ImageNode, TextUiReader},
    ComputedNode,
};
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    prelude::{DetectChanges, Entity},
    query::{Changed, Without},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    world::Ref,
};

use accesskit::{Node, Rect, Role};
use bevy_camera::CameraUpdateSystems;

fn calc_label(
    text_reader: &mut TextUiReader,
    children: impl Iterator<Item = Entity>,
) -> Option<Box<str>> {
    let mut name = None;
    for child in children {
        let values = text_reader
            .iter(child)
            .map(|(_, _, text, _, _)| text.into())
            .collect::<Vec<String>>();
        if !values.is_empty() {
            name = Some(values.join(" "));
        }
    }
    name.map(String::into_boxed_str)
}

fn calc_bounds(
    mut nodes: Query<(
        &mut AccessibilityNode,
        Ref<ComputedNode>,
        Ref<UiGlobalTransform>,
    )>,
) {
    for (mut accessible, node, transform) in &mut nodes {
        if node.is_changed() || transform.is_changed() {
            let center = transform.translation;
            let half_size = 0.5 * node.size;
            let min = center - half_size;
            let max = center + half_size;
            let bounds = Rect::new(min.x as f64, min.y as f64, max.x as f64, max.y as f64);
            accessible.set_bounds(bounds);
        }
    }
}

fn button_changed(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&mut AccessibilityNode>), Changed<Button>>,
    ui_children: UiChildren,
    mut text_reader: TextUiReader,
) {
    for (entity, accessible) in &mut query {
        let label = calc_label(&mut text_reader, ui_children.iter_ui_children(entity));
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Button);
            if let Some(name) = label {
                accessible.set_label(name);
            } else {
                accessible.clear_label();
            }
        } else {
            let mut node = Node::new(Role::Button);
            if let Some(label) = label {
                node.set_label(label);
            }
            commands
                .entity(entity)
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

fn image_changed(
    mut commands: Commands,
    mut query: Query<
        (Entity, Option<&mut AccessibilityNode>),
        (Changed<ImageNode>, Without<Button>),
    >,
    ui_children: UiChildren,
    mut text_reader: TextUiReader,
) {
    for (entity, accessible) in &mut query {
        let label = calc_label(&mut text_reader, ui_children.iter_ui_children(entity));
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Image);
            if let Some(label) = label {
                accessible.set_label(label);
            } else {
                accessible.clear_label();
            }
        } else {
            let mut node = Node::new(Role::Image);
            if let Some(label) = label {
                node.set_label(label);
            }
            commands
                .entity(entity)
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

fn label_changed(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&mut AccessibilityNode>), Changed<Label>>,
    mut text_reader: TextUiReader,
) {
    for (entity, accessible) in &mut query {
        let values = text_reader
            .iter(entity)
            .map(|(_, _, text, _, _)| text.into())
            .collect::<Vec<String>>();
        let label = Some(values.join(" ").into_boxed_str());
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Label);
            if let Some(label) = label {
                accessible.set_value(label);
            } else {
                accessible.clear_value();
            }
        } else {
            let mut node = Node::new(Role::Label);
            if let Some(label) = label {
                node.set_value(label);
            }
            commands
                .entity(entity)
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

/// `AccessKit` integration for `bevy_ui`.
pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                calc_bounds
                    .after(bevy_transform::TransformSystems::Propagate)
                    .after(CameraUpdateSystems)
                    // the listed systems do not affect calculated size
                    .ambiguous_with(crate::ui_stack_system),
                button_changed,
                image_changed,
                label_changed,
            ),
        );
    }
}
