use alloc::{string::String, vec, vec::Vec};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Commands, Query, SystemParam},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    px, widget::Text, Display, FlexDirection, GridTrack, Node, Overflow, UiRect, UiSystems, Val,
};

use crate::{
    ControlOrientation, DockError, DockLeaf, DockNode, DockNodeId, DockPanelKey, DockSplit,
    DockTab, DockTabId, DockTree, Pane, SelectedTab, SplitPane, SplitPaneHandle, Tab, TabDragMode,
    TabList, TabMoved, ValueChange,
};

/// Builds and maintains UI for every entity with a [`DockTree`].
///
/// The built UI is spawned as children of the entity holding the tree, which should also have
/// a [`Node`]. It is rebuilt whenever the tree changes, reusing the entities of nodes and tabs
/// that are still present.
#[derive(Default)]
pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(forward_split_change)
            .add_observer(forward_tab_change)
            .add_observer(forward_tab_move)
            .configure_sets(
                PostUpdate,
                (DockSystems::Structure, DockSystems::Panels)
                    .chain()
                    .before(crate::split_pane::update_split_pane_layout)
                    .before(UiSystems::Prepare),
            )
            .add_systems(PostUpdate, build_dock_ui.in_set(DockSystems::Structure))
            .add_systems(
                PostUpdate,
                sync_dock_titles
                    .after(DockSystems::Panels)
                    .before(crate::split_pane::update_split_pane_layout)
                    .before(UiSystems::Prepare),
            );
    }
}

/// The stages the dock runs in, in [`PostUpdate`].
///
/// The dock builds its structure first and then runs the app's panel builders, so builders see
/// content that is already spawned and parented. Both stages run before the UI is laid out.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockSystems {
    /// Builds the UI for changed [`DockTree`]s.
    Structure,
    /// The stage apps fill [`DockPanelContent`] entities in.
    ///
    /// The dock spawns a tab's content entity when the tab appears, and again if its panel key
    /// changes. Systems here match [`Added<DockPanelContent>`](bevy_ecs::query::Added) and run
    /// before the UI layout, so a panel is on screen in the frame it appears.
    ///
    /// A builder in plain [`PostUpdate`], or in `Update`, still gets every panel, but runs after
    /// layout, so the panel is empty for a frame.
    ///
    /// ```
    /// # use bevy_app::{App, PostUpdate};
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ui_widgets::{DockPanelContent, DockPanelTitle, DockSystems};
    /// fn build_panels(
    ///     panels: Query<(Entity, &DockPanelContent), Added<DockPanelContent>>,
    ///     mut commands: Commands,
    /// ) {
    ///     for (content, panel) in &panels {
    ///         if panel.panel.as_str() == "outliner" {
    ///             commands
    ///                 .entity(content)
    ///                 .insert(DockPanelTitle::new("Outliner"));
    ///         }
    ///     }
    /// }
    ///
    /// fn main() {
    ///     App::new().add_systems(PostUpdate, build_panels.in_set(DockSystems::Panels));
    /// }
    /// ```
    Panels,
}

/// Links a split or leaf entity built by the dock to its node in the tree.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Clone, Debug, PartialEq)]
pub struct DockNodeBinding {
    /// The entity holding the [`DockTree`].
    pub dock: Entity,
    /// The node this entity shows.
    pub node: DockNodeId,
}

/// Links a [`Tab`] entity built by the dock to its tab in the tree.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Clone, Debug, PartialEq)]
pub struct DockTabBinding {
    /// The entity holding the [`DockTree`].
    pub dock: Entity,
    /// The tab this entity shows.
    pub tab: DockTabId,
}

/// The content entity of a tab.
///
/// The dock spawns it when the tab appears and despawns it when the tab is removed, so the panel
/// keeps its state while the tab exists. Observe [`Remove`](bevy_ecs::lifecycle::Remove) to clean
/// up after it.
///
/// Fill it from a system in [`DockSystems::Panels`] that matches
/// [`Added<DockPanelContent>`](bevy_ecs::query::Added). The dock does not touch it afterwards, so
/// you can replace its [`Node`] or attach a scene.
///
/// The `Node` it spawns with fills the area below the tabs and can shrink below its own content,
/// so a panel wraps instead of spilling out of a narrow tab. A replacement `Node` should keep
/// `min_width` and `min_height` at `px(0)` for the same reason.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[component(immutable)]
#[reflect(Component, Clone, Debug, PartialEq)]
pub struct DockPanelContent {
    /// The entity holding the [`DockTree`].
    pub dock: Entity,
    /// The tab this content belongs to.
    pub tab: DockTabId,
    /// The panel the tab shows.
    pub panel: DockPanelKey,
}

/// The title shown on the tab of a [`DockPanelContent`] entity.
///
/// Tabs with no title show their [`DockPanelKey`]. The tab updates when the title changes, so a
/// title can mark a document as unsaved.
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Clone, Debug, Default, PartialEq)]
pub struct DockPanelTitle(pub String);

impl DockPanelTitle {
    /// Creates a title from a string.
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
}

/// A change the user asked for through the dock UI, sent to the entity holding the
/// [`DockTree`].
///
/// The dock UI is controlled: it shows what the tree says and does not change the tree
/// itself. Add [`dock_self_update`] as an observer on the dock entity to apply these changes,
/// or observe this event and edit the tree yourself.
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct DockChange {
    /// The entity holding the [`DockTree`].
    #[event_target]
    pub dock: Entity,
    /// The requested change.
    pub edit: DockEdit,
    /// False while a splitter is being dragged, true once the interaction is finished.
    pub is_final: bool,
}

/// A change to a [`DockTree`], as reported by [`DockChange`].
#[derive(Clone, Debug, PartialEq)]
pub enum DockEdit {
    /// New sizes for the children of a split, from dragging a splitter.
    Resize {
        /// The split being resized.
        split: DockNodeId,
        /// One size per child, as taken by [`DockTree::set_sizes`].
        sizes: Vec<f32>,
    },
    /// A tab was selected in a leaf.
    Activate {
        /// The leaf holding the tab.
        leaf: DockNodeId,
        /// The selected tab.
        tab: DockTabId,
    },
    /// A tab was dragged to a position in a leaf, as taken by [`DockTree::move_tab`].
    MoveTab {
        /// The dragged tab.
        tab: DockTabId,
        /// The leaf it was dropped on, which may be the leaf it came from.
        to_leaf: DockNodeId,
        /// The insertion index, or `None` for the end.
        index: Option<usize>,
    },
}

