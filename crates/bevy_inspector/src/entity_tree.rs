//! A panel listing the entities of the inspected world, and the systems that keep it in sync.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::cmp::Ordering;

use bevy_app::Propagate;
use bevy_dev_tools::inspection::label_resolution::{
    resolve_label, ComponentLabelData, LabelDefinitionPriority, LabelResolutionRegistry,
};
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    name::Name,
    observer::{Observer, On},
    query::{Or, With},
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::{IsResource, Resource},
    system::{Commands, Query, Res, ResMut, SystemIdMarker},
    template::template,
    world::World,
};
use bevy_feathers::{
    constants::size,
    containers::{subpane, subpane_body, subpane_header},
    controls::{
        FeathersTextInput, FeathersTextInputContainer, FeathersTreeItem, FeathersTreeItemChildren,
        FeathersTreeItemHeader, FeathersTreeView,
    },
    display::caption,
    theme::{SurfaceLevel, ThemeContext, ThemeTextColor, ThemedText},
    tokens,
};
use bevy_fuzzy::{segments, FuzzyPattern};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_text::{EditableText, Placeholder, TextEdit, TextEditChange, TextFont, TextSpan};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{percent, px, widget::Text, Display, Expanded, Node, Overflow, UiRect};
use bevy_ui_widgets::{
    tree_view_expand_self_update, tree_view_self_update, SelectedTreeItem, TreeItem,
    TreeItemExpandChange, ValueChange,
};
use bevy_utils::DebugName;

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

/// Marker for the text input holding the entity tree filter query.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct EntityTreeFilterInput;

/// Marker for the caption shown when more entities match the filter than the tree lists.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct EntityTreeFilterNote;

/// The maximum number of matching entities the filtered entity tree lists.
pub const MAX_FILTER_RESULTS: usize = 500;

/// The query filtering the entity tree.
///
/// While it is blank, the tree shows the entity hierarchy. Otherwise it shows the entities whose
/// label, or the short name of one of whose components, fuzzy matches the query (see
/// [`FuzzyPattern`]), together with their ancestors, which are expanded. The matched characters
/// of a label are highlighted. Clearing the query restores the rows expanded before filtering.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub struct EntityTreeFilter(pub String);

/// The text entity holding a row's label, and the label it shows.
///
/// A label without highlighted characters is the entity's [`Text`]. Otherwise, the text is split
/// into [`TextSpan`] children, and the highlighted ones use the [`tokens::TEXT_ACCENT`] color.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, Clone, PartialEq)]
pub struct InspectorRowLabel {
    /// The label text.
    pub text: String,
    /// The sorted char indices of `text` highlighted as filter matches.
    pub matched: Vec<usize>,
}

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
    applied_filter: String,
    expanded_before_filter: Option<HashSet<InspectableEntity>>,
}

impl Default for EntityTreeSync {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.25, TimerMode::Repeating);
        timer.set_elapsed(timer.duration());
        Self {
            timer,
            applied_filter: String::new(),
            expanded_before_filter: None,
        }
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

