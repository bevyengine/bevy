//! Helpers for mapping window entities to accessibility types

use alloc::{collections::VecDeque, sync::Arc};
use bevy_input_focus::InputFocus;
use core::cell::RefCell;
use std::sync::Mutex;
use winit::event_loop::ActiveEventLoop;

use accesskit::{
    ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId, Role, Tree,
    TreeId, TreeUpdate,
};
use accesskit_winit::Adapter;
use bevy_a11y::{
    AccessibilityNode, AccessibilityRequested, AccessibilitySystems,
    ActionRequest as ActionRequestWrapper, ManageAccessibilityUpdates,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{entity::EntityHashMap, prelude::*, system::NonSendMarker};
use bevy_window::{PrimaryWindow, Window, WindowClosed};

thread_local! {
    /// Temporary storage of access kit adapter data to replace usage of `!Send` resources. This will be replaced with proper
    /// storage of `!Send` data after issue #17667 is complete.
    pub static ACCESS_KIT_ADAPTERS: RefCell<AccessKitAdapters> = const { RefCell::new(AccessKitAdapters::new()) };
}

/// Maps window entities to their `AccessKit` [`Adapter`]s.
#[derive(Default, Deref, DerefMut)]
pub struct AccessKitAdapters(pub EntityHashMap<Adapter>);

impl AccessKitAdapters {
    /// Creates a new empty `AccessKitAdapters`.
    pub const fn new() -> Self {
        Self(EntityHashMap::new())
    }
}

/// Maps window entities to their respective [`ActionRequest`]s.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct WinitActionRequestHandlers(pub EntityHashMap<Arc<Mutex<WinitActionRequestHandler>>>);

/// Forwards `AccessKit` [`ActionRequest`]s from winit to an event channel.
#[derive(Clone, Default, Deref, DerefMut)]
pub struct WinitActionRequestHandler(pub VecDeque<ActionRequest>);

impl WinitActionRequestHandler {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self(VecDeque::new())))
    }
}

struct AccessKitState {
    name: String,
    entity: Entity,
    requested: AccessibilityRequested,
}

impl AccessKitState {
    fn new(
        name: impl Into<String>,
        entity: Entity,
        requested: AccessibilityRequested,
    ) -> Arc<Mutex<Self>> {
        let name = name.into();

        Arc::new(Mutex::new(Self {
            name,
            entity,
            requested,
        }))
    }

    fn build_root(&mut self) -> Node {
        let mut node = Node::new(Role::Window);
        node.set_label(self.name.clone());
        node
    }

    fn build_initial_tree(&mut self) -> TreeUpdate {
        let root = self.build_root();
        let accesskit_window_id = NodeId(self.entity.to_bits());
        let tree = Tree::new(accesskit_window_id);
        self.requested.set(true);

        TreeUpdate {
            nodes: vec![(accesskit_window_id, root)],
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus: accesskit_window_id,
        }
    }
}

struct WinitActivationHandler(Arc<Mutex<AccessKitState>>);

impl ActivationHandler for WinitActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().build_initial_tree())
    }
}

impl WinitActivationHandler {
    pub fn new(state: Arc<Mutex<AccessKitState>>) -> Self {
        Self(state)
    }
}

#[derive(Clone, Default)]
struct WinitActionHandler(Arc<Mutex<WinitActionRequestHandler>>);

impl ActionHandler for WinitActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        let mut requests = self.0.lock().unwrap();
        requests.push_back(request);
    }
}

impl WinitActionHandler {
    pub fn new(handler: Arc<Mutex<WinitActionRequestHandler>>) -> Self {
        Self(handler)
    }
}

struct WinitDeactivationHandler;

impl DeactivationHandler for WinitDeactivationHandler {
    fn deactivate_accessibility(&mut self) {}
}

