use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::{
    prelude::{Entity, EventReader},
    query::Changed,
    system::{Commands, NonSend, Query},
};
use bevy_hierarchy::Children;
use bevy_text::Text;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::default;
use bevy_winit::{
    accessibility::{AccessKitEntityExt, AccessibilityNode, Adapters},
    accesskit::{
        kurbo::Rect, ActionRequest, DefaultActionVerb, Node as AccessKitNode, Role, TreeUpdate,
    },
};

use crate::{
    prelude::{Button, Label},
    Interaction, Node, UiImage,
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

fn calc_bounds(transform: &GlobalTransform, node: &Node) -> Rect {
    Rect::new(
        transform.translation().x.into(),
        transform.translation().y.into(),
        (transform.translation().x + node.calculated_size.x).into(),
        (transform.translation().y + node.calculated_size.y).into(),
    )
}

fn button_changed(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform, &Node, &Children), Changed<Button>>,
    texts: Query<&Text>,
) {
    for (entity, transform, node, children) in &query {
        let node = AccessKitNode {
            role: Role::Button,
            bounds: Some(calc_bounds(transform, node)),
            name: calc_name(&texts, children),
            focusable: true,
            default_action_verb: Some(DefaultActionVerb::Click),
            ..default()
        };
        commands
            .entity(entity)
            .insert(AccessibilityNode::from(node));
    }
}

fn image_changed(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform, &Node, &Children), Changed<UiImage>>,
    texts: Query<&Text>,
) {
    for (entity, transform, node, children) in &query {
        let node = AccessKitNode {
            role: Role::Image,
            bounds: Some(calc_bounds(transform, node)),
            name: calc_name(&texts, children),
            ..default()
        };
        commands
            .entity(entity)
            .insert(AccessibilityNode::from(node));
    }
}

fn label_changed(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform, &Node, &Text), Changed<Label>>,
) {
    for (entity, transform, node, text) in &query {
        let values = text
            .sections
            .iter()
            .map(|v| v.value.to_string())
            .collect::<Vec<String>>();
        let name = values.join(" ");
        let bounds = Rect::new(
            transform.translation().x.into(),
            transform.translation().y.into(),
            (transform.translation().x + node.calculated_size.x).into(),
            (transform.translation().y + node.calculated_size.y).into(),
        );
        let node = AccessKitNode {
            role: Role::LabelText,
            bounds: Some(bounds),
            name: Some(name.into_boxed_str()),
            ..default()
        };
        commands
            .entity(entity)
            .insert(AccessibilityNode::from(node));
    }
}

fn update_focus(
    adapters: NonSend<Adapters>,
    interactions: Query<(Entity, &Interaction), Changed<Interaction>>,
) {
    let mut focus = None;
    let mut ran = false;
    for (entity, interaction) in &interactions {
        ran = true;
        if *interaction == Interaction::Hovered {
            focus = Some(entity.to_node_id());
            break;
        }
    }
    if ran {
        if let Some(adapter) = adapters.get_primary_adapter() {
            adapter.update(TreeUpdate { focus, ..default() });
        }
    }
}

fn action_requested(mut events: EventReader<ActionRequest>) {
    for action in events.iter() {
        println!("AT action request: {:?}", action);
    }
}

pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PreUpdate, button_changed)
            .add_system_to_stage(CoreStage::PreUpdate, image_changed)
            .add_system_to_stage(CoreStage::PreUpdate, label_changed)
            .add_system_to_stage(CoreStage::PreUpdate, update_focus)
            .add_system_to_stage(CoreStage::PreUpdate, action_requested);
    }
}
