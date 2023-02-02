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
    query: Query<(Entity, &Children), Changed<Button>>,
    texts: Query<&Text>,
) {
    for (entity, children) in &query {
        let node = AccessKitNode {
            role: Role::Button,
            name: calc_name(&texts, children),
            ..default()
        };
        commands
            .entity(entity)
            .insert(AccessibilityNode::from(node));
    }
}

fn image_changed(
    mut commands: Commands,
    query: Query<(Entity, &Children), (Changed<UiImage>, Without<Button>)>,
    texts: Query<&Text>,
) {
    for (entity, children) in &query {
        let node = AccessKitNode {
            role: Role::Image,
            name: calc_name(&texts, children),
            ..default()
        };
        commands
            .entity(entity)
            .insert(AccessibilityNode::from(node));
    }
}

fn label_changed(mut commands: Commands, query: Query<(Entity, &Text), Changed<Label>>) {
    for (entity, text) in &query {
        let values = text
            .sections
            .iter()
            .map(|v| v.value.to_string())
            .collect::<Vec<String>>();
        let name = values.join(" ");
        let node = AccessKitNode {
            role: Role::LabelText,
            name: Some(name.into_boxed_str()),
            ..default()
        };
        commands
            .entity(entity)
            .insert(AccessibilityNode::from(node));
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