/// Prepares accessibility for a winit window.
pub(crate) fn prepare_accessibility_for_window(
    event_loop: &ActiveEventLoop,
    winit_window: &winit::window::Window,
    entity: Entity,
    name: String,
    accessibility_requested: AccessibilityRequested,
    adapters: &mut AccessKitAdapters,
    handlers: &mut WinitActionRequestHandlers,
) {
    let state = AccessKitState::new(name, entity, accessibility_requested);
    let activation_handler = WinitActivationHandler::new(Arc::clone(&state));

    let action_request_handler = WinitActionRequestHandler::new();
    let action_handler = WinitActionHandler::new(Arc::clone(&action_request_handler));
    let deactivation_handler = WinitDeactivationHandler;

    let adapter = Adapter::with_direct_handlers(
        event_loop,
        winit_window,
        activation_handler,
        action_handler,
        deactivation_handler,
    );

    adapters.insert(entity, adapter);
    handlers.insert(entity, action_request_handler);
}

fn window_closed(
    mut handlers: ResMut<WinitActionRequestHandlers>,
    mut window_closed_reader: MessageReader<WindowClosed>,
    _non_send_marker: NonSendMarker,
) {
    ACCESS_KIT_ADAPTERS.with_borrow_mut(|adapters| {
        for WindowClosed { window, .. } in window_closed_reader.read() {
            adapters.remove(window);
            handlers.remove(window);
        }
    });
}