/// The contents of [`entity_tree_panel`] without its header: the filter box and the scrolling
/// tree.
pub fn entity_tree_body() -> impl Scene {
    bsn! {
        @subpane_body()
        Node {
            overflow: Overflow::clip(),
        }
        Children [
            @FeathersTextInputContainer
            Node {
                flex_grow: 0.0,
                flex_shrink: 0.0,
                padding: UiRect {
                    left: px(8.0),
                },
            }
            Propagate::<ThemeContext>(ThemeContext(SurfaceLevel::Base))
            Children [
                EntityTreeFilterInput
                @FeathersTextInput
                TextFont {
                    font_size: size::SMALL_FONT,
                }
                template(|_| Ok(Placeholder::new("Filter...")))
                on(inspector_tree_filter_changed)
            ]
            --
            EntityTreeFilterNote
            @caption("")
            Node {
                display: Display::None,
            }
            --
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

/// Observer that copies the filter input's text into [`EntityTreeFilter`].
pub fn inspector_tree_filter_changed(
    change: On<TextEditChange>,
    inputs: Query<&EditableText, With<EntityTreeFilterInput>>,
    mut filter: ResMut<EntityTreeFilter>,
) {
    let Ok(input) = inputs.get(change.event_target()) else {
        return;
    };
    let query = input.value();
    if filter.0 != query {
        filter.0 = query.to_string();
    }
}

/// Shows the [`EntityTreeFilter`] query in the filter inputs when it is set by other means than
/// typing.
pub(crate) fn sync_entity_tree_filter_input(
    filter: Res<EntityTreeFilter>,
    mut inputs: Query<(Ref<EntityTreeFilterInput>, &mut EditableText)>,
) {
    for (marker, mut input) in &mut inputs {
        if !(filter.is_changed() || marker.is_added()) || input.value() == filter.0 {
            continue;
        }
        if filter.0.is_empty() {
            input.clear();
        } else {
            input.queue_edit(TextEdit::SelectAll);
            input.queue_edit(TextEdit::Insert(filter.0.as_str().into()));
        }
    }
}

/// Adds, removes, relabels, expands and collapses tree rows so that they match the inspected
/// worlds and the [`EntityTreeFilter`].
///
/// A change to the filter forces a pass. Entering the filtered tree remembers the expanded rows,
/// and clearing the filter expands those rows and the ancestors of the [`InspectorSelection`]
/// again, and collapses the others. Rows expanded by a pass are populated in the same pass, so
/// nested matches and restored rows show at once.
pub fn sync_entity_tree(world: &mut World) {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();
    let query = world
        .get_resource::<EntityTreeFilter>()
        .map(|filter| filter.0.trim().to_string())
        .unwrap_or_default();

    let mut sync = world.resource_mut::<EntityTreeSync>();
    let due = sync.timer.tick(delta).just_finished();
    let query_changed = sync.applied_filter != query;
    if !due && !query_changed {
        return;
    }
    let entering = sync.applied_filter.is_empty() && !query.is_empty();
    let leaving = !sync.applied_filter.is_empty() && query.is_empty();
    sync.applied_filter.clone_from(&query);
    let mut restore = leaving.then(|| sync.expanded_before_filter.take().unwrap_or_default());
    if let Some(restore) = &mut restore {
        restore.extend(selection_ancestors(world));
    }

    if entering {
        let expanded = expanded_rows(world);
        world
            .resource_mut::<EntityTreeSync>()
            .expanded_before_filter = Some(expanded);
    }

    let pattern = FuzzyPattern::new(&query);
    let filter = (!pattern.is_empty()).then(|| FilteredTree::new(world, &pattern));
    loop {
        let plan = plan_sync(world, filter.as_ref(), query_changed, restore.as_ref());
        let descended = apply_sync(world, plan);
        rebuild_index(world);
        if !descended {
            break;
        }
    }
    if query_changed {
        restore_selection(world);
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
    label: InspectorRowLabel,
    expandable: bool,
    expanded: bool,
}

/// The set of changes a single synchronization pass needs to apply.
///
/// [`plan_sync`] reads the world without mutating it, diffing each header row against the current
/// root entities of its source and each populated row's children container against that row's
/// entity's current children (see [`diff_container`]). Rows that were never expanded are not
/// diffed. The result records which rows to despawn, which to spawn, which labels to update,
/// which `expandable` flags to flip, which rows to expand or collapse and which containers to
/// reorder. [`apply_sync`] then applies those changes. Expanding a row populates it, so its
/// children are diffed by the next pass, which runs while the previous one expanded a row or
/// added a header row.
///
/// With a filter, only the entities kept by [`FilteredTree`] are diffed, and `filter_note`
/// reports when the matches were capped at [`MAX_FILTER_RESULTS`].
///
/// Each row is a [`InspectorHeaderRow`] or [`InspectorEntityRow`]
#[derive(Default)]
struct SyncPlan {
    header_despawn: Vec<Entity>,
    header_spawn: Vec<HeaderSpawn>,
    despawn: Vec<Entity>,
    spawn: Vec<RowSpawn>,
    relabel: Vec<(Entity, InspectorRowLabel)>,
    expandable: Vec<(Entity, bool)>,
    expand: Vec<Entity>,
    collapse: Vec<Entity>,
    reorder: Vec<(Entity, Vec<InspectableEntity>)>,
    filter_note: Option<String>,
}

/// An entity row of the entity tree as [`plan_sync`] sees it.
struct RowState {
    row: Entity,
    inspected: InspectableEntity,
    populated: bool,
    expanded: bool,
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

/// Builds the [`SyncPlan`] for one synchronization pass, showing only what `filter` keeps if
/// there is one.
///
/// With `query_changed`, the ancestors of matches are expanded even if the user collapsed them.
/// `restore` holds the entities whose rows to expand when the filter has just been cleared; the
/// other rows are collapsed.
///
/// The rows live in `world`, while the entities they display are read from the inspected world,
/// see [`crate::world_to_inspect`].
fn plan_sync(
    world: &World,
    filter: Option<&FilteredTree>,
    query_changed: bool,
    restore: Option<&HashSet<InspectableEntity>>,
) -> SyncPlan {
    let mut plan = SyncPlan::default();
    let mut tree_view = None;
    let mut rows = Vec::new();

    for entity_ref in world.iter_entities() {
        let entity = entity_ref.id();
        if entity_ref.contains::<InspectorTreeView>() {
            tree_view = Some(entity);
        }
        if let Some(row) = entity_ref.get::<InspectorEntityRow>() {
            rows.push(RowState {
                row: entity,
                inspected: InspectableEntity::new(row.inspected, row.source),
                populated: entity_ref.contains::<InspectorRowPopulated>(),
                expanded: entity_ref.contains::<Expanded>(),
            });
        }
    }

    let Some(tree_view) = tree_view else {
        return plan;
    };

    let index = world.resource::<TreeRowIndex>();

    let sources = Sources::new(world);
    diff_header_container(world, tree_view, &sources.ordered(), &mut plan);

    if let Some(filter) = filter
        && filter.total > MAX_FILTER_RESULTS
    {
        plan.filter_note = Some(format!(
            "Showing {MAX_FILTER_RESULTS} of {} matches",
            filter.total
        ));
    }
    let shown = ShownTree {
        sources: &sources,
        filter,
        restore,
    };

    for (source, mut roots) in sources.roots() {
        let Some(container) = index.source_to_row.get(&source) else {
            continue;
        };
        roots.retain(|root| shown.keeps(root));
        diff_container(world, &shown, *container, &roots, &mut plan);
    }

    for row in rows {
        let Some(inspected) = sources.get_world(row.inspected.source) else {
            continue;
        };
        if inspected.get_entity(row.inspected.entity).is_err() {
            continue;
        }
        let expand = match (&filter, restore) {
            (Some(filter), _) => {
                filter.parents.contains(&row.inspected)
                    && (!row.populated || (query_changed && !row.expanded))
            }
            (None, Some(restore)) => {
                restore.contains(&row.inspected) && !(row.populated && row.expanded)
            }
            (None, None) => false,
        };
        let collapse = filter.is_none()
            && row.expanded
            && restore.is_some_and(|restore| !restore.contains(&row.inspected));
        if expand {
            plan.expand.push(row.row);
        } else if collapse {
            plan.collapse.push(row.row);
        }

        if !row.populated || !shown.keeps(&row.inspected) {
            continue;
        }
        let Some(container) = child_with::<FeathersTreeItemChildren>(world, row.row) else {
            continue;
        };
        let expected = shown.children(&row.inspected);
        diff_container(world, &shown, container, &expected, &mut plan);
    }

    plan
}

/// The inspected worlds in tree order, and the label priorities of the world the inspector runs
/// in.
struct Sources<'w> {
    inspectable_worlds: Vec<(InspectorSource, &'w World)>,
    priorities: Option<&'w LabelResolutionRegistry>,
}

impl<'w> Sources<'w> {
    fn new(world: &'w World) -> Self {
        let ordered = determine_sources_order(&world.resource::<InspectorSources>().sources);
        Self {
            inspectable_worlds: ordered
                .into_iter()
                .map(|source| (source, crate::world_to_inspect(world, &source)))
                .collect(),
            priorities: world.get_resource::<LabelResolutionRegistry>(),
        }
    }

    fn ordered(&self) -> Vec<InspectorSource> {
        self.inspectable_worlds
            .iter()
            .map(|(source, _)| *source)
            .collect()
    }

    fn get_world(&self, want_kind: InspectorSource) -> Option<&'w World> {
        self.inspectable_worlds
            .iter()
            .find(|(kind, _)| *kind == want_kind)
            .map(|(_, world)| *world)
    }

    /// The shown root entities of each source, sorted by index.
    fn roots(&self) -> Vec<(InspectorSource, Vec<InspectableEntity>)> {
        self.inspectable_worlds
            .iter()
            .map(|(source, inspected)| {
                let mut roots: Vec<Entity> = inspected
                    .iter_entities()
                    .filter(|entity_ref| !entity_ref.contains::<ChildOf>())
                    .map(|entity_ref| entity_ref.id())
                    .filter(|entity| !is_excluded(inspected, *entity))
                    .collect();
                roots.sort_unstable_by_key(|root| root.index());
                let roots = roots
                    .into_iter()
                    .map(|root| InspectableEntity::new(root, *source))
                    .collect();
                (*source, roots)
            })
            .collect()
    }
}

/// The entities a filtered entity tree shows.
struct FilteredTree {
    /// The matches, up to [`MAX_FILTER_RESULTS`] in tree order, and all their ancestors.
    kept: HashSet<InspectableEntity>,
    /// The kept entities with a kept child.
    parents: HashSet<InspectableEntity>,
    /// The matched label chars of the kept matches whose label matched.
    highlights: HashMap<InspectableEntity, Vec<usize>>,
    /// The number of matches before the cap.
    total: usize,
}

impl FilteredTree {
    /// Matches every shown entity of the inspected worlds against `pattern`, walking the tree
    /// depth first.
    fn new(world: &World, pattern: &FuzzyPattern) -> Self {
        let sources = Sources::new(world);
        let mut filter = Self {
            kept: HashSet::new(),
            parents: HashSet::new(),
            highlights: HashMap::new(),
            total: 0,
        };
        let mut matches = Vec::new();
        let mut component_matches = HashMap::new();
        let mut stack: Vec<InspectableEntity> = sources
            .roots()
            .into_iter()
            .flat_map(|(_, roots)| roots)
            .rev()
            .collect();
        while let Some(entity) = stack.pop() {
            let Some(world) = sources.get_world(entity.source) else {
                continue;
            };
            if let Some(highlight) = entity_match(
                world,
                sources.priorities,
                pattern,
                &entity,
                &mut component_matches,
            ) {
                filter.total += 1;
                if matches.len() < MAX_FILTER_RESULTS {
                    if !highlight.is_empty() {
                        filter.highlights.insert(entity.clone(), highlight);
                    }
                    matches.push(entity.clone());
                }
            }
            stack.extend(
                visible_children(world, entity.entity)
                    .into_iter()
                    .rev()
                    .map(|child| InspectableEntity::new(child, entity.source)),
            );
        }

        for entity in matches {
            let Some(world) = sources.get_world(entity.source) else {
                continue;
            };
            let mut current = entity.entity;
            filter.kept.insert(entity.clone());
            while let Some(child_of) = world.get::<ChildOf>(current) {
                current = child_of.parent();
                let parent = InspectableEntity::new(current, entity.source);
                filter.parents.insert(parent.clone());
                if !filter.kept.insert(parent) {
                    break;
                }
            }
        }
        filter
    }
}

/// The entity tree to show: the whole hierarchy, or the entities a filter keeps.
struct ShownTree<'a> {
    sources: &'a Sources<'a>,
    filter: Option<&'a FilteredTree>,
    restore: Option<&'a HashSet<InspectableEntity>>,
}

impl ShownTree<'_> {
    fn keeps(&self, entity: &InspectableEntity) -> bool {
        self.filter
            .is_none_or(|filter| filter.kept.contains(entity))
    }

    fn children(&self, entity: &InspectableEntity) -> Vec<InspectableEntity> {
        let Some(world) = self.sources.get_world(entity.source) else {
            return Vec::new();
        };
        visible_children(world, entity.entity)
            .into_iter()
            .map(|child| InspectableEntity::new(child, entity.source))
            .filter(|child| self.keeps(child))
            .collect()
    }

    fn label(&self, world: &World, entity: &InspectableEntity) -> InspectorRowLabel {
        InspectorRowLabel {
            text: entity_label(world, self.sources.priorities, entity.entity),
            matched: self
                .filter
                .and_then(|filter| filter.highlights.get(entity))
                .cloned()
                .unwrap_or_default(),
        }
    }

    /// Whether a new row for `entity` starts expanded.
    fn expands(&self, entity: &InspectableEntity) -> bool {
        match self.filter {
            Some(filter) => filter.parents.contains(entity),
            None => self.restore.is_some_and(|restore| restore.contains(entity)),
        }
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
    shown: &ShownTree,
    container: Entity,
    expected: &[InspectableEntity],
    plan: &mut SyncPlan,
) {
    let mut existing: HashMap<InspectableEntity, Entity> = HashMap::new();
    let mut order = Vec::new();
    if let Some(children) = world.get::<Children>(container) {
        for child in children.iter().copied() {
            if let Some(row) = world.get::<InspectorEntityRow>(child) {
                let inspected = InspectableEntity::new(row.inspected, row.source);
                existing.insert(inspected.clone(), child);
                order.push(inspected);
            }
        }
    }

    let expected_set: HashSet<InspectableEntity> = expected.iter().cloned().collect();
    for (source, row) in existing.iter() {
        if !expected_set.contains(source) {
            plan.despawn.push(*row);
        }
    }
    order.retain(|inspected| expected_set.contains(inspected));
    order.extend(
        expected
            .iter()
            .filter(|inspected| !existing.contains_key(*inspected))
            .cloned(),
    );
    if order != expected {
        plan.reorder.push((container, expected.to_vec()));
    }

    for inspected in expected.iter() {
        let Some(inspected_world) = shown.sources.get_world(inspected.source) else {
            continue;
        };
        let expandable = !shown.children(inspected).is_empty();
        let label = shown.label(inspected_world, inspected);
        let Some(row) = existing.get(inspected).copied() else {
            plan.spawn.push(RowSpawn {
                container,
                inspected: inspected.entity,
                source: inspected.source,
                label,
                expandable,
                expanded: expandable && shown.expands(inspected),
            });
            continue;
        };

        if let Some(label_entity) = row_label_entity(world, row)
            && world.get::<InspectorRowLabel>(label_entity) != Some(&label)
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

/// Applies `plan`, returning whether it expanded a row or added a header row.
fn apply_sync(world: &mut World, plan: SyncPlan) -> bool {
    let mut descended = false;
    let notes: Vec<Entity> = world
        .query_filtered::<Entity, With<EntityTreeFilterNote>>()
        .iter(world)
        .collect();
    for note in notes {
        let display = match plan.filter_note {
            Some(_) => Display::Flex,
            None => Display::None,
        };
        if let Some(mut node) = world.get_mut::<Node>(note)
            && node.display != display
        {
            node.display = display;
        }
        if let Some(text) = &plan.filter_note
            && let Some(mut label) = world.get_mut::<Text>(note)
            && label.0 != *text
        {
            label.0.clone_from(text);
        }
    }

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
        write_label(world, label_entity, label);
    }

    for (row, expandable) in plan.expandable {
        if let Some(mut item) = world.get_mut::<TreeItem>(row) {
            item.expandable = expandable;
        }
    }

    for row in plan.collapse {
        if let Ok(mut row) = world.get_entity_mut(row) {
            row.remove::<Expanded>();
        }
    }

    for row in plan.expand {
        if let Ok(mut row) = world.get_entity_mut(row) {
            row.insert((InspectorRowPopulated, Expanded));
            descended = true;
        }
    }

    for spawn in plan.header_spawn {
        if world.get_entity(spawn.container).is_err() {
            continue;
        }
        let row = match world.spawn_scene(header_row()) {
            Ok(mut row) => {
                row.insert((
                    InspectorHeaderRow {
                        source: spawn.source,
                    },
                    ChildOf(spawn.container),
                ));
                descended = true;
                row.id()
            }
            Err(error) => {
                warn!("failed to spawn an inspector tree row: {error}");
                continue;
            }
        };
        if let Some(label_entity) = row_label_entity(world, row) {
            let label = InspectorRowLabel {
                text: spawn.label,
                matched: Vec::new(),
            };
            write_label(world, label_entity, label);
        }
    }

    for spawn in plan.spawn {
        if world.get_entity(spawn.container).is_err() {
            continue;
        }
        let row = match world.spawn_scene(entity_row(spawn.expandable)) {
            Ok(mut row) => {
                row.insert((
                    InspectorEntityRow {
                        inspected: spawn.inspected,
                        source: spawn.source,
                    },
                    ChildOf(spawn.container),
                ));
                if spawn.expanded {
                    row.insert((InspectorRowPopulated, Expanded));
                    descended = true;
                }
                row.id()
            }
            Err(error) => {
                warn!("failed to spawn an inspector tree row: {error}");
                continue;
            }
        };
        if let Some(label_entity) = row_label_entity(world, row) {
            write_label(world, label_entity, spawn.label);
        }
    }

    for (container, expected) in plan.reorder {
        reorder_rows(world, container, expected);
    }
    descended
}

/// Sorts the entity rows among the children of `container` into the order of `expected`, keeping
/// the other children in place.
fn reorder_rows(world: &mut World, container: Entity, expected: Vec<InspectableEntity>) {
    let rank: HashMap<InspectableEntity, usize> = expected
        .into_iter()
        .enumerate()
        .map(|(rank, inspected)| (inspected, rank))
        .collect();
    let Some(children) = world.get::<Children>(container) else {
        return;
    };
    let mut slots = Vec::new();
    let mut rows = Vec::new();
    for (slot, child) in children.iter().copied().enumerate() {
        if let Some(row) = world.get::<InspectorEntityRow>(child) {
            let key = rank
                .get(&InspectableEntity::new(row.inspected, row.source))
                .copied()
                .unwrap_or(usize::MAX);
            slots.push(slot);
            rows.push((key, child));
        }
    }
    rows.sort_by_key(|(key, _)| *key);
    let mut position: HashMap<Entity, usize> = children
        .iter()
        .copied()
        .enumerate()
        .map(|(slot, child)| (child, slot))
        .collect();
    for (slot, (_, row)) in slots.into_iter().zip(rows) {
        position.insert(row, slot);
    }
    if let Some(mut children) = world.get_mut::<Children>(container) {
        children.sort_by_key(|child| position.get(child).copied().unwrap_or(usize::MAX));
    }
}

/// Shows `label` on the `label_entity` of a row, rebuilding its highlighted spans.
fn write_label(world: &mut World, label_entity: Entity, label: InspectorRowLabel) {
    let Ok(mut entity) = world.get_entity_mut(label_entity) else {
        return;
    };
    entity.despawn_children();
    let text = match label.matched.is_empty() {
        true => label.text.clone(),
        false => {
            entity.with_children(|spans| {
                for (segment, matched) in segments(&label.text, &label.matched) {
                    match matched {
                        true => spans
                            .spawn((TextSpan::new(segment), ThemeTextColor(tokens::TEXT_ACCENT))),
                        false => spans.spawn((TextSpan::new(segment), ThemedText)),
                    };
                }
            });
            String::new()
        }
    };
    entity.insert((Text(text), label));
}

/// The ancestors of the selected entity in its inspected world, if it still exists and is shown.
fn selection_ancestors(world: &World) -> Vec<InspectableEntity> {
    let Some(selected) = world
        .get_resource::<InspectorSelection>()
        .and_then(|selection| selection.0.clone())
    else {
        return Vec::new();
    };
    let inspected = crate::world_to_inspect(world, &selected.source);
    if is_excluded(inspected, selected.entity) {
        return Vec::new();
    }
    let mut ancestors = Vec::new();
    let mut current = selected.entity;
    while let Some(child_of) = inspected.get::<ChildOf>(current) {
        current = child_of.parent();
        ancestors.push(InspectableEntity::new(current, selected.source));
    }
    ancestors
}

/// The inspected entities whose rows are expanded and populated.
fn expanded_rows(world: &mut World) -> HashSet<InspectableEntity> {
    world
        .query_filtered::<&InspectorEntityRow, (With<Expanded>, With<InspectorRowPopulated>)>()
        .iter(world)
        .map(|row| InspectableEntity::new(row.inspected, row.source))
        .collect()
}

/// Despawns every tree row, for when the inspected world is replaced and its entity ids no longer
/// mean the same entities.
#[cfg(feature = "remote")]
pub(crate) fn clear_rows(world: &mut World) {
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
    if let Some(mut sync) = world.get_resource_mut::<EntityTreeSync>() {
        sync.expanded_before_filter = None;
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

fn restore_selection(world: &mut World) {
    let row = world
        .get_resource::<InspectorSelection>()
        .and_then(|selection| selection.0.clone())
        .and_then(|selected| {
            world
                .resource::<TreeRowIndex>()
                .row(selected.entity, selected.source)
        });
    let trees: Vec<Entity> = world
        .query_filtered::<Entity, With<InspectorTreeView>>()
        .iter(world)
        .collect();
    for tree in trees {
        if let Some(mut selected) = world.get_mut::<SelectedTreeItem>(tree)
            && selected.0 != row
        {
            selected.0 = row;
        }
    }
}

fn header_row() -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @expandable: true,
            @label: bsn_list! { @row_label() },
        }
    }
}

fn entity_row(expandable: bool) -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @expandable: {expandable},
            @label: bsn_list! { @row_label() },
        }
    }
}

