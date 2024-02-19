//! Helpers for mapping window entities to accessibility types

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use accesskit_winit::Adapter;
use bevy_a11y::{
    accesskit::{
        ActionHandler, ActionRequest, NodeBuilder, NodeClassSet, NodeId, Role, TreeUpdate,
    },
    AccessibilityNode, AccessibilityRequested, AccessibilitySystem, Focus,
};
use bevy_a11y::{ActionRequest as ActionRequestWrapper, ManageAccessibilityUpdates};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    prelude::{DetectChanges, Entity, EventReader, EventWriter},
    query::With,
    schedule::IntoSystemConfigs,
    system::{NonSend, NonSendMut, Query, Res, ResMut, Resource},
};
use bevy_hierarchy::{Children, Parent};
use bevy_window::{PrimaryWindow, Window, WindowClosed};

/// Maps window entities to their `AccessKit` [`Adapter`]s.
#[derive(Default, Deref, DerefMut)]
pub struct AccessKitAdapters(pub EntityHashMap<Adapter>);

/// Maps window entities to their respective [`WinitActionHandler`]s.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct WinitActionHandlers(pub EntityHashMap<WinitActionHandler>);

/// Forwards `AccessKit` [`ActionRequest`]s from winit to an event channel.
#[derive(Clone, Default, Deref, DerefMut)]
pub struct WinitActionHandler(pub Arc<Mutex<VecDeque<ActionRequest>>>);

impl ActionHandler for WinitActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        let mut requests = self.0.lock().unwrap();
        requests.push_back(request);
    }
}

fn window_closed(
    mut adapters: NonSendMut<AccessKitAdapters>,
    mut receivers: ResMut<WinitActionHandlers>,
    mut events: EventReader<WindowClosed>,
) {
    for WindowClosed { window, .. } in events.read() {
        adapters.remove(window);
        receivers.remove(window);
    }
}

fn poll_receivers(
    handlers: Res<WinitActionHandlers>,
    mut actions: EventWriter<ActionRequestWrapper>,
) {
    for (_id, handler) in handlers.iter() {
        let mut handler = handler.lock().unwrap();
        while let Some(event) = handler.pop_front() {
            actions.send(ActionRequestWrapper(event));
        }
    }
}

fn should_update_accessibility_nodes(
    accessibility_requested: Res<AccessibilityRequested>,
    manage_accessibility_updates: Res<ManageAccessibilityUpdates>,
) -> bool {
    accessibility_requested.get() && manage_accessibility_updates.get()
}

fn update_accessibility_nodes(
    adapters: NonSend<AccessKitAdapters>,
    focus: Res<Focus>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    nodes: Query<(
        Entity,
        &AccessibilityNode,
        Option<&Children>,
        Option<&Parent>,
    )>,
    node_entities: Query<Entity, With<AccessibilityNode>>,
) {
    let Ok((primary_window_id, primary_window)) = primary_window.get_single() else {
        return;
    };
    let Some(adapter) = adapters.get(&primary_window_id) else {
        return;
    };
    if focus.is_changed() || !nodes.is_empty() {
        adapter.update_if_active(|| {
            update_adapter(
                nodes,
                node_entities,
                primary_window,
                primary_window_id,
                focus,
            )
        });
    }
}

fn update_adapter(
    nodes: Query<(
        Entity,
        &AccessibilityNode,
        Option<&Children>,
        Option<&Parent>,
    )>,
    node_entities: Query<Entity, With<AccessibilityNode>>,
    primary_window: &Window,
    primary_window_id: Entity,
    focus: Res<Focus>,
) -> TreeUpdate {
    let mut to_update = vec![];
    let mut window_children = vec![];
    for (entity, node, children, parent) in &nodes {
        let mut node = (**node).clone();
        queue_node_for_update(entity, parent, &node_entities, &mut window_children);
        add_children_nodes(children, &node_entities, &mut node);
        let node_id = NodeId(entity.to_bits());
        let node = node.build(&mut NodeClassSet::lock_global());
        to_update.push((node_id, node));
    }
    let mut window_node = NodeBuilder::new(Role::Window);
    if primary_window.focused {
        let title = primary_window.title.clone();
        window_node.set_name(title.into_boxed_str());
    }
    window_node.set_children(window_children);
    let window_node = window_node.build(&mut NodeClassSet::lock_global());
    let node_id = NodeId(primary_window_id.to_bits());
    let window_update = (node_id, window_node);
    to_update.insert(0, window_update);
    TreeUpdate {
        nodes: to_update,
        tree: None,
        focus: NodeId(focus.unwrap_or(primary_window_id).to_bits()),
    }
}

#[inline]
fn queue_node_for_update(
    node_entity: Entity,
    parent: Option<&Parent>,
    node_entities: &Query<Entity, With<AccessibilityNode>>,
    window_children: &mut Vec<NodeId>,
) {
    let should_push = if let Some(parent) = parent {
        !node_entities.contains(parent.get())
    } else {
        true
    };
    if should_push {
        window_children.push(NodeId(node_entity.to_bits()));
    }
}

#[inline]
fn add_children_nodes(
    children: Option<&Children>,
    node_entities: &Query<Entity, With<AccessibilityNode>>,
    node: &mut NodeBuilder,
) {
    let Some(children) = children else {
        return;
    };
    for child in children {
        if node_entities.contains(*child) {
            node.push_child(NodeId(child.to_bits()));
        }
    }
}

/// Implements winit-specific `AccessKit` functionality.
pub struct AccessKitPlugin;

impl Plugin for AccessKitPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AccessKitAdapters>()
            .init_resource::<WinitActionHandlers>()
            .add_event::<ActionRequestWrapper>()
            .add_systems(
                PostUpdate,
                (
                    poll_receivers,
                    update_accessibility_nodes.run_if(should_update_accessibility_nodes),
                    window_closed
                        .before(poll_receivers)
                        .before(update_accessibility_nodes),
                )
                    .in_set(AccessibilitySystem::Update),
            );
    }
}