fn poll_receivers(
    handlers: Res<WinitActionRequestHandlers>,
    mut actions: MessageWriter<ActionRequestWrapper>,
) {
    for (_id, handler) in handlers.iter() {
        let mut handler = handler.lock().unwrap();
        while let Some(event) = handler.pop_front() {
            actions.write(ActionRequestWrapper(event));
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
    focus: Option<Res<InputFocus>>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    nodes: Query<(Entity, &AccessibilityNode)>,
    node_entities: Query<Entity, With<AccessibilityNode>>,
    parents: Query<&ChildOf>,
    children_query: Query<&Children>,
    _non_send_marker: NonSendMarker,
) {
    ACCESS_KIT_ADAPTERS.with_borrow_mut(|adapters| {
        let Ok((primary_window_id, primary_window)) = primary_window.single() else {
            return;
        };
        let Some(adapter) = adapters.get_mut(&primary_window_id) else {
            return;
        };
        let Some(focus) = focus else {
            return;
        };
        if focus.is_changed() || !nodes.is_empty() {
            // Don't panic if the focused entity does not currently exist
            // It's probably waiting to be spawned
            if let Some(focused_entity) = focus.get()
                && !node_entities.contains(focused_entity)
            {
                return;
            }

            adapter.update_if_active(|| {
                update_adapter(
                    &nodes,
                    &node_entities,
                    &parents,
                    &children_query,
                    primary_window,
                    primary_window_id,
                    Some(&focus),
                )
            });
        }
    });
}

fn update_adapter(
    nodes: &Query<(Entity, &AccessibilityNode)>,
    node_entities: &Query<Entity, With<AccessibilityNode>>,
    parents: &Query<&ChildOf>,
    children_query: &Query<&Children>,
    primary_window: &Window,
    primary_window_id: Entity,
    focus: Option<&InputFocus>,
) -> TreeUpdate {
    let mut to_update = vec![];
    let mut window_children = vec![];
    for (entity, node) in nodes {
        let mut node = (**node).clone();
        
        let has_accessible_ancestor = parents
            .iter_ancestors(entity)
            .any(|ancestor| node_entities.contains(ancestor));

        if !has_accessible_ancestor {
            window_children.push(NodeId(entity.to_bits()));
        }

        let mut accessible_children = vec![];
        collect_accessible_descendants(entity, children_query, node_entities, &mut accessible_children);
        if !accessible_children.is_empty() {
            node.set_children(accessible_children);
        }

        let node_id = NodeId(entity.to_bits());
        to_update.push((node_id, node));
    }
    let mut window_node = Node::new(Role::Window);
    if primary_window.focused {
        let title = primary_window.title.clone();
        window_node.set_label(title.into_boxed_str());
    }
    window_node.set_children(window_children);
    let node_id = NodeId(primary_window_id.to_bits());
    let window_update = (node_id, window_node);
    to_update.insert(0, window_update);
    TreeUpdate {
        nodes: to_update,
        tree: None,
        tree_id: TreeId::ROOT,
        focus: NodeId(
            focus
                .and_then(|f| f.get())
                .unwrap_or(primary_window_id)
                .to_bits(),
        ),
    }
}

fn collect_accessible_descendants(
    entity: Entity,
    children_query: &Query<&Children>,
    node_entities: &Query<Entity, With<AccessibilityNode>>,
    accessible_children: &mut Vec<NodeId>,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children {
            if node_entities.contains(*child) {
                accessible_children.push(NodeId(child.to_bits()));
            } else {
                collect_accessible_descendants(
                    *child,
                    children_query,
                    node_entities,
                    accessible_children,
                );
            }
        }
    }
}

/// Implements winit-specific `AccessKit` functionality.
pub struct AccessKitPlugin;

impl Plugin for AccessKitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WinitActionRequestHandlers>()
            .add_message::<ActionRequestWrapper>()
            .add_systems(
                PostUpdate,
                (
                    poll_receivers,
                    update_accessibility_nodes
                        .run_if(should_update_accessibility_nodes)
                        // This is unlikely to result in real conflicts,
                        // as FocusChangeEvents only mutates internal state of InputFocus,
                        // and update_accessibility_nodes only reads from it.
                        // However, in case this changes in the future, this is a safer choice,
                        // as accessibility updates could conceivably want to read focus change events.
                        .after(bevy_input_focus::InputFocusSystems::FocusChangeEvents),
                    window_closed
                        .before(poll_receivers)
                        .before(update_accessibility_nodes),
                )
                    .in_set(AccessibilitySystems::Update),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::Role;

    #[test]
    fn test_accessibility_hierarchy_object_nav() {
        let mut app = App::new();

        let parent_accessible = app
            .world_mut()
            .spawn(AccessibilityNode(Node::new(Role::Group)))
            .id();

        let intermediate_unannotated = app
            .world_mut()
            .spawn(ChildOf(parent_accessible))
            .id();

        let child_accessible = app
            .world_mut()
            .spawn((
                AccessibilityNode(Node::new(Role::Button)),
                ChildOf(intermediate_unannotated),
            ))
            .id();

        let primary_window = app
            .world_mut()
            .spawn((
                PrimaryWindow,
                Window {
                    focused: true,
                    title: "Test".into(),
                    ..Default::default()
                },
            ))
            .id();

        let focus = InputFocus::from_entity(primary_window);

        let mut system = IntoSystem::into_system(
            move |nodes: Query<(Entity, &AccessibilityNode)>,
                  node_entities: Query<Entity, With<AccessibilityNode>>,
                  parents: Query<&ChildOf>,
                  children_query: Query<&Children>,
                  windows: Query<(Entity, &Window), With<PrimaryWindow>>| {
                let (window_id, window) = windows.single().unwrap();
                update_adapter(
                    &nodes,
                    &node_entities,
                    &parents,
                    &children_query,
                    window,
                    window_id,
                    Some(&focus),
                )
            },
        );

        system.initialize(app.world_mut());
        let tree_update = system.run((), app.world_mut()).unwrap();

        let parent_node_id = NodeId(parent_accessible.to_bits());
        let child_node_id = NodeId(child_accessible.to_bits());

        let (_, parent_node) = tree_update
            .nodes
            .iter()
            .find(|(id, _)| *id == parent_node_id)
            .expect("Parent node should be in tree update");

        assert_eq!(parent_node.children(), &[child_node_id]);
    }
}
