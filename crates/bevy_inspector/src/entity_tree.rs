//! A panel listing the entities of the inspected world, and the systems that keep it in sync.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::cmp::Ordering;

use bevy_dev_tools::inspection::label_resolution::{
    resolve_label, ComponentLabelData, LabelDefinitionPriority, LabelResolutionRegistry,
};
use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::{Observer, On},
    query::{Or, With},
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
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{percent, px, widget::Text, Node, Overflow};
use bevy_ui_widgets::{
    tree_view_expand_self_update, tree_view_self_update, TreeItem, TreeItemExpandChange,
    ValueChange,
};

use crate::{InspectableEntity, InspectorSelection, InspectorSource, InspectorSources};

/// Marker for root entities spawned by the inspector. An `InspectorUi` entity's descendants are
/// hidden from the entity tree, but the marked entity itself still shows as a single,
/// non-expandable row.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorUi;

/// Marker for the tree view that holds the entity tree rows.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorTreeView;

/// A header row of the entity tree.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorHeaderRow {
    /// Whether the inspected entity is from.
    pub source: InspectorSource,
}

/// A row of the entity tree, and the entity it displays.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorEntityRow {
    /// The inspected entity this row displays.
    pub inspected: Entity,
    /// Whether the inspected entity is from.
    pub source: InspectorSource,
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
    source_to_row: HashMap<InspectorSource, Entity>,
    row_to_source: HashMap<Entity, InspectorSource>,

    inspectable_to_row: HashMap<InspectableEntity, Entity>,
    row_to_inspectable: HashMap<Entity, InspectableEntity>,
}

impl TreeRowIndex {
    /// The row displaying `source`, if one exists.
    pub fn row(&self, inspected: Entity, source: InspectorSource) -> Option<Entity> {
        self.inspectable_to_row
            .get(&InspectableEntity::new(inspected, source))
            .copied()
    }

    /// The inspected entity displayed by `row`, if one exists.
    pub fn source(&self, row: Entity) -> Option<&InspectableEntity> {
        self.row_to_inspectable.get(&row)
    }

    /// The number of rows currently tracked.
    pub fn len(&self) -> usize {
        self.row_to_inspectable.len()
    }

    /// Whether no rows are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.row_to_inspectable.is_empty()
    }
}

/// Pacing of the entity tree synchronization pass.
#[derive(Resource, Debug)]
pub struct EntityTreeSync {
    /// Time between synchronization passes.
    pub timer: Timer,
}

impl Default for EntityTreeSync {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.25, TimerMode::Repeating);
        timer.set_elapsed(timer.duration());
        Self { timer }
    }
}

impl EntityTreeSync {
    /// Forces a synchronization pass on the next tick.
    pub fn set_dirty(&mut self) {
        self.timer.almost_finish();
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
            @entity_tree_body()
        ]
    }
}

/// The scrolling contents of [`entity_tree_panel`], without its header.
pub fn entity_tree_body() -> impl Scene {
    bsn! {
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
    }
}

/// Observer that records the inspected entity of the selected row in [`InspectorSelection`].
pub fn inspector_tree_selected(
    change: On<ValueChange<Option<Entity>>>,
    trees: Query<(), With<InspectorTreeView>>,
    rows: Query<&InspectorEntityRow>,
    mut selection: ResMut<InspectorSelection>,
) {
    if !trees.contains(change.source) {
        return;
    }

    selection.0 = change
        .value
        .and_then(|row| rows.get(row).ok())
        .map(|row| InspectableEntity::new(row.inspected, row.source));
}

/// Observer that starts keeping a row's child rows in sync the first time it is expanded.
pub fn inspector_tree_expanded(
    change: On<TreeItemExpandChange>,
    rows: Query<(), Or<(With<InspectorHeaderRow>, With<InspectorEntityRow>)>>,
    mut sync: ResMut<EntityTreeSync>,
    mut commands: Commands,
) {
    if !change.expanded || !rows.contains(change.item) {
        return;
    }
    commands.entity(change.item).insert(InspectorRowPopulated);
    sync.set_dirty();
}