impl DockEdit {
    /// Applies this change to a tree.
    pub fn apply(&self, tree: &mut DockTree) -> Result<(), DockError> {
        match self {
            DockEdit::Resize { split, sizes } => tree.set_sizes(*split, sizes),
            DockEdit::Activate { leaf, tab } => tree.set_active(*leaf, *tab),
            DockEdit::MoveTab {
                tab,
                to_leaf,
                index,
            } => tree.move_tab(*tab, *to_leaf, *index),
        }
    }
}

/// Observer that applies each [`DockChange`] to the dock's [`DockTree`].
pub fn dock_self_update(change: On<DockChange>, mut docks: Query<&mut DockTree>) {
    if let Ok(mut tree) = docks.get_mut(change.dock) {
        change.edit.apply(&mut tree).ok();
    }
}

fn forward_split_change(
    change: On<ValueChange<Vec<f32>>>,
    splits: Query<&DockNodeBinding, With<SplitPane>>,
    mut commands: Commands,
) {
    let Ok(binding) = splits.get(change.source) else {
        return;
    };
    commands.trigger(DockChange {
        dock: binding.dock,
        edit: DockEdit::Resize {
            split: binding.node,
            sizes: change.value.clone(),
        },
        is_final: change.is_final,
    });
}

fn forward_tab_change(
    change: On<ValueChange<Option<Entity>>>,
    tabs: Query<&DockTabBinding>,
    docks: Query<&DockTree>,
    mut commands: Commands,
) {
    let Some(binding) = change.value.and_then(|tab| tabs.get(tab).ok()) else {
        return;
    };
    let Some(leaf) = docks
        .get(binding.dock)
        .ok()
        .and_then(|tree| tree.find_leaf_for_tab(binding.tab))
    else {
        return;
    };
    commands.trigger(DockChange {
        dock: binding.dock,
        edit: DockEdit::Activate {
            leaf,
            tab: binding.tab,
        },
        is_final: change.is_final,
    });
}

fn forward_tab_move(
    moved: On<TabMoved>,
    tabs: Query<&DockTabBinding>,
    parents: Query<&ChildOf>,
    leaves: Query<&DockNodeBinding>,
    mut commands: Commands,
) {
    let Ok(binding) = tabs.get(moved.tab) else {
        return;
    };
    let Some(leaf) = parents
        .get(moved.to_strip)
        .ok()
        .and_then(|parent| leaves.get(parent.parent()).ok())
        .filter(|leaf| leaf.dock == binding.dock)
    else {
        return;
    };
    commands.trigger(DockChange {
        dock: binding.dock,
        edit: DockEdit::MoveTab {
            tab: binding.tab,
            to_leaf: leaf.node,
            index: Some(moved.index),
        },
        is_final: true,
    });
}

#[derive(Component, Default)]
struct DockUi {
    root: Option<Entity>,
    nodes: HashMap<DockNodeId, NodeUi>,
    tabs: HashMap<DockTabId, TabUi>,
}

enum NodeUi {
    Split {
        entity: Entity,
        orientation: ControlOrientation,
        handles: Vec<Entity>,
    },
    Leaf {
        entity: Entity,
        tab_list: Entity,
        /// Holds the content of the active tab, if any.
        area: Entity,
        /// Holds the content of the inactive tabs, out of the layout.
        hidden: Entity,
    },
}

impl NodeUi {
    fn entity(&self) -> Entity {
        match self {
            NodeUi::Split { entity, .. } | NodeUi::Leaf { entity, .. } => *entity,
        }
    }

    fn matches(&self, node: &DockNode) -> bool {
        match (self, node) {
            (NodeUi::Split { orientation, .. }, DockNode::Split(split)) => {
                *orientation == split.orientation()
            }
            (NodeUi::Leaf { .. }, DockNode::Leaf(_)) => true,
            _ => false,
        }
    }
}

struct TabUi {
    panel: DockPanelKey,
    header: Entity,
    content: Entity,
}

#[derive(SystemParam)]
struct DockBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    panes: Query<'w, 's, &'static Pane>,
    selected: Query<'w, 's, &'static SelectedTab>,
    nodes: Query<'w, 's, &'static mut Node>,
}

fn build_dock_ui(
    mut docks: Query<(Entity, &DockTree, Option<&mut DockUi>), Changed<DockTree>>,
    mut builder: DockBuilder,
) {
    for (dock, tree, ui) in &mut docks {
        match ui {
            Some(mut ui) => builder.sync(dock, tree, &mut ui),
            None => {
                let mut ui = DockUi::default();
                builder.sync(dock, tree, &mut ui);
                builder.commands.entity(dock).insert(ui);
            }
        }
    }
}

fn base_node() -> Node {
    Node {
        flex_grow: 1.0,
        min_width: px(0),
        min_height: px(0),
        ..Default::default()
    }
}