fn row_label() -> impl Scene {
    bsn! {
        InspectorRowLabel
        @caption("")
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
    let Some(ids) = shown_components(world, entity) else {
        return entity.to_string();
    };

    let component_data: Vec<(ComponentId, String, LabelDefinitionPriority)> = ids
        .into_iter()
        .filter_map(|component_id| {
            let priority = world
                .components()
                .get_info(component_id)
                .and_then(ComponentInfo::type_id)
                .and_then(|type_id| priorities?.get_priority_by_type_id(type_id))?;
            Some((
                component_id,
                crate::component_short_name(world, component_id),
                priority,
            ))
        })
        .collect();

    let label_data: Vec<ComponentLabelData> = component_data
        .iter()
        .map(|(component_id, short_name, priority)| ComponentLabelData {
            component_id: *component_id,
            short_name: short_name.as_str(),
            label_definition_priority: Some(*priority),
        })
        .collect();

    resolve_label(world, entity, &label_data)
        .map(|label| label.label.as_str().to_string())
        .unwrap_or_else(|| entity.to_string())
}

/// The components of `entity` the inspector shows, see [`entity_label`].
fn shown_components(world: &World, entity: Entity) -> Option<Vec<ComponentId>> {
    let entity_ref = world.get_entity(entity).ok()?;
    #[cfg(feature = "remote")]
    if let Some(record) = entity_ref.get::<crate::remote::RemoteComponents>() {
        return Some(record.reported().collect());
    }
    Some(entity_ref.archetype().components().to_vec())
}

/// Matches the label of `entity`, then the short names of its shown components, against
/// `pattern`, returning the matched label chars, which are empty for a component match.
///
/// `component_matches` caches whether each component name matched.
///
/// [`Name`] and the hierarchy components are not matched by name, since nearly every entity has
/// one, and neither are components whose type name is unknown.
fn entity_match(
    world: &World,
    priorities: Option<&LabelResolutionRegistry>,
    pattern: &FuzzyPattern,
    entity: &InspectableEntity,
    component_matches: &mut HashMap<(InspectorSource, ComponentId), bool>,
) -> Option<Vec<usize>> {
    let label = entity_label(world, priorities, entity.entity);
    if let Some(matched) = pattern.match_str(&label) {
        return Some(matched.indices);
    }
    let skipped = [
        world.component_id::<Name>(),
        world.component_id::<ChildOf>(),
        world.component_id::<Children>(),
    ];
    shown_components(world, entity.entity)
        .unwrap_or_default()
        .into_iter()
        .any(|component_id| {
            *component_matches
                .entry((entity.source, component_id))
                .or_insert_with(|| {
                    !skipped.contains(&Some(component_id))
                        && has_known_name(world, component_id)
                        && pattern
                            .score(&crate::component_short_name(world, component_id))
                            .is_some()
                })
        })
        .then(Vec::new)
}

/// Whether the short name of a component names its type, which it does not for an unregistered
/// type while the `debug` feature is off.
fn has_known_name(world: &World, component_id: ComponentId) -> bool {
    DebugName::ENABLED
        || world
            .components()
            .get_info(component_id)
            .and_then(ComponentInfo::type_id)
            .zip(world.get_resource::<AppTypeRegistry>())
            .is_some_and(|(type_id, registry)| registry.read().contains(type_id))
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

    fn spawn_tree(app: &mut App) -> (Entity, Entity) {
        let panel = app.world_mut().spawn(InspectorUi).id();
        let tree = app
            .world_mut()
            .spawn((
                InspectorTreeView,
                bevy_ui_widgets::TreeView::default(),
                ChildOf(panel),
            ))
            .id();
        app.world_mut()
            .entity_mut(tree)
            .observe(inspector_tree_selected);
        (panel, tree)
    }

    fn sync(app: &mut App) {
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
    }

    fn set_filter(app: &mut App, query: &str) {
        app.world_mut().resource_mut::<EntityTreeFilter>().0 = query.to_string();
        app.update();
    }

    fn local_rows(app: &App, tree: Entity) -> Vec<Entity> {
        let headers = header_rows(app.world(), tree);
        row_sources(app.world(), local_header(&headers).unwrap())
    }

    fn row_of(app: &App, inspected: Entity) -> Entity {
        app.world()
            .resource::<TreeRowIndex>()
            .row(inspected, InspectorSource::Local)
            .unwrap()
    }

    fn has_row(app: &App, inspected: Entity) -> bool {
        app.world()
            .resource::<TreeRowIndex>()
            .row(inspected, InspectorSource::Local)
            .is_some()
    }

    fn child_rows(app: &App, inspected: Entity) -> Vec<Entity> {
        let container =
            child_with::<FeathersTreeItemChildren>(app.world(), row_of(app, inspected)).unwrap();
        row_sources(app.world(), container)
    }

    fn is_expanded(app: &App, inspected: Entity) -> bool {
        let row = app.world().entity(row_of(app, inspected));
        row.contains::<Expanded>() && row.contains::<InspectorRowPopulated>()
    }

    fn label_of(app: &App, inspected: Entity) -> InspectorRowLabel {
        let label = row_label_entity(app.world(), row_of(app, inspected)).unwrap();
        app.world().get::<InspectorRowLabel>(label).unwrap().clone()
    }

    fn label_text(app: &App, inspected: Entity) -> String {
        let label = row_label_entity(app.world(), row_of(app, inspected)).unwrap();
        app.world().get::<Text>(label).unwrap().0.clone()
    }

    fn label_spans(app: &App, inspected: Entity) -> Vec<(String, bool)> {
        let label = row_label_entity(app.world(), row_of(app, inspected)).unwrap();
        app.world()
            .get::<Children>(label)
            .map(|spans| {
                spans
                    .iter()
                    .map(|span| {
                        let span = app.world().entity(*span);
                        (
                            span.get::<TextSpan>().unwrap().0.clone(),
                            span.contains::<ThemeTextColor>(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn expand(app: &mut App, inspected: Entity) {
        let row = row_of(app, inspected);
        app.world_mut()
            .entity_mut(row)
            .insert((InspectorRowPopulated, Expanded));
        sync(app);
    }

    fn select(app: &mut App, tree: Entity, inspected: Entity) {
        let row = row_of(app, inspected);
        app.world_mut().trigger(ValueChange {
            source: tree,
            value: Some(row),
            is_final: true,
        });
    }

    fn selected_row(app: &App, tree: Entity) -> Option<Entity> {
        app.world().get::<SelectedTreeItem>(tree).unwrap().0
    }

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct Health;

    #[test]
    fn filter_shows_matches_with_their_ancestors_expanded() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        let player = app.world_mut().spawn(Name::new("Player")).id();
        app.world_mut().spawn((Name::new("Sword"), ChildOf(player)));
        let level = app.world_mut().spawn(Name::new("Level")).id();
        let room = app
            .world_mut()
            .spawn((Name::new("Room"), ChildOf(level)))
            .id();
        let enemy = app
            .world_mut()
            .spawn((Name::new("Enemy Player"), ChildOf(room)))
            .id();
        app.world_mut().spawn((Name::new("Wall"), ChildOf(room)));
        app.world_mut().spawn((Name::new("Door"), ChildOf(level)));
        sync(&mut app);
        assert_eq!(local_rows(&app, tree), [panel, player, level]);

        set_filter(&mut app, "  pLaYeR ");
        assert_eq!(local_rows(&app, tree), [player, level]);
        assert!(is_expanded(&app, level));
        assert!(is_expanded(&app, room));
        assert_eq!(child_rows(&app, level), [room]);
        assert_eq!(child_rows(&app, room), [enemy]);
        assert!(label_of(&app, room).matched.is_empty());

        let player_row = row_of(&app, player);
        assert!(!app.world().get::<TreeItem>(player_row).unwrap().expandable);
        assert!(!app.world().entity(player_row).contains::<Expanded>());
        let enemy_row = row_of(&app, enemy);
        assert!(!app.world().get::<TreeItem>(enemy_row).unwrap().expandable);

        set_filter(&mut app, "enemy");
        assert_eq!(local_rows(&app, tree), [level]);
        assert_eq!(child_rows(&app, room), [enemy]);
        assert!(!has_row(&app, player));
    }

    #[test]
    fn filter_re_expands_ancestors_collapsed_by_the_user() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let level = app.world_mut().spawn(Name::new("Level")).id();
        let target = app
            .world_mut()
            .spawn((Name::new("Target"), ChildOf(level)))
            .id();
        sync(&mut app);

        set_filter(&mut app, "targ");
        assert_eq!(local_rows(&app, tree), [level]);
        assert!(is_expanded(&app, level));
        let level_row = row_of(&app, level);
        app.world_mut().entity_mut(level_row).remove::<Expanded>();

        set_filter(&mut app, "target");
        assert!(is_expanded(&app, level));
        assert_eq!(child_rows(&app, level), [target]);
    }

    #[test]
    fn filter_matches_fuzzily_and_highlights_the_label() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let light = app.world_mut().spawn(Name::new("SpotLight")).id();
        app.world_mut().spawn(Name::new("Camera"));

        set_filter(&mut app, "slt");
        assert_eq!(local_rows(&app, tree), [light]);
        assert_eq!(label_of(&app, light).matched, [0, 4, 8]);
        assert_eq!(
            label_spans(&app, light),
            [
                ("S".to_string(), true),
                ("pot".to_string(), false),
                ("L".to_string(), true),
                ("igh".to_string(), false),
                ("t".to_string(), true),
            ]
        );
        assert_eq!(label_text(&app, light), "");

        set_filter(&mut app, "");
        assert_eq!(label_of(&app, light).matched, Vec::<usize>::new());
        assert!(label_spans(&app, light).is_empty());
        assert_eq!(label_text(&app, light), "SpotLight");
    }

    #[test]
    fn filter_matches_component_names_without_highlight() {
        let mut app = test_app();
        app.register_type::<Health>();
        let (_, tree) = spawn_tree(&mut app);
        let alpha = app.world_mut().spawn((Name::new("Alpha"), Health)).id();
        app.world_mut().spawn(Name::new("Beta"));

        set_filter(&mut app, "hlth");
        assert_eq!(local_rows(&app, tree), [alpha]);
        assert!(label_of(&app, alpha).matched.is_empty());
        assert!(label_spans(&app, alpha).is_empty());
        assert_eq!(label_text(&app, alpha), "Alpha");
    }

    #[test]
    fn filter_ignores_name_and_hierarchy_component_names() {
        let mut app = test_app();
        app.register_type::<Name>()
            .register_type::<ChildOf>()
            .register_type::<Children>();
        let (_, tree) = spawn_tree(&mut app);
        let parent = app.world_mut().spawn(Name::new("Alpha")).id();
        app.world_mut().spawn((Name::new("Beta"), ChildOf(parent)));

        for query in ["name", "childof", "children"] {
            set_filter(&mut app, query);
            assert!(local_rows(&app, tree).is_empty(), "{query}");
        }
    }

    #[test]
    fn filter_skips_excluded_entities() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        app.world_mut().spawn((Name::new("Hidden"), ChildOf(panel)));
        let shown = app.world_mut().spawn(Name::new("Hidden too")).id();

        set_filter(&mut app, "hidden");
        assert_eq!(local_rows(&app, tree), [shown]);
    }

    #[test]
    fn clearing_the_filter_restores_the_expanded_rows() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        let child = app
            .world_mut()
            .spawn((Name::new("Child"), ChildOf(parent)))
            .id();
        let grandchild = app
            .world_mut()
            .spawn((Name::new("Grandchild"), ChildOf(child)))
            .id();
        let other = app.world_mut().spawn(Name::new("Other")).id();
        let leaf = app
            .world_mut()
            .spawn((Name::new("Leafy"), ChildOf(other)))
            .id();
        sync(&mut app);
        expand(&mut app, parent);
        expand(&mut app, child);
        assert_eq!(child_rows(&app, child), [grandchild]);

        set_filter(&mut app, "leafy");
        assert_eq!(local_rows(&app, tree), [other]);
        assert_eq!(child_rows(&app, other), [leaf]);

        set_filter(&mut app, "");
        assert_eq!(local_rows(&app, tree), [panel, parent, other]);
        assert!(is_expanded(&app, parent));
        assert!(is_expanded(&app, child));
        assert_eq!(child_rows(&app, parent), [child]);
        assert_eq!(child_rows(&app, child), [grandchild]);
        assert!(!app
            .world()
            .entity(row_of(&app, other))
            .contains::<Expanded>());
    }

    #[test]
    fn restores_only_rows_whose_entities_still_exist() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        app.world_mut().spawn((Name::new("Child"), ChildOf(parent)));
        sync(&mut app);
        expand(&mut app, parent);

        set_filter(&mut app, "child");
        app.world_mut().entity_mut(parent).despawn();
        set_filter(&mut app, "");
        assert_eq!(local_rows(&app, tree), [panel]);
    }

    #[test]
    fn clearing_the_filter_follows_entities_that_changed_parents() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        let first = app.world_mut().spawn(Name::new("First")).id();
        let second = app.world_mut().spawn(Name::new("Second")).id();
        let moved = app
            .world_mut()
            .spawn((Name::new("Moved"), ChildOf(first)))
            .id();
        let stay = app
            .world_mut()
            .spawn((Name::new("Stay"), ChildOf(second)))
            .id();
        sync(&mut app);
        expand(&mut app, first);

        set_filter(&mut app, "moved");
        assert_eq!(child_rows(&app, first), [moved]);
        app.world_mut().entity_mut(moved).insert(ChildOf(second));
        sync(&mut app);
        assert_eq!(local_rows(&app, tree), [second]);
        assert_eq!(child_rows(&app, second), [moved]);

        set_filter(&mut app, "");
        assert_eq!(local_rows(&app, tree), [panel, first, second]);
        assert!(child_rows(&app, first).is_empty());
        assert_eq!(child_rows(&app, second), [stay, moved]);
        assert!(!app
            .world()
            .entity(row_of(&app, second))
            .contains::<Expanded>());
    }

    #[test]
    fn selection_survives_filtering() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        let target = app
            .world_mut()
            .spawn((Name::new("Target"), ChildOf(parent)))
            .id();
        app.world_mut().spawn(Name::new("Other"));
        sync(&mut app);
        let selected = Some(InspectableEntity::new(target, InspectorSource::Local));

        set_filter(&mut app, "target");
        select(&mut app, tree, target);
        assert_eq!(app.world().resource::<InspectorSelection>().0, selected);

        set_filter(&mut app, "");
        assert_eq!(selected_row(&app, tree), Some(row_of(&app, target)));
        assert_eq!(app.world().resource::<InspectorSelection>().0, selected);

        set_filter(&mut app, "other");
        assert_eq!(selected_row(&app, tree), None);
        assert_eq!(app.world().resource::<InspectorSelection>().0, selected);
    }

    #[test]
    fn clearing_the_filter_reveals_the_selection() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let level = app.world_mut().spawn(Name::new("Level")).id();
        let room = app
            .world_mut()
            .spawn((Name::new("Room"), ChildOf(level)))
            .id();
        let target = app
            .world_mut()
            .spawn((Name::new("Target"), ChildOf(room)))
            .id();
        let other = app.world_mut().spawn(Name::new("Other")).id();
        app.world_mut().spawn((Name::new("Item"), ChildOf(other)));
        sync(&mut app);

        set_filter(&mut app, "target");
        select(&mut app, tree, target);
        set_filter(&mut app, "item");
        assert!(!has_row(&app, target));

        set_filter(&mut app, "");
        assert!(is_expanded(&app, level));
        assert!(is_expanded(&app, room));
        assert_eq!(child_rows(&app, room), [target]);
        assert!(!app
            .world()
            .entity(row_of(&app, other))
            .contains::<Expanded>());
        assert_eq!(selected_row(&app, tree), Some(row_of(&app, target)));

        sync(&mut app);
        assert!(is_expanded(&app, room));

        app.world_mut().entity_mut(target).despawn();
        set_filter(&mut app, "room");
        set_filter(&mut app, "");
        assert!(is_expanded(&app, room));
    }

    #[test]
    fn caps_the_filtered_matches_in_tree_order() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        let note = app
            .world_mut()
            .spawn((
                EntityTreeFilterNote,
                Text::default(),
                Node::default(),
                ChildOf(panel),
            ))
            .id();
        let group = app.world_mut().spawn(Name::new("Group")).id();
        let items: Vec<Entity> = (0..MAX_FILTER_RESULTS + 20)
            .map(|index| {
                app.world_mut()
                    .spawn((Name::new(format!("Item {index}")), ChildOf(group)))
                    .id()
            })
            .collect();

        set_filter(&mut app, "item");
        assert_eq!(local_rows(&app, tree), [group]);
        assert_eq!(child_rows(&app, group), items[..MAX_FILTER_RESULTS]);
        assert_eq!(
            app.world().get::<Node>(note).unwrap().display,
            Display::Flex
        );
        assert_eq!(
            app.world().get::<Text>(note).unwrap().0,
            format!("Showing {MAX_FILTER_RESULTS} of {} matches", items.len())
        );

        set_filter(&mut app, "item 7");
        assert!(child_rows(&app, group).len() < MAX_FILTER_RESULTS);
        assert_eq!(
            app.world().get::<Node>(note).unwrap().display,
            Display::None
        );
    }

    #[test]
    fn clearing_the_filter_restores_the_row_order() {
        let mut app = test_app();
        let (panel, tree) = spawn_tree(&mut app);
        let first = app.world_mut().spawn(Name::new("First")).id();
        let second = app.world_mut().spawn(Name::new("Second")).id();
        sync(&mut app);

        set_filter(&mut app, "second");
        assert_eq!(local_rows(&app, tree), [second]);
        set_filter(&mut app, "");
        assert_eq!(local_rows(&app, tree), [panel, first, second]);
        let headers = header_rows(app.world(), tree);
        let header = local_header(&headers).unwrap();
        let children = app.world().get::<Children>(header).unwrap();
        assert!(app
            .world()
            .entity(children[0])
            .contains::<FeathersTreeItemHeader>());
    }

    #[test]
    fn filter_input_shows_a_query_set_by_the_app() {
        let mut app = test_app();
        app.add_plugins(bevy_text::TextPlugin);
        let input = app
            .world_mut()
            .spawn((EntityTreeFilterInput, EditableText::default()))
            .id();
        app.update();

        set_filter(&mut app, "player");
        assert_eq!(
            app.world().get::<EditableText>(input).unwrap().value(),
            "player"
        );
        set_filter(&mut app, "");
        assert_eq!(app.world().get::<EditableText>(input).unwrap().value(), "");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn filters_the_remote_tree() {
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

        let (_, tree) = spawn_tree(&mut app);
        app.world_mut().spawn(Name::new("Local leaf"));
        let parent = Entity::from_raw_u32(40).unwrap();
        let child = Entity::from_raw_u32(41).unwrap();
        let sibling = Entity::from_raw_u32(42).unwrap();
        apply(
            app.world_mut(),
            alloc::vec![
                row(parent, json!({ "bevy_ecs::name::Name": "Remote" })),
                row(
                    child,
                    json!({ "bevy_ecs::name::Name": "Remote leaf", "bevy_ecs::hierarchy::ChildOf": parent })
                ),
                row(
                    sibling,
                    json!({ "bevy_ecs::name::Name": "Rock", "bevy_ecs::hierarchy::ChildOf": parent })
                ),
            ],
        );
        sync(&mut app);

        let remote = InspectorSource::Remote(RemoteSource::new("localhost", 1, WorldKind::Main));
        let remote_header = header_rows(app.world(), tree)
            .into_iter()
            .find_map(|(header, source)| (source == remote).then_some(header))
            .unwrap();
        let remote_row = |app: &App, inspected: Entity| {
            app.world()
                .resource::<TreeRowIndex>()
                .row(inspected, remote)
                .unwrap()
        };
        let remote_label = |app: &App, inspected: Entity| {
            let label = row_label_entity(app.world(), remote_row(app, inspected)).unwrap();
            app.world().get::<InspectorRowLabel>(label).unwrap().clone()
        };
        let remote_children = |app: &App, inspected: Entity| {
            let container =
                child_with::<FeathersTreeItemChildren>(app.world(), remote_row(app, inspected))
                    .unwrap();
            row_sources(app.world(), container)
        };

        set_filter(&mut app, "leaf");
        assert_eq!(row_sources(app.world(), remote_header), [parent]);
        assert!(app
            .world()
            .entity(remote_row(&app, parent))
            .contains::<Expanded>());
        assert_eq!(remote_children(&app, parent), [child]);
        assert!(remote_label(&app, parent).matched.is_empty());
        assert!(!remote_label(&app, child).matched.is_empty());

        set_filter(&mut app, "REMOTE");
        assert_eq!(row_sources(app.world(), remote_header), [parent]);
        assert_eq!(remote_children(&app, parent), [child]);
        assert!(!remote_label(&app, parent).matched.is_empty());
    }
}
