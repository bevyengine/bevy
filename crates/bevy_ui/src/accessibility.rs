use crate::{
    prelude::{Button, Label},
    Node, UiImage,
};
use bevy_a11y::{
    accesskit::{NodeBuilder, Rect, Role},
    AccessibilityNode,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    prelude::{DetectChanges, Entity},
    query::{Changed, With, Without},
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_render::{camera::CameraUpdateSystem, prelude::Camera};
use bevy_text::{Text, TextSection};
use bevy_transform::prelude::GlobalTransform;

fn calc_name(
    texts: &Query<Option<&Children>, With<Text>>,
    text_sections: &Query<&TextSection, With<Parent>>,
    children: &Children,
) -> Option<Box<str>> {
    let mut name = None;
    for child in children {
        if let Ok(maybe_children) = texts.get(*child) {
            let mut sections = Vec::new();

            if let Ok(section) = text_sections.get(*child) {
                sections.push(section);
            }

            if let Some(children) = maybe_children {
                for section in text_sections.iter_many(children) {
                    sections.push(section);
                }
            }

            let values = sections
                .iter()
                .map(|v| v.value.to_string())
                .collect::<Vec<String>>();
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
                if let Some(translation) =
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
    mut query: Query<(Entity, &Children, Option<&mut AccessibilityNode>), Changed<Button>>,
    texts: Query<Option<&Children>, With<Text>>,
    text_sections: Query<&TextSection, With<Parent>>,
) {
    for (entity, children, accessible) in &mut query {
        let name = calc_name(&texts, &text_sections, children);
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
    mut query: Query<
        (Entity, &Children, Option<&mut AccessibilityNode>),
        (Changed<UiImage>, Without<Button>),
    >,
    texts: Query<Option<&Children>, With<Text>>,
    text_sections: Query<&TextSection, With<Parent>>,
) {
    for (entity, children, accessible) in &mut query {
        let name = calc_name(&texts, &text_sections, children);
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
    mut query: Query<
        (
            Entity,
            Option<&TextSection>,
            Option<&Children>,
            Option<&mut AccessibilityNode>,
        ),
        (Changed<Label>, With<Text>),
    >,
    text_sections: Query<&TextSection, With<Parent>>,
) {
    for (entity, maybe_section, maybe_children, accessible) in &mut query {
        let mut sections = Vec::new();

        if let Some(section) = maybe_section {
            sections.push(section);
        }

        if let Some(children) = maybe_children {
            for section in text_sections.iter_many(children) {
                sections.push(section);
            }
        }

        let values = sections
            .iter()
            .map(|v| v.value.to_string())
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
                    .ambiguous_with(crate::resolve_outlines_system)
                    .ambiguous_with(crate::ui_stack_system),
                button_changed,
                image_changed,
                label_changed,
            ),
        );
    }
}
