use std::{num::NonZeroU128, sync::Arc};

use accesskit::{ActionHandler, ActionRequest, Node, NodeId, Role, TreeUpdate};
use accesskit_winit::Adapter;
use bevy_app::{App, CoreStage, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity, EventReader, EventWriter},
    query::{Changed, With},
    system::{NonSend, NonSendMut, Query, RemovedComponents, Res, ResMut, Resource},
};
use bevy_utils::{default, HashMap};
use bevy_window::{WindowClosed, WindowFocused, WindowId};
use crossbeam_channel::{Receiver, Sender};

#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct AccessibilityNode(pub Node);

impl From<Node> for AccessibilityNode {
    fn from(node: Node) -> Self {
        Self(node)
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct Adapters(pub HashMap<WindowId, Adapter>);

impl Adapters {
    pub fn get_primary_adapter(&self) -> Option<&Adapter> {
        self.get(&WindowId::primary())
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Receivers(pub HashMap<WindowId, Receiver<ActionRequest>>);

pub struct WinitActionHandler(pub Sender<ActionRequest>);

impl ActionHandler for WinitActionHandler {
    fn do_action(&self, request: ActionRequest) {
        self.0.send(request).expect("Failed to send");
    }
}

trait EntityExt {
    fn to_node_id(&self) -> NodeId;
}

impl EntityExt for Entity {
    fn to_node_id(&self) -> NodeId {
        let id = NonZeroU128::new((self.to_bits() + 1) as u128);
        NodeId(id.unwrap().into())
    }
}

fn handle_focus(adapters: NonSend<Adapters>, mut focus: EventReader<WindowFocused>) {
    let root_id = NodeId(NonZeroU128::new(WindowId::primary().as_u128()).unwrap());
    for event in focus.iter() {
        if let Some(adapter) = adapters.get_primary_adapter() {
            adapter.update(TreeUpdate {
                focus: if event.focused { Some(root_id) } else { None },
                ..default()
            });
        }
    }
}

fn window_closed(
    mut adapters: NonSendMut<Adapters>,
    mut receivers: ResMut<Receivers>,
    mut events: EventReader<WindowClosed>,
) {
    for WindowClosed { id, .. } in events.iter() {
        adapters.remove(id);
        receivers.remove(id);
    }
}

fn poll_receivers(receivers: Res<Receivers>, mut actions: EventWriter<ActionRequest>) {
    for (_id, receiver) in receivers.iter() {
        if let Ok(event) = receiver.try_recv() {
            actions.send(event);
        }
    }
}

fn update_accessibility_nodes(
    adapters: NonSend<Adapters>,
    query: Query<(Entity, &AccessibilityNode), Changed<AccessibilityNode>>,
) {
    let mut nodes = vec![];
    for (entity, node) in &query {
        nodes.push((entity.to_node_id(), Arc::new((**node).clone())));
    }
    if let Some(adapter) = adapters.get_primary_adapter() {
        if !nodes.is_empty() {
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
            adapter.update(TreeUpdate { nodes, ..default() });
        }
    }
}

fn remove_accessibility_nodes(
    adapters: NonSend<Adapters>,
    removed: RemovedComponents<AccessibilityNode>,
    remaining_nodes: Query<Entity, With<AccessibilityNode>>,
) {
    if removed.iter().len() != 0 {
        if let Some(adapter) = adapters.get_primary_adapter() {
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
            adapter.update(TreeUpdate {
                nodes: vec![window_update],
                ..default()
            });
        }
    }
}

pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<Adapters>()
            .init_resource::<Receivers>()
            .add_event::<ActionRequest>()
            .add_system_to_stage(CoreStage::PreUpdate, handle_focus)
            .add_system_to_stage(CoreStage::PreUpdate, window_closed)
            .add_system_to_stage(CoreStage::PreUpdate, poll_receivers)
            .add_system_to_stage(CoreStage::PreUpdate, update_accessibility_nodes)
            .add_system_to_stage(CoreStage::PostUpdate, remove_accessibility_nodes);
    }
}