impl DockBuilder<'_, '_> {
    fn sync(&mut self, dock: Entity, tree: &DockTree, ui: &mut DockUi) {
        let order = tree.iter_dfs();
        let mut stale = Vec::new();
        for &(id, _) in &order {
            if let Some(node) = tree.node(id) {
                self.ensure_node(dock, id, node, ui, &mut stale);
            }
        }

        let mut live_tabs = HashSet::new();
        for &(id, _) in &order {
            match tree.node(id) {
                Some(DockNode::Split(split)) => self.sync_split(id, split, ui),
                Some(DockNode::Leaf(leaf)) => {
                    self.sync_leaf(dock, id, leaf, ui, &mut live_tabs, &mut stale);
                }
                None => {}
            }
        }

        let root = ui.nodes[&tree.root()].entity();
        if ui.root != Some(root) {
            self.commands
                .entity(root)
                .remove::<Pane>()
                .insert(ChildOf(dock));
            self.set_size(root, Val::Auto, Val::Auto);
            if let Ok(mut node) = self.nodes.get_mut(root) {
                let base = base_node();
                node.flex_grow = base.flex_grow;
                node.flex_basis = Val::Auto;
                node.min_width = base.min_width;
                node.min_height = base.min_height;
            }
            ui.root = Some(root);
        }

        ui.tabs.retain(|id, tab| {
            let live = live_tabs.contains(id);
            if !live {
                self.commands.entity(tab.header).try_despawn();
                self.commands.entity(tab.content).try_despawn();
            }
            live
        });
        let live_nodes: HashSet<DockNodeId> = order.iter().map(|(id, _)| *id).collect();
        ui.nodes.retain(|id, node| {
            let live = live_nodes.contains(id);
            if !live {
                self.commands.entity(node.entity()).try_despawn();
            }
            live
        });
        for entity in stale {
            self.commands.entity(entity).try_despawn();
        }
    }

    fn ensure_node(
        &mut self,
        dock: Entity,
        id: DockNodeId,
        node: &DockNode,
        ui: &mut DockUi,
        stale: &mut Vec<Entity>,
    ) {
        match ui.nodes.get(&id) {
            Some(existing) if existing.matches(node) => return,
            Some(existing) => stale.push(existing.entity()),
            None => {}
        }
        let binding = DockNodeBinding { dock, node: id };
        let entry = match node {
            DockNode::Split(split) => NodeUi::Split {
                entity: self
                    .commands
                    .spawn((
                        binding,
                        SplitPane {
                            orientation: split.orientation(),
                        },
                        base_node(),
                    ))
                    .id(),
                orientation: split.orientation(),
                handles: Vec::new(),
            },
            DockNode::Leaf(_) => {
                let entity = self
                    .commands
                    .spawn((
                        binding,
                        Node {
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::clip(),
                            ..base_node()
                        },
                    ))
                    .id();
                let tab_list = self
                    .commands
                    .spawn((
                        TabList {
                            drag: TabDragMode::External,
                            ..Default::default()
                        },
                        Node {
                            flex_shrink: 0.0,
                            overflow: Overflow::clip(),
                            ..Default::default()
                        },
                        ChildOf(entity),
                    ))
                    .id();
                // A single grid cell stretches the content to fill the area, so the dock never
                // has to grow it. Sizes the app sets on its own `Node` still win.
                let area = self
                    .commands
                    .spawn((
                        Node {
                            display: Display::Grid,
                            grid_template_columns: vec![GridTrack::flex(1.0)],
                            grid_template_rows: vec![GridTrack::flex(1.0)],
                            overflow: Overflow::clip(),
                            ..base_node()
                        },
                        ChildOf(entity),
                    ))
                    .id();
                // Inactive tabs keep their content here. `Display::None` takes it out of layout,
                // and moving it to a node of our own means we never touch the content's `Node`.
                let hidden = self
                    .commands
                    .spawn((
                        Node {
                            display: Display::None,
                            ..Default::default()
                        },
                        ChildOf(entity),
                    ))
                    .id();
                NodeUi::Leaf {
                    entity,
                    tab_list,
                    area,
                    hidden,
                }
            }
        };
        ui.nodes.insert(id, entry);
    }

    fn sync_split(&mut self, id: DockNodeId, split: &DockSplit, ui: &mut DockUi) {
        let children: Vec<(Entity, f32)> = split
            .children()
            .iter()
            .map(|child| (ui.nodes[&child.node].entity(), child.size))
            .collect();
        let Some(NodeUi::Split {
            entity, handles, ..
        }) = ui.nodes.get_mut(&id)
        else {
            return;
        };
        let handle_count = children.len().saturating_sub(1);
        while handles.len() > handle_count {
            if let Some(handle) = handles.pop() {
                self.commands.entity(handle).try_despawn();
            }
        }
        while handles.len() < handle_count {
            let (width, height) = match split.orientation() {
                ControlOrientation::Horizontal => (px(4), Val::Auto),
                ControlOrientation::Vertical => (Val::Auto, px(4)),
            };
            let node = Node {
                width,
                height,
                flex_shrink: 0.0,
                ..Default::default()
            };
            handles.push(self.commands.spawn((SplitPaneHandle, node)).id());
        }

        let (width, height) = match split.orientation() {
            ControlOrientation::Horizontal => (px(0), Val::Auto),
            ControlOrientation::Vertical => (Val::Auto, px(0)),
        };
        let mut ordered = Vec::with_capacity(children.len() + handle_count);
        for (index, &(child, size)) in children.iter().enumerate() {
            if index > 0 {
                ordered.push(handles[index - 1]);
            }
            ordered.push(child);
            let pane = Pane {
                size,
                min_size: 0.0,
            };
            if self.panes.get(child) != Ok(&pane) {
                self.commands.entity(child).insert(pane);
            }
            self.set_size(child, width, height);
        }
        self.set_children(*entity, &ordered);
    }

    fn sync_leaf(
        &mut self,
        dock: Entity,
        id: DockNodeId,
        leaf: &DockLeaf,
        ui: &mut DockUi,
        live_tabs: &mut HashSet<DockTabId>,
        stale: &mut Vec<Entity>,
    ) {
        let Some(&NodeUi::Leaf {
            tab_list,
            area,
            hidden,
            ..
        }) = ui.nodes.get(&id)
        else {
            return;
        };
        let mut headers = Vec::with_capacity(leaf.tabs().len());
        let mut inactive = Vec::with_capacity(leaf.tabs().len());
        let mut selected = None;
        for tab in leaf.tabs() {
            let active = leaf.active() == Some(tab.id);
            live_tabs.insert(tab.id);
            let tab_ui = match ui.tabs.get(&tab.id) {
                Some(tab_ui) if tab_ui.panel == tab.panel => tab_ui,
                existing => {
                    if let Some(old) = existing {
                        stale.extend([old.header, old.content]);
                    }
                    let tab_ui = self.spawn_tab(dock, tab);
                    ui.tabs.entry(tab.id).insert(tab_ui).into_mut()
                }
            };
            headers.push(tab_ui.header);
            if active {
                selected = Some((tab_ui.header, tab_ui.content));
            } else {
                inactive.push(tab_ui.content);
            }
        }
        let selected_tab = SelectedTab(selected.map(|(header, _)| header));
        if self.selected.get(tab_list) != Ok(&selected_tab) {
            self.commands.entity(tab_list).insert(selected_tab);
        }
        self.set_children(tab_list, &headers);
        let active: Vec<Entity> = selected.map(|(_, content)| content).into_iter().collect();
        self.set_children(area, &active);
        self.set_children(hidden, &inactive);
    }