/// Adds, removes and relabels tree rows so that they match the inspected world.
pub fn sync_entity_tree(world: &mut World) {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();

    if !world
        .resource_mut::<EntityTreeSync>()
        .timer
        .tick(delta)
        .just_finished()
    {
        return;
    }

    let plan = plan_sync(world);
    let headers_spawned = !plan.header_spawn.is_empty();
    apply_sync(world, plan);
    rebuild_index(world);

    if headers_spawned {
        let plan = plan_sync(world);
        apply_sync(world, plan);
        rebuild_index(world);
    }
}

struct HeaderSpawn {
    container: Entity,
    source: InspectorSource,
    label: String,
}

struct RowSpawn {
    container: Entity,
    inspected: Entity,
    source: InspectorSource,
    label: String,
    expandable: bool,
}

/// The set of changes a single synchronization pass needs to apply.
///
/// [`plan_sync`] reads the world without mutating it, diffing the root container against the
/// current root entities and each populated row's children container against that row's entity's
/// current children (see [`diff_container`]). Rows that were never expanded, and so have no
/// children container, are not diffed. The result records which rows to despawn, which to spawn,
/// which labels to update and which `expandable` flags to flip. [`apply_sync`] then applies those
/// changes in that order and rebuilds [`TreeRowIndex`].
///
/// Each row is a [`InspectorHeaderRow`] or [`InspectorEntityRow`]
#[derive(Default)]
struct SyncPlan {
    header_despawn: Vec<Entity>,
    header_spawn: Vec<HeaderSpawn>,
    despawn: Vec<Entity>,
    spawn: Vec<RowSpawn>,
    relabel: Vec<(Entity, String)>,
    expandable: Vec<(Entity, bool)>,
}

fn rank(item: &InspectorSource) -> i32 {
    match item {
        InspectorSource::Local => 1,
        #[cfg(feature = "remote")]
        InspectorSource::Remote(_) => 2,
    }
}

fn determine_sources_order(sources: &HashSet<InspectorSource>) -> Vec<InspectorSource> {
    let mut sources: Vec<InspectorSource> = sources.into_iter().copied().collect();

    sources.sort_by(|a, b| {
        rank(a).cmp(&rank(b)).then_with(|| match (a, b) {
            #[cfg(feature = "remote")]
            (InspectorSource::Remote(val_a), InspectorSource::Remote(val_b)) => val_a.cmp(val_b),
            _ => Ordering::Equal,
        })
    });

    sources
}

