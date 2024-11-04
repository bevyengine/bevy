use crate::{
    experimental::UiChildren,
    prelude::{Button, Label},
    widget::{ImageNode, TextUiReader},
    ComputedNode,
};
use bevy_a11y::{
    accesskit::{Node, Rect, Role},
    AccessibilityNode,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    prelude::{DetectChanges, Entity},
    query::{Changed, Without},
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
    world::Ref,
};
use bevy_render::{camera::CameraUpdateSystem, prelude::Camera};
use bevy_transform::prelude::GlobalTransform;

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
    camera: Query<(&Camera, &GlobalTransform)>,
    mut nodes: Query<(
        &mut AccessibilityNode,
        Ref<ComputedNode>,
        Ref<GlobalTransform>,
    )>,
) {
    if let Ok((camera, camera_transform)) = camera.get_single() {
        for (mut accessible, node, transform) in &mut nodes {
            if node.is_changed() || transform.is_changed() {
                if let Ok(translation) =
                    camera.world_to_viewport(camera_transform, transform.translation())
                {
                    let bounds = Rect::new(
                        translation.x.into(),
                        translation.y.into(),
                        (translation.x + node.size.x).into(),
                        (translation.y + node.size.y).into(),
                    );
                    accessible.set_bounds(bounds);
                }
            }
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
                accessible.set_label(label);
            } else {
                accessible.clear_label();
            }
        } else {
            let mut node = Node::new(Role::Label);
            if let Some(label) = label {
                node.set_label(label);
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
                    .after(bevy_transform::TransformSystem::TransformPropagate)
                    .after(CameraUpdateSystem)
                    // the listed systems do not affect calculated size
                    .ambiguous_with(crate::ui_stack_system),
                button_changed,
                image_changed,
                label_changed,
            ),
        );
    }
}
