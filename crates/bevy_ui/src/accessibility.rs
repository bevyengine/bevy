use crate::{
    prelude::{Button, Label},
    widget::{TextNEW, TextSpan},
    Node, UiChildren, UiImage,
};
use bevy_a11y::{
    accesskit::{NodeBuilder, Rect, Role},
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
use bevy_text::TextBlocks;
use bevy_transform::prelude::GlobalTransform;

fn calc_name(
    blocks: &mut TextBlocks<TextNEW, TextSpan>,
    children: impl Iterator<Item = Entity>,
) -> Option<Box<str>> {
    let mut name = None;
    for child in children {
        let values = blocks
            .iter(child)
            .map(|(_, _, text, _)| text.into())
            .collect::<Vec<String>>();
        if values.len() > 0 {
            name = Some(values.join(" "));
        }
    }
    name.map(String::into_boxed_str)
}

fn calc_bounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    mut nodes: Query<(&mut AccessibilityNode, Ref<Node>, Ref<GlobalTransform>)>,
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
                        (translation.x + node.calculated_size.x).into(),
                        (translation.y + node.calculated_size.y).into(),
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
    mut blocks: TextBlocks<TextNEW, TextSpan>,
) {
    for (entity, accessible) in &mut query {
        let name = calc_name(&mut blocks, ui_children.iter_ui_children(entity));
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
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

fn image_changed(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&mut AccessibilityNode>), (Changed<UiImage>, Without<Button>)>,
    ui_children: UiChildren,
    mut blocks: TextBlocks<TextNEW, TextSpan>,
) {
    for (entity, accessible) in &mut query {
        let name = calc_name(&mut blocks, ui_children.iter_ui_children(entity));
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Image);
            if let Some(name) = name {
                accessible.set_name(name);
            } else {
                accessible.clear_name();
            }
        } else {
            let mut node = NodeBuilder::new(Role::Image);
            if let Some(name) = name {
                node.set_name(name);
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
    mut blocks: TextBlocks<TextNEW, TextSpan>,
) {
    for (entity, accessible) in &mut query {
        let values = blocks
            .iter(entity)
            .map(|(_, _, text, _)| text.into())
            .collect::<Vec<String>>();
        let name = Some(values.join(" ").into_boxed_str());
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Label);
            if let Some(name) = name {
                accessible.set_name(name);
            } else {
                accessible.clear_name();
            }
        } else {
            let mut node = NodeBuilder::new(Role::Label);
            if let Some(name) = name {
                node.set_name(name);
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
