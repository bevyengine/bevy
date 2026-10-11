//! A panel listing the entities of the inspected world, and the systems that keep it in sync.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use bevy_dev_tools::inspection::label_resolution::{
    resolve_label, ComponentLabelData, LabelDefinitionPriority, LabelResolutionRegistry,
};
use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
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
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{percent, px, widget::Text, Node, Overflow};
use bevy_ui_widgets::{
    tree_view_expand_self_update, tree_view_self_update, TreeItem, TreeItemExpandChange,
    ValueChange,
};

use crate::{_InspectorSelection, InspectorSelection};

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

/// A row of the entity tree, and the entity it displays.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorRow {
    /// The inspected entity this row displays.
    pub source: Entity,
    pub is_main: bool,
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
    source_to_row: HashMap<_InspectorSelection, Entity>,
    row_to_source: HashMap<Entity, _InspectorSelection>,
}

impl TreeRowIndex {
    /// The row displaying `source`, if one exists.
    pub fn row(&self, source: Entity, is_main: bool) -> Option<Entity> {
        self.source_to_row
            .get(&_InspectorSelection {
                entity: source,
                is_main,
            })
            .copied()
    }

    /// The inspected entity displayed by `row`, if one exists.
    pub fn source(&self, row: Entity) -> Option<_InspectorSelection> {
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
    trees: Query<&InspectorTreeView>,
    rows: Query<&InspectorRow>,
    mut selection: ResMut<InspectorSelection>,
) {
    let Ok(tree) = trees.get(change.source) else {
        return;
    };

    selection.0 = change
        .value
        .and_then(|row| rows.get(row).ok())
        .map(|row| _InspectorSelection {
            entity: row.source,
            is_main: row.is_main,
        });
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
    sync.set_dirty();
}

/// Adds, removes and relabels tree rows so that they match the inspected world.
/// TODO: need to handle is_main - this is the system that gets scheduled
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
    apply_sync(world, plan);
    rebuild_index(world);
}

struct RowSpawn {
    container: Entity,
    source: Entity,
    is_main: bool,
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
/// Each row is a [`InspectorRow`]
#[derive(Default)]
struct SyncPlan {
    despawn: Vec<Entity>,
    spawn: Vec<RowSpawn>,
    relabel: Vec<(Entity, String)>,
    expandable: Vec<(Entity, bool)>,
}

/// Builds the [`SyncPlan`] for one synchronization pass.
///
/// The rows live in `world`, while the entities they display are read from the inspected world,
/// see [`crate::world_to_inspect`].
fn plan_sync(world: &World) -> SyncPlan {
    let mut plan = SyncPlan::default();
    let mut tree_view = None;
    let mut populated = Vec::new();

    let inspected_main = crate::world_to_inspect(world, true);
    // TODO: only do if remote
    let inspected_render = crate::world_to_inspect(world, false);

    let priorities = world.get_resource::<LabelResolutionRegistry>();

    for entity_ref in world.iter_entities() {
        let entity = entity_ref.id();
        if entity_ref.contains::<InspectorTreeView>() {
            tree_view = Some(entity);
        }
        if entity_ref.contains::<InspectorRow>() && entity_ref.contains::<InspectorRowPopulated>() {
            populated.push(entity);
        }
    }

    let Some(tree_view) = tree_view else {
        return plan;
    };

    let mut main_roots: Vec<Entity> = inspected_main
        .iter_entities()
        .filter(|entity_ref| !entity_ref.contains::<ChildOf>())
        .map(|entity_ref| entity_ref.id())
        .filter(|entity| !is_excluded(inspected_main, *entity))
        .collect();
    main_roots.sort_unstable_by_key(|root| root.index());

    let mut render_roots: Vec<Entity> = inspected_render
        .iter_entities()
        .filter(|entity_ref| !entity_ref.contains::<ChildOf>())
        .map(|entity_ref| entity_ref.id())
        .filter(|entity| !is_excluded(inspected_render, *entity))
        .collect();
    render_roots.sort_unstable_by_key(|root| root.index());

    let first = main_roots.iter().map(|e| _InspectorSelection {
        entity: *e,
        is_main: true,
    });
    let second = render_roots.iter().map(|e| _InspectorSelection {
        entity: *e,
        is_main: false,
    });
    let roots: Vec<_InspectorSelection> = first.chain(second).collect();
    // let roots: Vec<_InspectorSelection> = second.collect();

    let sources = Sources {
        main_world: inspected_main,
        render_world: inspected_render,
        priorities,
    };
    diff_container(world, &sources, tree_view, &roots, &mut plan);

    for row in populated {
        let Some((source, is_main)) = world
            .get::<InspectorRow>(row)
            .map(|row| (row.source, row.is_main))
        else {
            continue;
        };
        let inspected = if is_main {
            inspected_main
        } else {
            inspected_render
        };
        if inspected.get_entity(source).is_err() {
            continue;
        }
        let Some(container) = child_with::<FeathersTreeItemChildren>(world, row) else {
            continue;
        };
        let expected: Vec<_InspectorSelection> = visible_children(inspected, source)
            .iter()
            .map(|e| _InspectorSelection {
                entity: *e,
                is_main,
            })
            .collect();
        diff_container(world, &sources, container, &expected, &mut plan);
    }

    plan
}

/// The inspected world, and the label priorities of the world the inspector runs in.
struct Sources<'w> {
    main_world: &'w World,
    render_world: &'w World,
    priorities: Option<&'w LabelResolutionRegistry>,
}

