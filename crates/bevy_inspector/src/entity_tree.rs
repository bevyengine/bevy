//! A panel listing the entities of the inspected world, and the systems that keep it in sync.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use bevy_dev_tools::inspection::label_resolution::{
    resolve_label, ComponentLabelData, LabelDefinitionPriority, LabelResolutionRegistry,
};
use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::{Observer, On},
    query::With,
    reflect::{ReflectComponent, ReflectResource},
    resource::{IsResource, Resource},
    system::{Commands, Query, ResMut, SystemIdMarker},
    world::World,
};
use bevy_feathers::{
    containers::{subpane, subpane_body, subpane_header},
    controls::{
        FeathersTreeItem, FeathersTreeItemChildren, FeathersTreeItemHeader, FeathersTreeView,
    },
    display::caption,
};
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{percent, px, widget::Text, Node, Overflow};
use bevy_ui_widgets::{
    tree_view_expand_self_update, tree_view_self_update, TreeItem, TreeItemExpandChange,
    ValueChange,
};

use crate::InspectorSelection;

/// Marker for entities spawned by the inspector. Entities carrying it, and their descendants, are
/// hidden from the entity tree.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorUi;

/// Marker for the tree view that holds the entity tree rows.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorTreeView;

/// A row of the entity tree, and the entity it displays.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorRow {
    /// The inspected entity this row displays.
    pub source: Entity,
}

/// Marker for the text entity holding a row's label.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorRowLabel;

/// Marker for rows whose child rows are kept in sync, added the first time a row is expanded.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorRowPopulated;

/// Maps inspected entities to the tree rows displaying them, and back.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct TreeRowIndex {
    source_to_row: HashMap<Entity, Entity>,
    row_to_source: HashMap<Entity, Entity>,
}

impl TreeRowIndex {
    /// The row displaying `source`, if one exists.
    pub fn row(&self, source: Entity) -> Option<Entity> {
        self.source_to_row.get(&source).copied()
    }

    /// The inspected entity displayed by `row`, if one exists.
    pub fn source(&self, row: Entity) -> Option<Entity> {
        self.row_to_source.get(&row).copied()
    }

    /// The number of rows currently tracked.
    pub fn len(&self) -> usize {
        self.row_to_source.len()
    }

    /// Whether no rows are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.row_to_source.is_empty()
    }
}

/// Pacing of the entity tree synchronization pass.
#[derive(Resource, Debug)]
pub struct EntityTreeSync {
    /// Time between synchronization passes.
    pub timer: Timer,
    /// Forces a synchronization pass on the next update.
    pub dirty: bool,
}

impl Default for EntityTreeSync {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.25, TimerMode::Repeating),
            dirty: true,
        }
    }
}

/// A panel listing the entities of the inspected world as an expandable tree.
pub fn entity_tree_panel() -> impl Scene {
    bsn! {
        InspectorUi
        @subpane()
        Node {
            width: px(280),
            max_height: percent(100),
        }
        Children [
            @subpane_header() Children [
                @caption("Entities")
            ]
            --
            @subpane_body()
            Node {
                overflow: Overflow::scroll_y(),
            }
            Children [
                InspectorTreeView
                @FeathersTreeView
                on(tree_view_self_update)
                on(tree_view_expand_self_update)
                on(inspector_tree_selected)
                on(inspector_tree_expanded)
            ]
        ]
    }
}

/// Observer that records the inspected entity of the selected row in [`InspectorSelection`].
pub fn inspector_tree_selected(
    change: On<ValueChange<Option<Entity>>>,
    trees: Query<(), With<InspectorTreeView>>,
    rows: Query<&InspectorRow>,
    mut selection: ResMut<InspectorSelection>,
) {
    if !trees.contains(change.source) {
        return;
    }
    selection.0 = change
        .value
        .and_then(|row| rows.get(row).ok())
        .map(|row| row.source);
}

/// Observer that starts keeping a row's child rows in sync the first time it is expanded.
pub fn inspector_tree_expanded(
    change: On<TreeItemExpandChange>,
    rows: Query<(), With<InspectorRow>>,
    mut sync: ResMut<EntityTreeSync>,
    mut commands: Commands,
) {
    if !change.expanded || !rows.contains(change.item) {
        return;
    }
    commands.entity(change.item).insert(InspectorRowPopulated);
    sync.dirty = true;
}

