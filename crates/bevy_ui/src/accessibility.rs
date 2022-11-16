use bevy_app::{App, CoreStage, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Entity, EventReader},
    query::{Changed, Without},
    system::{Commands, NonSend, Query, RemovedComponents, ResMut, Resource},
};
use bevy_hierarchy::Children;
use bevy_render::{camera::RenderTarget, prelude::Camera};
use bevy_text::Text;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{default, HashMap};
use bevy_window::Windows;
use bevy_winit::{
    accessibility::{AccessKitEntityExt, AccessibilityNode, Adapters},
    accesskit::{
        kurbo::Rect, Action, ActionRequest, DefaultActionVerb, Node as AccessKitNode, Role,
        TreeUpdate,
    },
};

use crate::{
    prelude::{Button, Label, UiCameraConfig},
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
    query: Query<(Entity, &GlobalTransform, &Node, &Children), (Changed<UiImage>, Without<Button>)>,
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

#[derive(Resource, Default, Deref, DerefMut)]
struct InteractionCache(HashMap<Entity, Interaction>);

fn interaction_changed(
    mut cache: ResMut<InteractionCache>,
    adapters: NonSend<Adapters>,
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
) {
    for (entity, interaction) in &query {
        if let Some(adapter) = adapters.get_primary_adapter() {
            if *interaction == Interaction::Hovered {
                let focus = Some(entity.to_node_id());
                adapter.update(TreeUpdate { focus, ..default() });
            } else if let Some(old_interaction) = cache.get(&entity) {
                if *old_interaction == Interaction::Hovered {
                    adapter.update(TreeUpdate {
                        focus: None,
                        ..default()
                    });
                }
            }
        }
        cache.insert(entity, interaction.clone());
    }
}

fn interaction_removed(
    mut cache: ResMut<InteractionCache>,
    adapters: NonSend<Adapters>,
    removed: RemovedComponents<Interaction>,
) {
    for entity in removed.iter() {
        if let Some(old_interaction) = cache.get(&entity) {
            if *old_interaction == Interaction::Hovered {
                if let Some(adapter) = adapters.get_primary_adapter() {
                    adapter.update(TreeUpdate {
                        focus: None,
                        ..default()
                    });
                }
            }
        }
        cache.remove(&entity);
    }
}

fn action_requested(
    mut events: EventReader<ActionRequest>,
    transforms: Query<&GlobalTransform>,
    camera: Query<(&Camera, Option<&UiCameraConfig>)>,
    mut windows: ResMut<Windows>,
) {
    for action in events.iter() {
        let target = <Entity as AccessKitEntityExt>::from_node_id(&action.target);
        match action.action {
            Action::Focus => {
                if let Ok(transform) = transforms.get(target) {
                    let is_ui_disabled = |camera_ui| {
                        matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. }))
                    };
                    let window_id = camera
                        .iter()
                        .filter(|(_, camera_ui)| !is_ui_disabled(*camera_ui))
                        .filter_map(|(camera, _)| {
                            if let RenderTarget::Window(window_id) = camera.target {
                                Some(window_id)
                            } else {
                                None
                            }
                        })
                        .next();
                    if let Some(window_id) = window_id {
                        if let Some(window) = windows.get_mut(window_id) {
                            if window.is_focused() {
                                let position = transform.translation().truncate();
                                window.set_cursor_position(position);
                            }
                        }
                    }
                }
            }
            _ => {
                println!("Unsupported: {:?}", action);
            }
        };
        println!("AT action request: {:?}", action);
    }
}

pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InteractionCache>()
            .add_system_to_stage(CoreStage::PreUpdate, button_changed)
            .add_system_to_stage(CoreStage::PreUpdate, image_changed)
            .add_system_to_stage(CoreStage::PreUpdate, label_changed)
            .add_system_to_stage(CoreStage::PreUpdate, interaction_changed)
            .add_system_to_stage(CoreStage::PostUpdate, interaction_removed)
            .add_system_to_stage(CoreStage::PreUpdate, action_requested);
    }
}