/// Diffs one container's rows in `world` against the `expected` inspected entities, recording the
/// changes into `plan`. See [`SyncPlan`].
fn diff_container(
    world: &World,
    sources: &Sources,
    container: Entity,
    expected: &[_InspectorSelection],
    plan: &mut SyncPlan,
) {
    let mut existing: HashMap<_InspectorSelection, Entity> = HashMap::new();
    if let Some(children) = world.get::<Children>(container) {
        for child in children.iter().copied() {
            if let Some(row) = world.get::<InspectorRow>(child) {
                existing.insert(
                    _InspectorSelection {
                        entity: row.source,
                        is_main: row.is_main,
                    },
                    child,
                );
            }
        }
    }

    let expected_set: HashSet<_InspectorSelection> = expected.iter().copied().collect();
    for (source, row) in existing.iter() {
        if !expected_set.contains(source) {
            plan.despawn.push(*row);
        }
    }

    for _InspectorSelection {
        entity: source,
        is_main,
    } in expected.iter().copied()
    {
        let world = if is_main {
            sources.main_world
        } else {
            sources.render_world
        };
        let expandable = !visible_children(world, source).is_empty();
        let label = entity_label(world, sources.priorities, source);
        let Some(row) = existing
            .get(&_InspectorSelection {
                entity: source,
                is_main,
            })
            .copied()
        else {
            plan.spawn.push(RowSpawn {
                container,
                source,
                is_main,
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
    // TODO: need is_main
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
                        is_main: spawn.is_main,
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
    let rows: Vec<Entity> = world
        .query_filtered::<Entity, With<InspectorRow>>()
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
    for (entity, row) in world.query::<(Entity, &InspectorRow)>().iter(world) {
        source_to_row.insert(
            _InspectorSelection {
                entity: row.source,
                is_main: row.is_main,
            },
            entity,
        );
        row_to_source.insert(
            entity,
            _InspectorSelection {
                entity: row.source,
                is_main: row.is_main,
            },
        );
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
        println!("exc none");
        return true;
    };
    if entity_ref.contains::<IsResource>()
    {
        // println!("exc IsResource");
        return true;
    }
    if entity_ref.contains::<SystemIdMarker>() {
        println!("exc SystemIdMarker");
        return true;
    }
    if entity_ref.contains::<Observer>() {
        println!("exc Observer");
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
        use crate::remote::{
            tests::{apply, row},
            RemoteSource,
        };
        use crate::InspectorSource;
        use serde_json::json;

        let mut app = test_app();
        app.register_type::<ChildOf>().register_type::<Name>();
        app.insert_resource(InspectorSource::Remote(RemoteSource::localhost(1, 2)));
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

        assert_eq!(row_sources(app.world(), tree), [parent]);
        let row = app.world().resource::<TreeRowIndex>().row(parent).unwrap();
        let label = row_label_entity(app.world(), row).unwrap();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "Remote");
        assert!(app.world().get::<TreeItem>(row).unwrap().expandable);

        app.insert_resource(InspectorSource::Local);
        app.update();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        let sources = row_sources(app.world(), tree);
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
        assert_eq!(row_sources(app.world(), tree).len(), 501);

        for root in roots.iter().step_by(2) {
            app.world_mut().entity_mut(*root).despawn();
        }
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let sources = row_sources(app.world(), tree);
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

        let sources = row_sources(app.world(), tree);
        assert_eq!(sources.len(), 3);
        assert!(sources.contains(&panel));
        assert!(sources.contains(&parent));
        assert!(sources.contains(&sibling));

        let panel_row = app.world().resource::<TreeRowIndex>().row(panel).unwrap();
        assert!(!app.world().get::<TreeItem>(panel_row).unwrap().expandable);
        let parent_row = app.world().resource::<TreeRowIndex>().row(parent).unwrap();
        assert!(app.world().get::<TreeItem>(parent_row).unwrap().expandable);
        let sibling_row = app.world().resource::<TreeRowIndex>().row(sibling).unwrap();
        assert!(!app.world().get::<TreeItem>(sibling_row).unwrap().expandable);

        app.world_mut().entity_mut(sibling).despawn();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let sources = row_sources(app.world(), tree);
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&panel));
        assert!(sources.contains(&parent));
        assert!(app
            .world()
            .resource::<TreeRowIndex>()
            .row(sibling)
            .is_none());
    }
}