/// Adds, removes and relabels tree rows so that they match the inspected world.
pub fn sync_entity_tree(world: &mut World) {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();

    let run = {
        let mut sync = world.resource_mut::<EntityTreeSync>();
        sync.timer.tick(delta);
        let run = sync.dirty || sync.timer.just_finished();
        sync.dirty = false;
        run
    };
    if !run {
        return;
    }

    let plan = plan_sync(world);
    apply_sync(world, plan);
    rebuild_index(world);
}

struct RowSpawn {
    container: Entity,
    source: Entity,
    label: String,
    expandable: bool,
}

#[derive(Default)]
struct SyncPlan {
    despawn: Vec<Entity>,
    spawn: Vec<RowSpawn>,
    relabel: Vec<(Entity, String)>,
    expandable: Vec<(Entity, bool)>,
}

fn plan_sync(world: &World) -> SyncPlan {
    let mut plan = SyncPlan::default();
    let mut tree_view = None;
    let mut roots = Vec::new();
    let mut populated = Vec::new();

    for entity_ref in world.iter_entities() {
        let entity = entity_ref.id();
        if entity_ref.contains::<InspectorTreeView>() {
            tree_view = Some(entity);
        }
        if entity_ref.contains::<InspectorRow>() && entity_ref.contains::<InspectorRowPopulated>() {
            populated.push(entity);
        }
        if !entity_ref.contains::<ChildOf>() && !is_excluded(world, entity) {
            roots.push(entity);
        }
    }

    let Some(tree_view) = tree_view else {
        return plan;
    };

    roots.sort_unstable_by_key(|root| root.index());
    diff_container(world, tree_view, &roots, &mut plan);

    for row in populated {
        let Some(source) = world.get::<InspectorRow>(row).map(|row| row.source) else {
            continue;
        };
        if world.get_entity(source).is_err() {
            continue;
        }
        let Some(container) = child_with::<FeathersTreeItemChildren>(world, row) else {
            continue;
        };
        let expected = visible_children(world, source);
        diff_container(world, container, &expected, &mut plan);
    }

    plan
}

fn diff_container(world: &World, container: Entity, expected: &[Entity], plan: &mut SyncPlan) {
    let mut existing: HashMap<Entity, Entity> = HashMap::new();
    if let Some(children) = world.get::<Children>(container) {
        for child in children.iter().copied() {
            if let Some(row) = world.get::<InspectorRow>(child) {
                existing.insert(row.source, child);
            }
        }
    }

    for (source, row) in existing.iter() {
        if !expected.contains(source) {
            plan.despawn.push(*row);
        }
    }

    for source in expected.iter().copied() {
        let expandable = !visible_children(world, source).is_empty();
        let label = entity_label(world, source);
        let Some(row) = existing.get(&source).copied() else {
            plan.spawn.push(RowSpawn {
                container,
                source,
                label,
                expandable,
            });
            continue;
        };

        if let Some(label_entity) = row_label_entity(world, row)
            && world
                .get::<Text>(label_entity)
                .is_some_and(|text| text.0 != label)
        {
            plan.relabel.push((label_entity, label));
        }

        if world
            .get::<TreeItem>(row)
            .is_some_and(|item| item.expandable != expandable)
        {
            plan.expandable.push((row, expandable));
        }
    }
}

fn apply_sync(world: &mut World, plan: SyncPlan) {
    for row in plan.despawn {
        if let Ok(row) = world.get_entity_mut(row) {
            row.despawn();
        }
    }

    for (label_entity, label) in plan.relabel {
        if let Some(mut text) = world.get_mut::<Text>(label_entity) {
            text.0 = label;
        }
    }

    for (row, expandable) in plan.expandable {
        if let Some(mut item) = world.get_mut::<TreeItem>(row) {
            item.expandable = expandable;
        }
    }

    for spawn in plan.spawn {
        if world.get_entity(spawn.container).is_err() {
            continue;
        }
        match world.spawn_scene(entity_row(spawn.label, spawn.expandable)) {
            Ok(mut row) => {
                row.insert((
                    InspectorRow {
                        source: spawn.source,
                    },
                    ChildOf(spawn.container),
                ));
            }
            Err(error) => warn!("failed to spawn an inspector tree row: {error}"),
        }
    }
}

fn rebuild_index(world: &mut World) {
    let mut source_to_row = HashMap::new();
    let mut row_to_source = HashMap::new();
    for entity_ref in world.iter_entities() {
        if let Some(row) = entity_ref.get::<InspectorRow>() {
            source_to_row.insert(row.source, entity_ref.id());
            row_to_source.insert(entity_ref.id(), row.source);
        }
    }

    let mut index = world.resource_mut::<TreeRowIndex>();
    index.source_to_row = source_to_row;
    index.row_to_source = row_to_source;
}