    fn spawn_tab(&mut self, dock: Entity, tab: &DockTab) -> TabUi {
        let header = self
            .commands
            .spawn((
                Tab,
                DockTabBinding { dock, tab: tab.id },
                Node {
                    padding: UiRect::axes(px(8), px(4)),
                    flex_shrink: 0.0,
                    ..Default::default()
                },
                Text::new(tab.panel.as_str()),
            ))
            .id();
        let content = self
            .commands
            .spawn((
                DockPanelContent {
                    dock,
                    tab: tab.id,
                    panel: tab.panel.clone(),
                },
                // An auto minimum would size the content to its own content, so a wide panel
                // would spill out of a narrow tab instead of wrapping.
                Node {
                    min_width: px(0),
                    min_height: px(0),
                    ..Default::default()
                },
            ))
            .id();
        TabUi {
            panel: tab.panel.clone(),
            header,
            content,
        }
    }

    fn set_size(&mut self, entity: Entity, width: Val, height: Val) {
        match self.nodes.get_mut(entity) {
            Ok(mut node) => {
                if node.width != width || node.height != height {
                    node.width = width;
                    node.height = height;
                }
            }
            Err(_) => {
                self.commands
                    .entity(entity)
                    .entry::<Node>()
                    .and_modify(move |mut node| {
                        node.width = width;
                        node.height = height;
                    });
            }
        }
    }

    fn set_children(&mut self, parent: Entity, children: &[Entity]) {
        let current = self.children.get(parent).map_or(&[][..], |c| &c[..]);
        if current == children {
            return;
        }
        for &child in children {
            if self.parents.get(child).map(ChildOf::parent) != Ok(parent) {
                self.commands.entity(child).insert(ChildOf(parent));
            }
        }
        self.commands.entity(parent).replace_children(children);
    }
}

