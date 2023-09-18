//! Helpers for mapping window entities to accessibility types

use std::{
    collections::VecDeque,
    sync::{atomic::Ordering, Arc, Mutex},
};

use accesskit_winit::Adapter;
use bevy_a11y::ActionRequest as ActionRequestWrapper;
use bevy_a11y::{
    accesskit::{ActionHandler, ActionRequest, NodeBuilder, NodeClassSet, Role, TreeUpdate},
    AccessKitEntityExt, AccessibilityNode, AccessibilityRequested, Focus,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{DetectChanges, Entity, EventReader, EventWriter},
    query::With,
    system::{NonSend, NonSendMut, Query, Res, ResMut, Resource},
};
use bevy_hierarchy::{Children, Parent};
use bevy_utils::{default, HashMap};
use bevy_window::{PrimaryWindow, Window, WindowClosed, WindowFocused};

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
    for event in focused.read() {
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
    nodes: Query<(
        Entity,
        &AccessibilityNode,
        Option<&Children>,
        Option<&Parent>,
    )>,
    node_entities: Query<Entity, With<AccessibilityNode>>,
) {
    if !accessibility_requested.load(Ordering::SeqCst) {
        return;
    }
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
                    let mut root_children = vec![];
                    for (entity, node, children, parent) in &nodes {
                        let mut node = (**node).clone();
                        if let Some(parent) = parent {
                            if node_entities.get(**parent).is_err() {
                                root_children.push(entity.to_node_id());
                            }
                        } else {
                            root_children.push(entity.to_node_id());
                        }
                        if let Some(children) = children {
                            for child in children.iter() {
                                if node_entities.get(*child).is_ok() {
                                    node.push_child(child.to_node_id());
                                }
                            }
                        }
                        to_update.push((
                            entity.to_node_id(),
                            node.build(&mut NodeClassSet::lock_global()),
                        ));
                    }
                    let mut root = NodeBuilder::new(Role::Window);
                    if let Some(name) = name {
                        root.set_name(name);
                    }
                    root.set_children(root_children);
                    let root = root.build(&mut NodeClassSet::lock_global());
                    let window_update = (primary_window_id.to_node_id(), root);
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

/// Implements winit-specific `AccessKit` functionality.
pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AccessKitAdapters>()
            .init_resource::<WinitActionHandlers>()
            .add_event::<ActionRequestWrapper>()
            .add_systems(
                PostUpdate,
                (
                    handle_window_focus,
                    window_closed,
                    poll_receivers,
                    update_accessibility_nodes,
                ),
            );
    }
}