fn entity_row(label: String, expandable: bool) -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @expandable: {expandable},
            @label: bsn_list! { @row_label({label}) },
        }
    }
}

fn row_label(text: String) -> impl Scene {
    bsn! {
        InspectorRowLabel
        @caption(text)
    }
}

fn is_excluded(world: &World, entity: Entity) -> bool {
    let Ok(entity_ref) = world.get_entity(entity) else {
        return true;
    };
    if entity_ref.contains::<InspectorUi>()
        || entity_ref.contains::<IsResource>()
        || entity_ref.contains::<SystemIdMarker>()
        || entity_ref.contains::<Observer>()
    {
        return true;
    }

    let mut current = entity;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        current = child_of.parent();
        match world.get_entity(current) {
            Ok(parent) => {
                if parent.contains::<InspectorUi>() {
                    return true;
                }
            }
            Err(_) => return true,
        }
    }
    false
}

fn visible_children(world: &World, entity: Entity) -> Vec<Entity> {
    world
        .get::<Children>(entity)
        .map(|children| {
            children
                .iter()
                .copied()
                .filter(|child| !is_excluded(world, *child))
                .collect()
        })
        .unwrap_or_default()
}

fn entity_label(world: &World, entity: Entity) -> String {
    let Ok(components) = world.inspect_entity(entity) else {
        return entity.to_string();
    };
    let registry = world.get_resource::<LabelResolutionRegistry>();

    let component_data: Vec<(ComponentId, String, Option<LabelDefinitionPriority>)> = components
        .map(|(component_id, info)| {
            let priority = info
                .type_id()
                .and_then(|type_id| registry?.get_priority_by_type_id(type_id));
            (component_id, info.name().shortname().to_string(), priority)
        })
        .collect();

    let label_data: Vec<ComponentLabelData> = component_data
        .iter()
        .map(|(component_id, short_name, priority)| ComponentLabelData {
            component_id: *component_id,
            short_name: short_name.as_str(),
            label_definition_priority: *priority,
        })
        .collect();

    resolve_label(world, entity, &label_data)
        .map(|label| label.label.as_str().to_string())
        .unwrap_or_else(|| entity.to_string())
}

fn row_label_entity(world: &World, row: Entity) -> Option<Entity> {
    let header = child_with::<FeathersTreeItemHeader>(world, row)?;
    child_with::<InspectorRowLabel>(world, header)
}

fn child_with<C: Component>(world: &World, entity: Entity) -> Option<Entity> {
    world
        .get::<Children>(entity)?
        .iter()
        .copied()
        .find(|child| {
            world
                .get_entity(*child)
                .is_ok_and(|child| child.contains::<C>())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InspectorPlugin;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::name::Name;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            InspectorPlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app
    }

    fn row_sources(world: &World, container: Entity) -> Vec<Entity> {
        world
            .get::<Children>(container)
            .map(|children| {
                children
                    .iter()
                    .copied()
                    .filter_map(|child| world.get::<InspectorRow>(child).map(|row| row.source))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn excludes_inspector_ui_and_its_descendants() {
        let mut world = World::new();
        let panel = world.spawn(InspectorUi).id();
        let inner = world.spawn(ChildOf(panel)).id();
        let subject = world.spawn_empty().id();

        assert!(is_excluded(&world, panel));
        assert!(is_excluded(&world, inner));
        assert!(!is_excluded(&world, subject));
    }

    #[test]
    fn syncs_root_rows_with_the_world() {
        let mut app = test_app();
        let tree = app.world_mut().spawn((InspectorUi, InspectorTreeView)).id();
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        app.world_mut().spawn((Name::new("Child"), ChildOf(parent)));
        let sibling = app.world_mut().spawn(Name::new("Sibling")).id();

        app.update();

        let sources = row_sources(app.world(), tree);
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&parent));
        assert!(sources.contains(&sibling));

        let parent_row = app.world().resource::<TreeRowIndex>().row(parent).unwrap();
        assert!(app.world().get::<TreeItem>(parent_row).unwrap().expandable);
        let sibling_row = app.world().resource::<TreeRowIndex>().row(sibling).unwrap();
        assert!(!app.world().get::<TreeItem>(sibling_row).unwrap().expandable);

        app.world_mut().entity_mut(sibling).despawn();
        app.world_mut().resource_mut::<EntityTreeSync>().dirty = true;
        app.update();

        let sources = row_sources(app.world(), tree);
        assert_eq!(sources, [parent]);
        assert!(app
            .world()
            .resource::<TreeRowIndex>()
            .row(sibling)
            .is_none());
    }
}
