use bevy_a11y::{
    accesskit::{kurbo::Rect, Node as AccessKitNode, Role},
    AccessibilityNode,
};
use bevy_app::{App, CoreStage, Plugin};

use bevy_ecs::{
    prelude::Entity,
    query::{Changed, Or, Without},
    system::{Commands, Query},
};
use bevy_hierarchy::Children;

use bevy_text::Text;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::default;

use crate::{
    prelude::{Button, Label},
    Node, UiImage,
};

fn calc_name(texts: &Query<&Text>, children: &Children) -> Option<Box<str>> {
    let mut name = None;
    for child in children.iter() {
        if let Ok(text) = texts.get(*child) {
            let values = text
                .sections
                .iter()
                .map(|v| v.value.to_string())
                .collect::<Vec<String>>();
            name = Some(values.join(" "));
        }
    }
    name.map(|v| v.into_boxed_str())
}

fn calc_bounds(
    mut query: Query<
        (&mut AccessibilityNode, &Node, &GlobalTransform),
        Or<(Changed<Node>, Changed<GlobalTransform>)>,
    >,
) {
    for (mut accessible, node, transform) in &mut query {
        let bounds = Rect::new(
            transform.translation().x.into(),
            transform.translation().y.into(),
            (transform.translation().x + node.calculated_size.x).into(),
            (transform.translation().y + node.calculated_size.y).into(),
        );
        accessible.bounds = Some(bounds);
    }
}

fn button_changed(
    mut commands: Commands,
    mut query: Query<(Entity, &Children, Option<&mut AccessibilityNode>), Changed<Button>>,
    texts: Query<&Text>,
) {
    for (entity, children, accessible) in &mut query {
        let name = calc_name(&texts, children);
        if let Some(mut accessible) = accessible {
            accessible.role = Role::Button;
            accessible.name = name;
        } else {
            let node = AccessKitNode {
                role: Role::Button,
                name,
                ..default()
            };
            commands
                .entity(entity)
                .insert(AccessibilityNode::from(node));
        }
    }
}

fn image_changed(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Children, Option<&mut AccessibilityNode>),
        (Changed<UiImage>, Without<Button>),
    >,
    texts: Query<&Text>,
) {
    for (entity, children, accessible) in &mut query {
        let name = calc_name(&texts, children);
        if let Some(mut accessible) = accessible {
            accessible.role = Role::Image;
            accessible.name = name;
        } else {
            let node = AccessKitNode {
                role: Role::Image,
                name,
                ..default()
            };
            commands
                .entity(entity)
                .insert(AccessibilityNode::from(node));
        }
    }
}

fn label_changed(
    mut commands: Commands,
    mut query: Query<(Entity, &Text, Option<&mut AccessibilityNode>), Changed<Label>>,
) {
    for (entity, text, accessible) in &mut query {
        let values = text
            .sections
            .iter()
            .map(|v| v.value.to_string())
            .collect::<Vec<String>>();
        let name = Some(values.join(" ").into_boxed_str());
        if let Some(mut accessible) = accessible {
            accessible.role = Role::LabelText;
            accessible.name = name;
        } else {
            let node = AccessKitNode {
                role: Role::LabelText,
                name,
                ..default()
            };
            commands
                .entity(entity)
                .insert(AccessibilityNode::from(node));
        }
    }
}

pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PreUpdate, calc_bounds)
            .add_system_to_stage(CoreStage::PreUpdate, button_changed)
            .add_system_to_stage(CoreStage::PreUpdate, image_changed)
            .add_system_to_stage(CoreStage::PreUpdate, label_changed);
    }
}
