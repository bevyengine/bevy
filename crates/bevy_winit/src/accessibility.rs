use std::{
    collections::VecDeque,
    num::NonZeroU128,
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
    prelude::{Entity, EventReader, EventWriter},
    query::With,
    system::{NonSend, NonSendMut, Query, RemovedComponents, Res, ResMut, Resource},
};
use bevy_utils::{default, HashMap};
use bevy_window::{WindowClosed, WindowFocused, WindowId};

#[derive(Default, Deref, DerefMut)]
pub struct AccessKitAdapters(pub HashMap<WindowId, Adapter>);

impl AccessKitAdapters {
    pub fn get_primary_adapter(&self) -> Option<&Adapter> {
        self.get(&WindowId::primary())
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct WinitActionHandlers(pub HashMap<WindowId, WinitActionHandler>);

#[derive(Clone, Default, Deref, DerefMut)]
pub struct WinitActionHandler(pub Arc<Mutex<VecDeque<ActionRequest>>>);

impl ActionHandler for WinitActionHandler {
    fn do_action(&self, request: ActionRequest) {
        println!("Pushing {:?}", request);
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
        if let Some(adapter) = adapters.get_primary_adapter() {
            adapter.update_if_active(|| {
                let focus_id = (*focus).unwrap_or_else(|| {
                    NodeId(NonZeroU128::new(WindowId::primary().as_u128()).unwrap())
                });
                TreeUpdate {
                    focus: if event.focused { Some(focus_id) } else { None },
                    ..default()
                }
            });
        }
    }
}

fn window_closed(
    mut adapters: NonSendMut<AccessKitAdapters>,
    mut receivers: ResMut<WinitActionHandlers>,
    mut events: EventReader<WindowClosed>,
) {
    for WindowClosed { id, .. } in events.iter() {
        adapters.remove(id);
        receivers.remove(id);
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
    query: Query<(Entity, &AccessibilityNode)>,
) {
    if let Some(adapter) = adapters.get_primary_adapter() {
        adapter.update_if_active(|| {
            let mut nodes = vec![];
            let focus_id = (*focus).unwrap_or_else(|| {
                NodeId(NonZeroU128::new(WindowId::primary().as_u128()).unwrap())
            });
            for (entity, node) in &query {
                nodes.push((entity.to_node_id(), Arc::new((**node).clone())));
            }
            let root_id = NodeId(NonZeroU128::new(WindowId::primary().as_u128()).unwrap());
            let children = nodes.iter().map(|v| v.0).collect::<Vec<NodeId>>();
            let window_update = (
                root_id,
                Arc::new(Node {
                    role: Role::Window,
                    children,
                    ..default()
                }),
            );
            nodes.insert(0, window_update);
            // Dummy
            TreeUpdate {
                nodes,
                focus: Some(focus_id),
                ..default()
            }
        });
    }
}

fn remove_accessibility_nodes(
    adapters: NonSend<AccessKitAdapters>,
    mut focus: ResMut<Focus>,
    removed: RemovedComponents<AccessibilityNode>,
    remaining_nodes: Query<Entity, With<AccessibilityNode>>,
) {
    if removed.iter().len() != 0 {
        if let Some(adapter) = adapters.get_primary_adapter() {
            adapter.update_if_active(|| {
                if let Some(last_focused_entity) = focus.entity() {
                    for entity in removed.iter() {
                        if entity == last_focused_entity {
                            **focus = None;
                            break;
                        }
                    }
                }
                let root_id = NodeId(NonZeroU128::new(WindowId::primary().as_u128()).unwrap());
                let children = remaining_nodes
                    .iter()
                    .map(|v| v.to_node_id())
                    .collect::<Vec<NodeId>>();
                let window_update = (
                    root_id,
                    Arc::new(Node {
                        role: Role::Window,
                        children,
                        ..default()
                    }),
                );
                let focus = (**focus).unwrap_or(root_id);
                TreeUpdate {
                    nodes: vec![window_update],
                    focus: Some(focus),
                    ..default()
                }
            });
        }
    }
}

pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AccessKitAdapters>()
            .init_resource::<WinitActionHandlers>()
            .init_resource::<Focus>()
            .add_event::<ActionRequest>()
            .add_system_to_stage(CoreStage::PreUpdate, handle_window_focus)
            .add_system_to_stage(CoreStage::PreUpdate, window_closed)
            .add_system_to_stage(CoreStage::PreUpdate, poll_receivers)
            .add_system_to_stage(CoreStage::PreUpdate, update_accessibility_nodes)
            .add_system_to_stage(CoreStage::PostUpdate, remove_accessibility_nodes);
    }
}
