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
use bevy_ecs::{
    prelude::{DetectChanges, Entity, EventReader, EventWriter},
    query::With,
    schedule::IntoSystemConfigs,
    system::{NonSend, NonSendMut, Query, Res, ResMut, Resource},
};
use bevy_hierarchy::{Children, Parent};
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window, WindowClosed};

/// Maps window entities to their `AccessKit` [`Adapter`]s.
#[derive(Default, Deref, DerefMut)]
pub struct AccessKitAdapters(pub HashMap<Entity, Adapter>);

/// Maps window entities to their respective [`WinitActionHandler`]s.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct WinitActionHandlers(pub HashMap<Entity, WinitActionHandler>);

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

fn update_accessibility_nodes(
    adapters: NonSend<AccessKitAdapters>,
    focus: Res<Focus>,
    accessibility_requested: Res<AccessibilityRequested>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    manage_accessibility_updates: Res<ManageAccessibilityUpdates>,
    nodes: Query<(
        Entity,
        &AccessibilityNode,
        Option<&Children>,
        Option<&Parent>,
    )>,
    node_entities: Query<Entity, With<AccessibilityNode>>,
) {
    if !accessibility_requested.get() || !manage_accessibility_updates.get() {
        return;
    }
    if let Ok((primary_window_id, primary_window)) = primary_window.get_single() {
        if let Some(adapter) = adapters.get(&primary_window_id) {
            let should_run = focus.is_changed() || !nodes.is_empty();
            if should_run {
                adapter.update_if_active(|| {
                    let mut to_update = vec![];
                    let mut name = None;
                    if primary_window.focused {
                        let title = primary_window.title.clone();
                        name = Some(title.into_boxed_str());
                    }
                    let focus_id = (*focus).unwrap_or_else(|| primary_window_id).to_bits();
                    let mut root_children = vec![];
                    for (entity, node, children, parent) in &nodes {
                        let mut node = (**node).clone();
                        if let Some(parent) = parent {
                            if !node_entities.contains(**parent) {
                                root_children.push(NodeId(entity.to_bits()));
                            }
                        } else {
                            root_children.push(NodeId(entity.to_bits()));
                        }
                        if let Some(children) = children {
                            for child in children {
                                if node_entities.contains(*child) {
                                    node.push_child(NodeId(child.to_bits()));
                                }
                            }
                        }
                        to_update.push((
                            NodeId(entity.to_bits()),
                            node.build(&mut NodeClassSet::lock_global()),
                        ));
                    }
                    let mut root = NodeBuilder::new(Role::Window);
                    if let Some(name) = name {
                        root.set_name(name);
                    }
                    root.set_children(root_children);
                    let root = root.build(&mut NodeClassSet::lock_global());
                    let window_update = (NodeId(primary_window_id.to_bits()), root);
                    to_update.insert(0, window_update);
                    TreeUpdate {
                        nodes: to_update,
                        tree: None,
                        focus: NodeId(focus_id),
                    }
                });
            }
        }
    }
}

/// Implements winit-specific `AccessKit` functionality.
pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AccessKitAdapters>()
            .init_resource::<WinitActionHandlers>()
            .add_event::<ActionRequestWrapper>()
            .add_systems(
                PostUpdate,
                (window_closed, poll_receivers, update_accessibility_nodes)
                    .in_set(AccessibilitySystem::Update),
            );
    }
}