/// Builds the [`SyncPlan`] for one synchronization pass.
///
/// The rows live in `world`, while the entities they display are read from the inspected world,
/// see [`crate::world_to_inspect`].
fn plan_sync(world: &World) -> SyncPlan {
    let mut plan = SyncPlan::default();
    let mut tree_view = None;
    let mut populated = Vec::new();

    let priorities = world.get_resource::<LabelResolutionRegistry>();

    for entity_ref in world.iter_entities() {
        let entity = entity_ref.id();
        if entity_ref.contains::<InspectorTreeView>() {
            tree_view = Some(entity);
        }
        if entity_ref.contains::<InspectorEntityRow>()
            && entity_ref.contains::<InspectorRowPopulated>()
        {
            populated.push(entity);
        }
    }

    let Some(tree_view) = tree_view else {
        return plan;
    };

    let index = world.resource::<TreeRowIndex>();

    let inspector_sources = &world.resource::<InspectorSources>().sources;

    let ordered_sources = determine_sources_order(inspector_sources);
    diff_header_container(world, tree_view, &ordered_sources, &mut plan);

    let inspectable_worlds: Vec<(&InspectorSource, &World)> = ordered_sources
        .iter()
        .map(|inspector_source| match inspector_source {
            InspectorSource::Local => (inspector_source, world),
            #[cfg(feature = "remote")]
            InspectorSource::Remote(_rem) => {
                let inspected = crate::world_to_inspect(world, inspector_source);
                (inspector_source, inspected)
            }
        })
        .collect();

    let sources = Sources::new(inspectable_worlds, priorities);

    for source in ordered_sources.iter() {
        let Some(inspected) = sources.get_world(*source) else {
            continue;
        };
        let Some(container) = index.source_to_row.get(source) else {
            continue;
        };

        let mut sub_roots: Vec<Entity> = inspected
            .iter_entities()
            .filter(|entity_ref| !entity_ref.contains::<ChildOf>())
            .map(|entity_ref| entity_ref.id())
            .filter(|entity| !is_excluded(inspected, *entity))
            .collect();
        sub_roots.sort_unstable_by_key(|root| root.index());

        let mut roots: Vec<InspectableEntity> = vec![];
        sub_roots
            .iter()
            .map(|e| InspectableEntity::new(*e, *source))
            .for_each(|f| roots.push(f));

        diff_container(world, &sources, *container, &roots, &mut plan);
    }

    for row in populated {
        let Some((source, kind)) = world
            .get::<InspectorEntityRow>(row)
            .map(|row| (row.inspected, row.source))
        else {
            continue;
        };
        let Some(inspected) = sources.get_world(kind) else {
            continue;
        };
        if inspected.get_entity(source).is_err() {
            continue;
        }
        let Some(container) = child_with::<FeathersTreeItemChildren>(world, row) else {
            continue;
        };
        let expected: Vec<InspectableEntity> = visible_children(inspected, source)
            .iter()
            .map(|e| InspectableEntity::new(*e, kind))
            .collect();
        diff_container(world, &sources, container, &expected, &mut plan);
    }

    plan
}

/// The inspected world, and the label priorities of the world the inspector runs in.
struct Sources<'w> {
    inspectable_worlds: Vec<(&'w InspectorSource, &'w World)>,
    priorities: Option<&'w LabelResolutionRegistry>,
}

impl<'w> Sources<'w> {
    fn new(
        inspectable_worlds: Vec<(&'w InspectorSource, &'w World)>,
        priorities: Option<&'w LabelResolutionRegistry>,
    ) -> Self {
        Self {
            inspectable_worlds,
            priorities,
        }
    }