/// Shows the [`DockPanelTitle`] of each content entity on its tab, or the panel key when there is
/// none.
fn sync_dock_titles(
    changed: Query<Entity, Changed<DockPanelTitle>>,
    mut removed: RemovedComponents<DockPanelTitle>,
    contents: Query<(&DockPanelContent, Option<&DockPanelTitle>)>,
    docks: Query<&DockUi>,
    mut texts: Query<&mut Text>,
) {
    for entity in changed.iter().chain(removed.read()) {
        let Ok((content, title)) = contents.get(entity) else {
            continue;
        };
        let Some(tab) = docks
            .get(content.dock)
            .ok()
            .and_then(|ui| ui.tabs.get(&content.tab))
            .filter(|tab| tab.content == entity)
        else {
            continue;
        };
        let title = title.map_or(content.panel.as_str(), |title| title.0.as_str());
        if let Ok(mut text) = texts.get_mut(tab.header)
            && text.0 != title
        {
            text.0 = title.into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DockEdge;
    use alloc::vec;
    use bevy_ecs::{query::Added, resource::Resource, system::ResMut, world::World};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(DockPlugin);
        app
    }

    fn spawn_dock(app: &mut App, tree: DockTree) -> Entity {
        let dock = app
            .world_mut()
            .spawn((tree, Node::default()))
            .observe(dock_self_update)
            .id();
        app.update();
        dock
    }

    fn edit(app: &mut App, dock: Entity, f: impl FnOnce(&mut DockTree)) {
        f(&mut app.world_mut().get_mut::<DockTree>(dock).unwrap());
        app.update();
    }

    fn children(world: &World, entity: Entity) -> Vec<Entity> {
        world
            .get::<Children>(entity)
            .map(|c| c.to_vec())
            .unwrap_or_default()
    }

    fn node_entity(world: &mut World, node: DockNodeId) -> Entity {
        let mut query = world.query::<(Entity, &DockNodeBinding)>();
        query
            .iter(world)
            .find(|(_, b)| b.node == node)
            .map(|(e, _)| e)
            .unwrap()
    }

    fn content_entity(world: &mut World, tab: DockTabId) -> Entity {
        let mut query = world.query::<(Entity, &DockPanelContent)>();
        query
            .iter(world)
            .find(|(_, c)| c.tab == tab)
            .map(|(e, _)| e)
            .unwrap()
    }

    fn header_entity(world: &mut World, tab: DockTabId) -> Entity {
        let mut query = world.query::<(Entity, &DockTabBinding)>();
        query
            .iter(world)
            .find(|(_, c)| c.tab == tab)
            .map(|(e, _)| e)
            .unwrap()
    }

    /// Whether a content entity is in its leaf's visible area rather than the hidden node.
    fn shown(world: &World, content: Entity) -> bool {
        world.get::<Node>(parent(world, content)).unwrap().display != Display::None
    }

    fn parent(world: &World, entity: Entity) -> Entity {
        world.get::<ChildOf>(entity).unwrap().parent()
    }

    /// One group with two tabs, the second one active.
    fn two_tabs() -> (DockTree, DockTabId, DockTabId) {
        let mut tree = DockTree::new();
        let root = tree.root();
        let a = tree.add_tab(root, "a").unwrap();
        let b = tree.add_tab(root, "b").unwrap();
        (tree, a, b)
    }

    fn three_way() -> (DockTree, DockNodeId, DockNodeId, DockNodeId) {
        let mut tree = DockTree::new();
        let center = tree.root();
        tree.add_tab(center, "a").unwrap();
        tree.add_tab(center, "b").unwrap();
        let (left, _) = tree.split(center, DockEdge::Left, "c").unwrap();
        let (right, _) = tree.split(center, DockEdge::Right, "d").unwrap();
        (tree, left, center, right)
    }

    #[test]
    fn builds_structure() {
        let mut app = app();
        let (tree, left, center, right) = three_way();
        let split = tree.root();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();

        let split_entity = node_entity(world, split);
        assert_eq!(children(world, dock), vec![split_entity]);
        assert!(world.get::<SplitPane>(split_entity).is_some());
        assert!(world.get::<Pane>(split_entity).is_none());

        let panes = children(world, split_entity);
        assert_eq!(panes.len(), 5);
        assert_eq!(panes[0], node_entity(world, left));
        assert_eq!(panes[2], node_entity(world, center));
        assert_eq!(panes[4], node_entity(world, right));
        assert!(world.get::<SplitPaneHandle>(panes[1]).is_some());
        assert!(world.get::<SplitPaneHandle>(panes[3]).is_some());
        assert!(world.get::<Pane>(panes[2]).is_some());
        assert_eq!(world.get::<Node>(panes[2]).unwrap().width, px(0));

        let leaf_parts = children(world, panes[2]);
        assert_eq!(leaf_parts.len(), 3);
        let tabs = children(world, leaf_parts[0]);
        assert_eq!(tabs.len(), 2);
        assert!(world.get::<TabList>(leaf_parts[0]).is_some());
        assert!(world.get::<Tab>(tabs[0]).is_some());
        assert_eq!(world.get::<Text>(tabs[0]).unwrap().0, "a");
        assert_eq!(
            world.get::<SelectedTab>(leaf_parts[0]).unwrap().0,
            Some(tabs[1])
        );
        assert_eq!(
            world.get::<Node>(leaf_parts[1]).unwrap().display,
            Display::Grid
        );
        assert_eq!(
            world.get::<Node>(leaf_parts[2]).unwrap().display,
            Display::None
        );
        let panel = |entity| world.get::<DockPanelContent>(entity).unwrap().panel.clone();
        let shown_content = children(world, leaf_parts[1])[0];
        let shown: Vec<DockPanelKey> = children(world, leaf_parts[1])
            .into_iter()
            .map(panel)
            .collect();
        let hidden: Vec<DockPanelKey> = children(world, leaf_parts[2])
            .into_iter()
            .map(panel)
            .collect();
        assert_eq!(shown, vec!["b".into()]);
        assert_eq!(hidden, vec!["a".into()]);

        // See `spawn_tab`: without these the content sizes to its own content.
        let node = world.get::<Node>(shown_content).unwrap();
        assert_eq!(node.min_width, px(0));
        assert_eq!(node.min_height, px(0));
    }

    #[derive(Component)]
    struct Built;

    /// Builds only the "a" panel.
    fn build_alpha(
        panels: Query<(Entity, &DockPanelContent), Added<DockPanelContent>>,
        mut commands: Commands,
    ) {
        for (content, panel) in &panels {
            if panel.panel.as_str() == "a" {
                commands
                    .entity(content)
                    .insert((Built, DockPanelTitle::new("Alpha")));
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct BuiltRecord {
        content: Entity,
        parent: Option<Entity>,
    }

    #[derive(Resource, Default)]
    struct BuiltLog(Vec<BuiltRecord>);

    fn log_built(
        panels: Query<Entity, Added<DockPanelContent>>,
        parents: Query<&ChildOf>,
        mut log: ResMut<BuiltLog>,
    ) {
        for content in &panels {
            log.0.push(BuiltRecord {
                content,
                parent: parents.get(content).ok().map(ChildOf::parent),
            });
        }
    }

    #[test]
    fn builds_content_once_per_tab() {
        let mut app = app();
        app.init_resource::<BuiltLog>();
        app.add_systems(PostUpdate, log_built.in_set(DockSystems::Panels));
        let (tree, a, b) = two_tabs();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();

        // Builders see each content entity once, in no particular order, and already in place.
        let built = world.resource::<BuiltLog>().0.clone();
        assert_eq!(built.len(), 2);
        for tab in [a, b] {
            let content = content_entity(world, tab);
            let record = built
                .iter()
                .find(|record| record.content == content)
                .unwrap();
            let holder = record.parent.unwrap();
            assert_eq!(holder, parent(world, content));
            assert_ne!(holder, dock);
            assert!(world.get::<Node>(holder).is_some());
        }

        // Switching tabs keeps the content, so nothing is built again.
        edit(&mut app, dock, |tree| {
            tree.set_active(tree.root(), a).unwrap();
        });
        assert_eq!(app.world().resource::<BuiltLog>().0.len(), 2);
    }

    #[test]
    fn builders_fill_content_and_titles() {
        let mut app = app();
        app.add_systems(PostUpdate, build_alpha.in_set(DockSystems::Panels));
        let (tree, a, b) = two_tabs();
        spawn_dock(&mut app, tree);
        let world = app.world_mut();

        let content = content_entity(world, a);
        assert!(world.get::<Built>(content).is_some());
        let header = header_entity(world, a);
        assert_eq!(world.get::<Text>(header).unwrap().0, "Alpha");

        let b_content = content_entity(world, b);
        assert!(world.get::<Built>(b_content).is_none());
        let header = header_entity(world, b);
        assert_eq!(world.get::<Text>(header).unwrap().0, "b");
    }

    #[test]
    fn cloned_trees_build_their_own_content() {
        // Tabs hold keys, not entities, so one tree can drive two docks with separate content.
        let mut app = app();
        app.add_systems(PostUpdate, build_alpha.in_set(DockSystems::Panels));
        let (tree, a, _) = two_tabs();
        let first = app.world_mut().spawn((tree.clone(), Node::default())).id();
        let second = app.world_mut().spawn((tree, Node::default())).id();
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(Entity, &DockPanelContent)>();
        let contents: Vec<(Entity, Entity)> = query
            .iter(world)
            .filter(|(_, content)| content.tab == a)
            .map(|(entity, content)| (content.dock, entity))
            .collect();
        assert_eq!(contents.len(), 2);
        assert!(contents.iter().any(|(dock, _)| *dock == first));
        assert!(contents.iter().any(|(dock, _)| *dock == second));
        for (_, content) in contents {
            assert!(world.get::<Built>(content).is_some());
        }
    }

    fn custom_node() -> Node {
        Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn replace_content_nodes(
        panels: Query<Entity, Added<DockPanelContent>>,
        mut commands: Commands,
    ) {
        for content in &panels {
            commands.entity(content).insert(custom_node());
        }
    }

    #[test]
    fn content_node_belongs_to_the_app() {
        let mut app = app();
        app.add_systems(
            PostUpdate,
            replace_content_nodes.in_set(DockSystems::Panels),
        );
        let (tree, a, b) = two_tabs();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let (a_content, b_content) = (content_entity(world, a), content_entity(world, b));

        // Replacing the `Node` on the first frame does not show the inactive tab.
        assert!(!shown(world, a_content));
        assert!(shown(world, b_content));

        edit(&mut app, dock, |tree| {
            tree.set_active(tree.root(), a).unwrap();
        });
        let world = app.world();
        assert!(shown(world, a_content));
        assert!(!shown(world, b_content));
        for content in [a_content, b_content] {
            assert_eq!(world.get::<Node>(content), Some(&custom_node()));
        }
    }

    #[test]
    fn tabs_follow_title_changes() {
        let mut app = app();
        app.add_systems(PostUpdate, build_alpha.in_set(DockSystems::Panels));
        let mut tree = DockTree::new();
        let root = tree.root();
        let a = tree.add_tab(root, "a").unwrap();
        spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let content = content_entity(world, a);
        let header = header_entity(world, a);

        world.get_mut::<DockPanelTitle>(content).unwrap().0 = "Alpha *".into();
        app.update();
        assert_eq!(app.world().get::<Text>(header).unwrap().0, "Alpha *");

        app.world_mut()
            .entity_mut(content)
            .remove::<DockPanelTitle>();
        app.update();
        assert_eq!(app.world().get::<Text>(header).unwrap().0, "a");

        // Removing and re-adding a title in one frame leaves the new title.
        app.world_mut()
            .entity_mut(content)
            .insert(DockPanelTitle::new("First"))
            .remove::<DockPanelTitle>()
            .insert(DockPanelTitle::new("Second"));
        app.update();
        assert_eq!(app.world().get::<Text>(header).unwrap().0, "Second");
    }

    #[test]
    fn title_follows_a_tab_moved_in_the_same_frame() {
        let mut app = app();
        app.add_systems(PostUpdate, build_alpha.in_set(DockSystems::Panels));
        let (tree, left, _, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let content = content_entity(world, a);

        world.get_mut::<DockPanelTitle>(content).unwrap().0 = "Moved".into();
        edit(&mut app, dock, |tree| tree.move_tab(a, left, None).unwrap());
        let world = app.world_mut();
        let header = header_entity(world, a);
        assert_eq!(
            parent(world, parent(world, header)),
            node_entity(world, left)
        );
        assert_eq!(world.get::<Text>(header).unwrap().0, "Moved");
    }

    #[test]
    fn titles_on_entities_the_dock_did_not_spawn_are_ignored() {
        let mut app = app();
        let mut tree = DockTree::new();
        let root = tree.root();
        let a = tree.add_tab(root, "a").unwrap();
        let dock = spawn_dock(&mut app, tree);

        app.world_mut().spawn((
            DockPanelContent {
                dock,
                tab: a,
                panel: "a".into(),
            },
            DockPanelTitle::new("Impostor"),
        ));
        app.update();
        let world = app.world_mut();
        let header = header_entity(world, a);
        assert_eq!(world.get::<Text>(header).unwrap().0, "a");
    }

    #[test]
    fn content_is_rebuilt_when_a_tab_changes_panel() {
        let mut app = app();
        app.add_systems(PostUpdate, build_alpha.in_set(DockSystems::Panels));
        let mut tree = DockTree::new();
        let root = tree.root();
        let b = tree.add_tab(root, "b").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let old_content = content_entity(world, b);

        // Ids are only unique within a tree, so replacing the whole tree can give an existing
        // tab id to a different panel.
        let mut replacement = DockTree::new();
        let root = replacement.root();
        assert_eq!(replacement.add_tab(root, "a").unwrap(), b);
        edit(&mut app, dock, |tree| *tree = replacement);

        let world = app.world_mut();
        assert!(world.get_entity(old_content).is_err());
        let content = content_entity(world, b);
        assert!(world.get::<Built>(content).is_some());
        let header = header_entity(world, b);
        assert_eq!(world.get::<Text>(header).unwrap().0, "Alpha");
    }

    #[test]
    fn reuses_entities() {
        let mut app = app();
        let (tree, left, center, right) = three_way();
        let a = tree.find_panel("a").unwrap().1;
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let before = [left, center, right].map(|id| node_entity(world, id));
        let content = content_entity(world, a);

        let mut new_leaf = None;
        edit(&mut app, dock, |tree| {
            new_leaf = Some(tree.split(center, DockEdge::Bottom, "e").unwrap().0);
        });
        let world = app.world_mut();
        let after = [left, center, right].map(|id| node_entity(world, id));
        assert_eq!(before, after);
        assert_eq!(content_entity(world, a), content);
        let column = parent(world, after[1]);
        assert_eq!(
            world.get::<SplitPane>(column).unwrap().orientation,
            ControlOrientation::Vertical
        );
        assert_eq!(
            children(world, column)[2],
            node_entity(world, new_leaf.unwrap())
        );
    }

    #[test]
    fn moves_and_hides_content() {
        let mut app = app();
        let (tree, left, center, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let (_, b) = tree.find_panel("b").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let content = content_entity(world, a);
        assert!(!shown(world, content));

        edit(&mut app, dock, |tree| tree.move_tab(a, left, None).unwrap());
        let world = app.world_mut();
        let left_entity = node_entity(world, left);
        assert_eq!(content_entity(world, a), content);
        assert_eq!(parent(world, parent(world, content)), left_entity);
        assert!(shown(world, content));
        let header = header_entity(world, a);
        assert_eq!(parent(world, parent(world, header)), left_entity);

        let b_content = content_entity(world, b);
        assert!(shown(world, b_content));
        edit(&mut app, dock, |tree| {
            let c = tree.find_panel("c").unwrap().1;
            tree.set_active(left, c).unwrap();
        });
        let world = app.world_mut();
        assert!(!shown(world, content));
        assert_eq!(
            parent(world, parent(world, b_content)),
            node_entity(world, center)
        );
    }

    #[test]
    fn despawns_removed_tabs() {
        let mut app = app();
        let (tree, _, _, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let content = content_entity(world, a);
        let header = header_entity(world, a);

        edit(&mut app, dock, |tree| {
            tree.remove_tab(a).unwrap();
        });
        let world = app.world();
        assert!(world.get_entity(content).is_err());
        assert!(world.get_entity(header).is_err());
    }

    #[test]
    fn collapses_after_simplify() {
        let mut app = app();
        let (tree, left, center, right) = three_way();
        let split = tree.root();
        let (_, c) = tree.find_panel("c").unwrap();
        let (_, d) = tree.find_panel("d").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let split_entity = node_entity(world, split);
        let center_entity = node_entity(world, center);
        let left_entity = node_entity(world, left);
        let right_entity = node_entity(world, right);

        edit(&mut app, dock, |tree| {
            tree.remove_tab(c).unwrap();
        });
        let world = app.world();
        assert!(world.get_entity(left_entity).is_err());
        assert_eq!(children(world, split_entity).len(), 3);

        edit(&mut app, dock, |tree| {
            tree.remove_tab(d).unwrap();
        });
        let world = app.world();
        assert!(world.get_entity(split_entity).is_err());
        assert!(world.get_entity(right_entity).is_err());
        assert_eq!(children(world, dock), vec![center_entity]);
        assert!(world.get::<Pane>(center_entity).is_none());
        assert_eq!(world.get::<Node>(center_entity).unwrap().flex_grow, 1.0);
        assert_eq!(world.get::<Node>(center_entity).unwrap().width, Val::Auto);
    }

    #[test]
    fn replaces_whole_tree() {
        let mut app = app();
        let (tree, _, _, _) = three_way();
        let dock = spawn_dock(&mut app, tree);
        let mut replacement = DockTree::new();
        let root = replacement.root();
        replacement.add_tab(root, "x").unwrap();
        replacement.split(root, DockEdge::Top, "y").unwrap();
        edit(&mut app, dock, |tree| *tree = replacement);

        let world = app.world_mut();
        let mut titles: Vec<String> = world
            .query_filtered::<&Text, With<Tab>>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect();
        titles.sort();
        assert_eq!(titles, vec!["x", "y"]);
        let contents = world.query::<&DockPanelContent>().iter(world).count();
        assert_eq!(contents, 2);
        let nodes = world.query::<&DockNodeBinding>().iter(world).count();
        assert_eq!(nodes, 3);
        let split = children(world, dock)[0];
        assert_eq!(
            world.get::<SplitPane>(split).unwrap().orientation,
            ControlOrientation::Vertical
        );
    }

    #[test]
    fn self_update_writes_back() {
        let mut app = app();
        let (tree, left, _, _) = three_way();
        let split = tree.root();
        let (_, c) = tree.find_panel("c").unwrap();
        let (_, a) = tree.find_panel("a").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let split_entity = node_entity(world, split);
        let a_header = header_entity(world, a);
        let center_tabs = parent(world, a_header);
        let left_entity = node_entity(world, left);

        world.trigger(ValueChange {
            source: split_entity,
            value: vec![1.0_f32, 2.0, 3.0],
            is_final: true,
        });
        world.trigger(ValueChange {
            source: center_tabs,
            value: Some(a_header),
            is_final: true,
        });
        app.update();
        let world = app.world();
        let tree = world.get::<DockTree>(dock).unwrap();
        let sizes: Vec<f32> = tree
            .node(split)
            .and_then(DockNode::as_split)
            .unwrap()
            .children()
            .iter()
            .map(|child| child.size)
            .collect();
        assert_eq!(sizes, vec![1.0, 2.0, 3.0]);
        assert_eq!(world.get::<Pane>(left_entity).unwrap().size, 1.0);
        let center = tree.find_leaf_for_tab(a).unwrap();
        assert_eq!(tree.leaf(center).unwrap().active(), Some(a));
        assert_eq!(
            world.get::<SelectedTab>(center_tabs).unwrap().0,
            Some(a_header)
        );
        assert_eq!(tree.leaf(left).unwrap().active(), Some(c));
    }

    #[test]
    fn controlled_without_self_update() {
        let mut app = app();
        let (tree, _, _, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let dock = app.world_mut().spawn((tree.clone(), Node::default())).id();
        app.update();
        let world = app.world_mut();
        let header = header_entity(world, a);
        world.trigger(ValueChange {
            source: parent(world, header),
            value: Some(header),
            is_final: true,
        });
        app.update();
        assert_eq!(app.world().get::<DockTree>(dock).unwrap(), &tree);
    }

    fn tab_list(world: &mut World, leaf: DockNodeId) -> Entity {
        let leaf = node_entity(world, leaf);
        children(world, leaf)[0]
    }

    fn drop_tab(app: &mut App, tab: DockTabId, to_leaf: DockNodeId, index: usize) {
        let world = app.world_mut();
        let header = header_entity(world, tab);
        let from_strip = parent(world, header);
        let to_strip = tab_list(world, to_leaf);
        world.trigger(TabMoved {
            from_strip,
            tab: header,
            to_strip,
            index,
        });
        app.update();
    }

    fn leaf_tabs(app: &App, dock: Entity, leaf: DockNodeId) -> Vec<DockTabId> {
        let tree = app.world().get::<DockTree>(dock).unwrap();
        tree.leaf(leaf)
            .unwrap()
            .tabs()
            .iter()
            .map(|t| t.id)
            .collect()
    }

    #[test]
    fn split_merged_into_parent_keeps_siblings() {
        let mut app = app();
        let (tree, left, center, right) = three_way();
        let split = tree.root();
        let (_, d) = tree.find_panel("d").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let mut new_leaf = None;
        edit(&mut app, dock, |tree| {
            new_leaf = Some(tree.split(center, DockEdge::Right, "e").unwrap().0);
        });
        let world = app.world_mut();
        let split_entity = node_entity(world, split);
        let panes = children(world, split_entity);
        let expected = [left, center, new_leaf.unwrap(), right].map(|id| node_entity(world, id));
        assert_eq!(panes.len(), 7);
        for (index, leaf) in expected.into_iter().enumerate() {
            assert_eq!(panes[index * 2], leaf);
            let parts = children(world, leaf);
            assert_eq!(parts.len(), 3);
            assert!(!children(world, parts[0]).is_empty());
            assert!(!children(world, parts[1]).is_empty());
        }
        for handle in [1, 3, 5] {
            assert!(world.get::<SplitPaneHandle>(panes[handle]).is_some());
        }
        let d_header = header_entity(world, d);
        assert_eq!(parent(world, parent(world, d_header)), expected[3]);
        let d_content = content_entity(world, d);
        assert_eq!(parent(world, parent(world, d_content)), expected[3]);
        assert!(shown(world, d_content));
    }

    #[test]
    fn tab_lists_drag_externally() {
        let mut app = app();
        let (tree, left, _, _) = three_way();
        spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let list = tab_list(world, left);
        assert_eq!(
            world.get::<TabList>(list).unwrap().drag,
            TabDragMode::External
        );
    }

    #[test]
    fn drags_tab_between_groups() {
        let mut app = app();
        let (tree, left, center, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let (_, b) = tree.find_panel("b").unwrap();
        let (_, c) = tree.find_panel("c").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let content = content_entity(world, a);

        drop_tab(&mut app, a, left, 0);
        assert_eq!(leaf_tabs(&app, dock, left), vec![a, c]);
        assert_eq!(leaf_tabs(&app, dock, center), vec![b]);
        let world = app.world_mut();
        let left_list = tab_list(world, left);
        let header = header_entity(world, a);
        assert_eq!(
            children(world, left_list),
            vec![header, header_entity(world, c)]
        );
        assert_eq!(world.get::<SelectedTab>(left_list).unwrap().0, Some(header));
        assert_eq!(content_entity(world, a), content);
        assert_eq!(
            parent(world, parent(world, content)),
            node_entity(world, left)
        );
        assert!(shown(world, content));
    }

    #[test]
    fn drags_tab_within_group() {
        let mut app = app();
        let (tree, _, center, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let (_, b) = tree.find_panel("b").unwrap();
        let dock = spawn_dock(&mut app, tree);

        drop_tab(&mut app, b, center, 0);
        assert_eq!(leaf_tabs(&app, dock, center), vec![b, a]);
        let world = app.world_mut();
        let list = tab_list(world, center);
        assert_eq!(
            children(world, list),
            vec![header_entity(world, b), header_entity(world, a)]
        );
    }

    #[test]
    fn drag_that_empties_group_collapses_it() {
        let mut app = app();
        let (tree, left, center, right) = three_way();
        let split = tree.root();
        let (_, c) = tree.find_panel("c").unwrap();
        let dock = spawn_dock(&mut app, tree);
        let world = app.world_mut();
        let left_entity = node_entity(world, left);
        let c_content = content_entity(world, c);

        drop_tab(&mut app, c, center, 2);
        let tree = app.world().get::<DockTree>(dock).unwrap();
        assert!(tree.node(left).is_none());
        assert_eq!(tree.leaf(center).unwrap().tabs()[2].id, c);
        let world = app.world_mut();
        assert!(world.get_entity(left_entity).is_err());
        let split_entity = node_entity(world, split);
        let panes = children(world, split_entity);
        assert_eq!(panes.len(), 3);
        assert_eq!(panes[0], node_entity(world, center));
        assert_eq!(panes[2], node_entity(world, right));
        assert_eq!(world.get_entity(c_content).map(|e| e.id()), Ok(c_content));
        assert_eq!(parent(world, parent(world, c_content)), panes[0]);
    }

    #[test]
    fn ignores_drops_on_other_docks() {
        let mut app = app();
        let (tree, left, _, _) = three_way();
        let (_, a) = tree.find_panel("a").unwrap();
        let first = spawn_dock(&mut app, tree.clone());
        let second = spawn_dock(&mut app, tree.clone());
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &DockTabBinding)>();
        let header = query
            .iter(world)
            .find(|(_, b)| b.dock == first && b.tab == a)
            .unwrap()
            .0;
        let from_strip = parent(world, header);
        let mut lists = world.query::<(&DockNodeBinding, &Children)>();
        let to_strip = lists
            .iter(world)
            .find(|(b, _)| b.dock == second && b.node == left)
            .unwrap()
            .1[0];
        world.trigger(TabMoved {
            from_strip,
            tab: header,
            to_strip,
            index: 0,
        });
        app.update();
        assert_eq!(app.world().get::<DockTree>(first).unwrap(), &tree);
        assert_eq!(app.world().get::<DockTree>(second).unwrap(), &tree);
        assert_eq!(parent(app.world(), header), from_strip);
    }

    #[test]
    fn split_layout_is_applied_in_the_same_frame() {
        let mut app = app();
        app.add_plugins(crate::SplitPanePlugin);
        let (tree, _, center, _) = three_way();
        let dock = spawn_dock(&mut app, tree);
        let layout = |app: &mut App| {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &Node)>();
            let mut nodes: Vec<_> = query
                .iter(world)
                .map(|(entity, node)| {
                    (entity, node.flex_grow, node.flex_basis, node.flex_direction)
                })
                .collect();
            nodes.sort_by_key(|(entity, ..)| *entity);
            nodes
        };

        edit(&mut app, dock, |tree| {
            tree.split(center, DockEdge::Bottom, "e").unwrap();
        });
        let after_split = layout(&mut app);
        app.update();
        assert_eq!(after_split, layout(&mut app));
    }
}
