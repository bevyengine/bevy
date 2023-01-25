use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use accesskit_winit::Adapter;
use bevy_a11y::{
    accesskit::{ActionHandler, ActionRequest, Node, NodeId, Role, TreeUpdate},
    AccessKitEntityExt, AccessibilityNode, Focus,
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{DetectChanges, Entity, EventReader, EventWriter},
    query::{Changed, With},
    system::{NonSend, NonSendMut, Query, RemovedComponents, Res, ResMut, Resource},
};
use bevy_utils::{default, HashMap};
use bevy_window::{PrimaryWindow, Window, WindowClosed, WindowFocused};

#[derive(Default, Deref, DerefMut)]
pub struct AccessKitAdapters(pub HashMap<Entity, Adapter>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct WinitActionHandlers(pub HashMap<Entity, WinitActionHandler>);

#[derive(Clone, Default, Deref, DerefMut)]
pub struct WinitActionHandler(pub Arc<Mutex<VecDeque<ActionRequest>>>);

impl ActionHandler for WinitActionHandler {
    fn do_action(&self, request: ActionRequest) {
        let mut requests = self.0.lock().unwrap();
        requests.push_back(request);
    }
}

fn handle_window_focus(
    focus: Res<Focus>,
    adapters: NonSend<AccessKitAdapters>,
    mut focused: EventReader<WindowFocused>,
) {
    for event in focused.iter() {
        if let Some(adapter) = adapters.get(&event.window) {
            adapter.update_if_active(|| {
                let focus_id = (*focus).unwrap_or_else(|| event.window);
                TreeUpdate {
                    focus: if event.focused {
                        Some(focus_id.to_node_id())
                    } else {
                        None
                    },
                    ..default()
                }
            });
        }
    }
}

fn window_closed(
    mut adapters: NonSendMut<AccessKitAdapters>,
    mut receivers: NonSendMut<WinitActionHandlers>,
    mut events: EventReader<WindowClosed>,
) {
    for WindowClosed { window, .. } in events.iter() {
        adapters.remove(window);
        receivers.remove(window);
    }
}

fn poll_receivers(handlers: Res<WinitActionHandlers>, mut actions: EventWriter<ActionRequest>) {
    for (_id, handler) in handlers.iter() {
        let mut handler = handler.lock().unwrap();
        while let Some(event) = handler.pop_front() {
            actions.send(event);
        }
    }
}

fn update_accessibility_nodes(
    adapters: NonSend<AccessKitAdapters>,
    focus: Res<Focus>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    nodes: Query<(Entity, &AccessibilityNode), Changed<AccessibilityNode>>,
) {
    if let Ok((primary_window_id, primary_window)) = primary_window.get_single() {
        if let Some(adapter) = adapters.get(&primary_window_id) {
            let should_run = focus.is_changed() || !nodes.is_empty();
            if should_run {
                adapter.update_if_active(|| {
                    let mut to_update = vec![];
                    let mut has_focus = false;
                    let mut name = None;
                    if primary_window.focused {
                        has_focus = true;
                        let title = primary_window.title.clone();
                        name = Some(title.into_boxed_str());
                    }
                    let focus_id = if has_focus {
                        (*focus).or_else(|| Some(primary_window_id))
                    } else {
                        None
                    };
                    for (entity, node) in &nodes {
                        to_update.push((entity.to_node_id(), Arc::new((**node).clone())));
                    }
                    let children = to_update.iter().map(|v| v.0).collect::<Vec<NodeId>>();
                    let window_update = (
                        primary_window_id.to_node_id(),
                        Arc::new(Node {
                            role: Role::Window,
                            name,
                            children,
                            ..default()
                        }),
                    );
                    to_update.insert(0, window_update);
                    TreeUpdate {
                        nodes: to_update,
                        focus: focus_id.map(|v| v.to_node_id()),
                        ..default()
                    }
                });
            }
        }
    }
}

fn remove_accessibility_nodes(
    adapters: NonSend<AccessKitAdapters>,
    mut focus: ResMut<Focus>,
    removed: RemovedComponents<AccessibilityNode>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    remaining_nodes: Query<Entity, With<AccessibilityNode>>,
) {
    if removed.iter().len() != 0 {
        if let Ok(primary_window_id) = primary_window.get_single() {
            if let Some(adapter) = adapters.get(&primary_window_id) {
                adapter.update_if_active(|| {
                    if let Some(last_focused_entity) = **focus {
                        for entity in removed.iter() {
                            if entity == last_focused_entity {
                                **focus = None;
                                break;
                            }
                        }
                    }
                    let children = remaining_nodes
                        .iter()
                        .map(|v| v.to_node_id())
                        .collect::<Vec<NodeId>>();
                    let window_update = (
                        primary_window_id.to_node_id(),
                        Arc::new(Node {
                            role: Role::Window,
                            children,
                            ..default()
                        }),
                    );
                    let focus = (**focus).unwrap_or(primary_window_id);
                    TreeUpdate {
                        nodes: vec![window_update],
                        focus: Some(focus.to_node_id()),
                        ..default()
                    }
                });
            }
        }
    }
}

pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AccessKitAdapters>()
            .init_non_send_resource::<WinitActionHandlers>()
            .add_event::<ActionRequest>()
            .add_system_to_stage(CoreStage::PreUpdate, handle_window_focus)
            .add_system_to_stage(CoreStage::PreUpdate, window_closed)
            .add_system_to_stage(CoreStage::PreUpdate, poll_receivers)
            .add_system_to_stage(CoreStage::PreUpdate, update_accessibility_nodes)
            .add_system_to_stage(CoreStage::PostUpdate, remove_accessibility_nodes);
    }
}