    fn get_world(&'w self, want_kind: InspectorSource) -> Option<&'w World> {
        for (kind, world) in &self.inspectable_worlds {
            if **kind == want_kind {
                return Some(*world);
            }
        }
        None
    }
}

/// Diffs top container's header rows in `world` against the `expected` inspected entities, recording the
/// changes into `plan`. See [`SyncPlan`].
fn diff_header_container(
    world: &World,
    container: Entity,
    expected: &[InspectorSource],
    plan: &mut SyncPlan,
) {
    let mut existing: HashMap<InspectorSource, Entity> = HashMap::new();
    if let Some(children) = world.get::<Children>(container) {
        for child in children.iter().copied() {
            if let Some(row) = world.get::<InspectorHeaderRow>(child) {
                existing.insert(row.source, child);
            }
        }
    }

    let expected_set: HashSet<InspectorSource> = expected.iter().cloned().collect();
    for (source, row) in existing.iter() {
        if !expected_set.contains(source) {
            plan.header_despawn.push(*row);
        }
    }

    for source in expected.iter().cloned() {
        let label = header_label(source);
        let Some(_row) = existing.get(&source).copied() else {
            plan.header_spawn.push(HeaderSpawn {
                container,
                source,
                label,
            });
            continue;
        };
    }
}

/// Diffs one container's rows in `world` against the `expected` inspected entities, recording the
/// changes into `plan`. See [`SyncPlan`].
fn diff_container(
    world: &World,
    sources: &Sources,
    container: Entity,
    expected: &[InspectableEntity],
    plan: &mut SyncPlan,
) {
    let mut existing: HashMap<InspectableEntity, Entity> = HashMap::new();
    if let Some(children) = world.get::<Children>(container) {
        for child in children.iter().copied() {
            if let Some(row) = world.get::<InspectorEntityRow>(child) {
                existing.insert(InspectableEntity::new(row.inspected, row.source), child);
            }
        }
    }

    let expected_set: HashSet<InspectableEntity> = expected.iter().cloned().collect();
    for (source, row) in existing.iter() {
        if !expected_set.contains(source) {
            plan.despawn.push(*row);
        }
    }

    for InspectableEntity {
        entity: inspected,
        source,
    } in expected.iter().cloned()
    {
        let Some(inspected_world) = sources.get_world(source) else {
            continue;
        };
        let expandable = !visible_children(inspected_world, inspected).is_empty();
        let label = entity_label(inspected_world, sources.priorities, inspected);
        let Some(row) = existing
            .get(&InspectableEntity::new(inspected, source))
            .copied()
        else {
            plan.spawn.push(RowSpawn {
                container,
                inspected,
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
    for row in plan.header_despawn {
        if let Ok(row) = world.get_entity_mut(row) {
            row.despawn();
        }
    }

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

    for spawn in plan.header_spawn {
        if world.get_entity(spawn.container).is_err() {
            continue;
        }
        match world.spawn_scene(header_row(spawn.label)) {
            Ok(mut row) => {
                row.insert((
                    InspectorHeaderRow {
                        source: spawn.source,
                    },
                    ChildOf(spawn.container),
                ));
            }
            Err(error) => warn!("failed to spawn an inspector tree row: {error}"),
        }
    }

    for spawn in plan.spawn {
        if world.get_entity(spawn.container).is_err() {
            continue;
        }
        match world.spawn_scene(entity_row(spawn.label, spawn.expandable)) {
            Ok(mut row) => {
                row.insert((
                    InspectorEntityRow {
                        inspected: spawn.inspected,
                        source: spawn.source,
                    },
                    ChildOf(spawn.container),
                ));
            }
            Err(error) => warn!("failed to spawn an inspector tree row: {error}"),
        }
    }
}

/// Despawns every tree row, for when the inspected world is replaced and its entity ids no longer
/// mean the same entities.
#[cfg(feature = "remote")]
pub(crate) fn clear_rows(world: &mut World) {
    use bevy_ecs::query::Or;

    let rows: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<InspectorHeaderRow>, With<InspectorEntityRow>)>>()
        .iter(world)
        .collect();
    for row in rows {
        if let Ok(row) = world.get_entity_mut(row) {
            row.despawn();
        }
    }
    if let Some(mut index) = world.get_resource_mut::<TreeRowIndex>() {
        *index = TreeRowIndex::default();
    }
}

fn rebuild_index(world: &mut World) {
    let mut source_to_row = HashMap::new();
    let mut row_to_source = HashMap::new();
    for (entity, row) in world.query::<(Entity, &InspectorHeaderRow)>().iter(world) {
        source_to_row.insert(row.source, entity);
        row_to_source.insert(entity, row.source);
    }

    let mut inspectable_to_row = HashMap::new();
    let mut row_to_inspectable = HashMap::new();
    for (entity, row) in world.query::<(Entity, &InspectorEntityRow)>().iter(world) {
        inspectable_to_row.insert(InspectableEntity::new(row.inspected, row.source), entity);
        row_to_inspectable.insert(entity, InspectableEntity::new(row.inspected, row.source));
    }

    let mut index = world.resource_mut::<TreeRowIndex>();
    index.source_to_row = source_to_row;
    index.row_to_source = row_to_source;
    index.inspectable_to_row = inspectable_to_row;
    index.row_to_inspectable = row_to_inspectable;
}

fn header_row(label: String) -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @expandable: true,
            @label: bsn_list! { @row_label(label) },
        }
    }
}

fn entity_row(label: String, expandable: bool) -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @expandable: {expandable},
            @label: bsn_list! { @row_label(label) },
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
    if entity_ref.contains::<IsResource>()
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

pub(crate) fn header_label(source: InspectorSource) -> String {
    match source {
        InspectorSource::Local => "Local".to_string(),
        #[cfg(feature = "remote")]
        InspectorSource::Remote(rem) => format!("{}:{} ({})", rem.host.0, rem.port, rem.subapp),
    }
}

/// The label of `entity` in the inspected `world`, resolved with the label `priorities` of the
/// world the inspector runs in.
///
/// For a remote entity, only the components the remote app reported count, see
/// [`crate::remote::RemoteComponents`].
pub(crate) fn entity_label(
    world: &World,
    priorities: Option<&LabelResolutionRegistry>,
    entity: Entity,
) -> String {
    let Ok(entity_ref) = world.get_entity(entity) else {
        return entity.to_string();
    };
    #[cfg(feature = "remote")]
    let ids = match entity_ref.get::<crate::remote::RemoteComponents>() {
        Some(record) => record.reported().collect(),
        None => entity_ref.archetype().components().to_vec(),
    };
    #[cfg(not(feature = "remote"))]
    let ids = entity_ref.archetype().components().to_vec();

    let component_data: Vec<(ComponentId, String, Option<LabelDefinitionPriority>)> = ids
        .into_iter()
        .map(|component_id| {
            let priority = world
                .components()
                .get_info(component_id)
                .and_then(ComponentInfo::type_id)
                .and_then(|type_id| priorities?.get_priority_by_type_id(type_id));
            (
                component_id,
                crate::component_short_name(world, component_id),
                priority,
            )
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
            bevy_time::TimePlugin,
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            InspectorPlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app
    }

    fn header_rows(world: &World, container: Entity) -> Vec<(Entity, InspectorSource)> {
        world
            .get::<Children>(container)
            .map(|children| {
                children
                    .iter()
                    .copied()
                    .filter_map(|child| {
                        world
                            .get::<InspectorHeaderRow>(child)
                            .map(|f| (child, f.source))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn local_header(headers: &Vec<(Entity, InspectorSource)>) -> Option<Entity> {
        for (e, is) in headers {
            if *is == InspectorSource::Local {
                return Some(*e);
            }
        }
        None
    }

    fn row_sources(world: &World, container: Entity) -> Vec<Entity> {
        world
            .get::<Children>(container)
            .map(|children| {
                children
                    .iter()
                    .copied()
                    .filter_map(|child| {
                        world
                            .get::<InspectorEntityRow>(child)
                            .map(|row| row.inspected)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn excludes_inspector_ui_descendants_but_shows_the_root() {
        let mut world = World::new();
        let panel = world.spawn(InspectorUi).id();
        let inner = world.spawn(ChildOf(panel)).id();
        let subject = world.spawn_empty().id();

        assert!(!is_excluded(&world, panel));
        assert!(is_excluded(&world, inner));
        assert!(!is_excluded(&world, subject));
    }

    #[test]
    fn nested_inspector_ui_entities_stay_hidden() {
        let mut world = World::new();
        let panel = world.spawn(InspectorUi).id();
        let nested = world.spawn((InspectorUi, ChildOf(panel))).id();
        let nested_inner = world.spawn(ChildOf(nested)).id();

        assert!(!is_excluded(&world, panel));
        assert!(is_excluded(&world, nested));
        assert!(is_excluded(&world, nested_inner));
    }

    #[cfg(feature = "remote")]
    #[test]
    fn shows_the_inspected_world_only() {
        use crate::{
            remote::{
                tests::{apply, row},
                RemoteSource,
            },
            WorldKind,
        };
        use serde_json::json;

        let mut app = test_app();
        app.register_type::<ChildOf>().register_type::<Name>();
        app.insert_resource(InspectorSources::remote_main("localhost", 1));
        app.update();

        let panel = app.world_mut().spawn(InspectorUi).id();
        let tree = app
            .world_mut()
            .spawn((InspectorTreeView, ChildOf(panel)))
            .id();
        let local = app.world_mut().spawn(Name::new("Local")).id();
        let parent = Entity::from_raw_u32(40).unwrap();
        let child = Entity::from_raw_u32(41).unwrap();
        apply(
            app.world_mut(),
            alloc::vec![
                row(parent, json!({ "bevy_ecs::name::Name": "Remote" })),
                row(child, json!({ "bevy_ecs::hierarchy::ChildOf": parent })),
            ],
        );
        app.update();

        let remote = InspectorSource::Remote(RemoteSource::new("localhost", 1, WorldKind::Main));
        let remote_header = header_rows(app.world(), tree)
            .into_iter()
            .find_map(|(header, source)| (source == remote).then_some(header))
            .unwrap();
        assert_eq!(row_sources(app.world(), remote_header), [parent]);
        let row = app
            .world()
            .resource::<TreeRowIndex>()
            .row(parent, remote)
            .unwrap();
        let label = row_label_entity(app.world(), row).unwrap();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "Remote");
        assert!(app.world().get::<TreeItem>(row).unwrap().expandable);

        app.insert_resource(InspectorSources::local());
        app.update();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        let headers = header_rows(app.world(), tree);
        let local_header = local_header(&headers);
        let sources = row_sources(app.world(), local_header.unwrap());
        assert!(sources.contains(&local));
        assert!(!sources.contains(&parent));
    }

    #[test]
    fn diffs_many_rows_against_the_expected_set() {
        let mut app = test_app();
        let panel = app.world_mut().spawn(InspectorUi).id();
        let tree = app
            .world_mut()
            .spawn((InspectorTreeView, ChildOf(panel)))
            .id();
        let roots: Vec<Entity> = (0..500)
            .map(|index| app.world_mut().spawn(Name::new(format!("{index}"))).id())
            .collect();
        app.update();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        let headers = header_rows(app.world(), tree);
        let local_header = local_header(&headers).unwrap();
        assert_eq!(row_sources(app.world(), local_header).len(), 501);

        for root in roots.iter().step_by(2) {
            app.world_mut().entity_mut(*root).despawn();
        }
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let sources = row_sources(app.world(), local_header);
        assert_eq!(sources.len(), 251);
        assert!(roots
            .iter()
            .skip(1)
            .step_by(2)
            .all(|root| sources.contains(root)));
    }

    #[test]
    fn syncs_root_rows_with_the_world() {
        let mut app = test_app();
        let panel = app.world_mut().spawn(InspectorUi).id();
        let tree = app
            .world_mut()
            .spawn((InspectorTreeView, ChildOf(panel)))
            .id();
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        app.world_mut().spawn((Name::new("Child"), ChildOf(parent)));
        let sibling = app.world_mut().spawn(Name::new("Sibling")).id();

        app.update();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let headers = header_rows(app.world(), tree);
        let local_header = local_header(&headers).unwrap();
        let sources = row_sources(app.world(), local_header);
        assert_eq!(sources.len(), 3);
        assert!(sources.contains(&panel));
        assert!(sources.contains(&parent));
        assert!(sources.contains(&sibling));

        let panel_row = app
            .world()
            .resource::<TreeRowIndex>()
            .row(panel, InspectorSource::Local)
            .unwrap();
        assert!(!app.world().get::<TreeItem>(panel_row).unwrap().expandable);
        let parent_row = app
            .world()
            .resource::<TreeRowIndex>()
            .row(parent, InspectorSource::Local)
            .unwrap();
        assert!(app.world().get::<TreeItem>(parent_row).unwrap().expandable);
        let sibling_row = app
            .world()
            .resource::<TreeRowIndex>()
            .row(sibling, InspectorSource::Local)
            .unwrap();
        assert!(!app.world().get::<TreeItem>(sibling_row).unwrap().expandable);

        app.world_mut().entity_mut(sibling).despawn();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let sources = row_sources(app.world(), local_header);
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&panel));
        assert!(sources.contains(&parent));
        assert!(app
            .world()
            .resource::<TreeRowIndex>()
            .row(sibling, InspectorSource::Local)
            .is_none());
    }
}
