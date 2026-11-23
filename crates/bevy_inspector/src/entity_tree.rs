//! A panel listing the entities of the inspected world, and the systems that keep it in sync.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::cmp::Ordering;

use bevy_app::Propagate;
use bevy_camera::visibility::Visibility;
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
    query::{Added, Changed, Has, Or, With},
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::{IsResource, Resource},
    system::{Commands, Query, Res, ResMut, SystemIdMarker},
    template::template,
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    containers::{subpane, subpane_body, subpane_header},
    controls::{
        FeathersTextInput, FeathersTextInputContainer, FeathersTreeItem, FeathersTreeItemChildren,
        FeathersTreeItemHeader, FeathersTreeItemToggle, FeathersTreeView,
    },
    display::{caption, icon},
    font_styles::InheritableFont,
    theme::{
        SurfaceLevel, ThemeBackgroundColor, ThemeContext, ThemeTextColor, ThemeToken, ThemedText,
    },
    tokens,
};
use bevy_fuzzy::{segments, FuzzyPattern};
use bevy_log::warn;
use bevy_picking::{events::PointerClick, pointer::PointerButton, Pickable};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_text::{
    EditableText, FontSource, FontWeight, Placeholder, TextEdit, TextEditChange, TextFont, TextSpan,
};
use bevy_ui::{
    percent, px, widget::Text, BorderRadius, ComputedNode, Display, Expanded, Node, Overflow,
    PositionType, ScrollIntoView, Selected, UiRect, Val,
};
use bevy_ui_widgets::{
    tree_view_expand_self_update, tree_view_self_update, SelectedTreeItem, TreeItem,
    TreeItemChildren, TreeItemExpandChange, ValueChange,
};
use bevy_utils::DebugName;

use crate::{InspectableEntity, InspectorSelection, InspectorSource, InspectorSources, PanelSync};

/// The minimum height of a row's header line, in logical pixels.
const ROW_HEIGHT: f32 = 21.0;
/// The width of an [`InspectorTreeGuide`], in logical pixels.
const GUIDE_WIDTH: f32 = 3.0;

const EYE: &str = "embedded://bevy_inspector/assets/icons/eye.png";
const EYE_CLOSED: &str = "embedded://bevy_inspector/assets/icons/eye-closed.png";

/// Marker for root entities spawned by the inspector. An `InspectorUi` entity and its descendants
/// are hidden from the entity tree, and clicks on them do not select entities.
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

const FILTER_ICON: &str = "embedded://bevy_inspector/assets/icons/filter.png";

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

/// The vertical line drawn alongside the child rows of an expanded row.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorTreeGuide;

/// The dot shown in place of the disclosure chevron of a row without children.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorLeafDot;

/// The eye at the end of a row whose entity has a [`Visibility`]. Clicking it hides or shows that
/// entity.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorVisibilityToggle {
    /// The entity whose visibility the eye toggles.
    pub inspected: Entity,
    /// Whether the entity's own [`Visibility`] is [`Visibility::Hidden`].
    pub hidden: bool,
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
pub type EntityTreeSync = PanelSync<InspectorTreeView>;

#[derive(Resource, Debug, Default)]
pub(crate) struct TreeReveal(pub(crate) Option<Entity>);

/// The filter the entity tree last applied, and the rows that were expanded before it.
#[derive(Resource, Debug, Default)]
pub struct EntityTreeFilterState {
    applied: String,
    expanded_before: Option<HashSet<InspectableEntity>>,
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
            min_height: px(0),
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
                @icon(FILTER_ICON)
                --
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
            InspectorTreeView
            @FeathersTreeView
            Node {
                flex_grow: 1.0,
                min_height: px(0),
            }
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
    headers: Query<(), With<InspectorHeaderRow>>,
    mut selection: ResMut<InspectorSelection>,
) {
    if !trees.contains(change.source) {
        return;
    }
    if change.value.is_some_and(|row| headers.contains(row)) {
        return;
    }

    selection.0 = change
        .value
        .and_then(|row| rows.get(row).ok())
        .map(|row| InspectableEntity::new(row.inspected, row.source));
}

/// Observer that starts keeping a row's child rows in sync the first time it is expanded.
///
/// While a filter is active, it also records the user's own expansions and collapses, so that
/// clearing the filter keeps them. Expansions the filter forces on ancestors do not go through
/// this observer.
pub fn inspector_tree_expanded(
    change: On<TreeItemExpandChange>,
    rows: Query<
        Option<&InspectorEntityRow>,
        Or<(With<InspectorHeaderRow>, With<InspectorEntityRow>)>,
    >,
    filter: Res<EntityTreeFilter>,
    mut sync: ResMut<EntityTreeSync>,
    mut filter_state: ResMut<EntityTreeFilterState>,
    mut commands: Commands,
) {
    let Ok(row) = rows.get(change.item) else {
        return;
    };
    if let Some(row) = row
        && !filter.0.trim().is_empty()
        && let Some(expanded) = &mut filter_state.expanded_before
    {
        let inspected = InspectableEntity::new(row.inspected, row.source);
        match change.expanded {
            true => expanded.insert(inspected),
            false => expanded.remove(&inspected),
        };
    }
    if !change.expanded {
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
    let query = world
        .get_resource::<EntityTreeFilter>()
        .map(|filter| filter.0.trim().to_string())
        .unwrap_or_default();
    let due = EntityTreeSync::tick(world);
    let mut state = world.resource_mut::<EntityTreeFilterState>();
    let query_changed = state.applied != query;
    if !due && !query_changed {
        return;
    }
    let entering = state.applied.is_empty() && !query.is_empty();
    let leaving = !state.applied.is_empty() && query.is_empty();
    state.applied.clone_from(&query);
    let mut restore = leaving.then(|| state.expanded_before.take().unwrap_or_default());
    if let Some(restore) = &mut restore {
        restore.extend(selection_ancestors(world));
    }

    if entering {
        let expanded = expanded_rows(world);
        world
            .resource_mut::<EntityTreeFilterState>()
            .expanded_before = Some(expanded);
    }
    sync_filtered_rows(world, &query, query_changed, restore.as_ref());
}

/// Brings the tree rows up to date with the filter that is already applied.
fn sync_rows(world: &mut World) {
    let query = world.resource::<EntityTreeFilterState>().applied.clone();
    sync_filtered_rows(world, &query, false, None);
}

fn sync_filtered_rows(
    world: &mut World,
    query: &str,
    query_changed: bool,
    restore: Option<&HashSet<InspectableEntity>>,
) {
    let pattern = FuzzyPattern::new(query);
    let Some(tree_view) = world
        .query_filtered::<Entity, With<InspectorTreeView>>()
        .iter(world)
        .next()
    else {
        return;
    };
    // A pass only spawns and despawns inspector rows, which `is_excluded` keeps out of the
    // roots, so the inspected tree is the same for every pass of the loop below.
    let roots = Sources::new(world).roots();
    let filter = (!pattern.is_empty()).then(|| FilteredTree::new(world, &roots, &pattern));
    loop {
        let plan = plan_sync(
            world,
            tree_view,
            &roots,
            filter.as_ref(),
            query_changed,
            restore,
        );
        let descended = apply_sync(world, plan);
        rebuild_index(world);
        if !descended {
            break;
        }
    }
    restore_selection(world);
}

pub(crate) fn reveal_in_tree(world: &mut World) {
    let Some(target) = world.resource::<TreeReveal>().0 else {
        return;
    };
    let selected = world.resource::<InspectorSelection>().0
        == Some(InspectableEntity::new(target, InspectorSource::Local));
    // An excluded entity never gets a row, so waiting for one would retry forever.
    if !selected || world.get_entity(target).is_err() || is_excluded(world, target) {
        world.resource_mut::<TreeReveal>().0 = None;
        return;
    }

    let mut ancestors = Vec::new();
    let mut current = target;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        current = child_of.parent();
        ancestors.push(current);
    }
    ancestors.reverse();

    let mut expanded = false;
    for _ in 0..=ancestors.len() {
        let index = world.resource::<TreeRowIndex>();
        let collapsed: Vec<(Entity, Entity)> = ancestors
            .iter()
            .map_while(|ancestor| index.row(*ancestor, InspectorSource::Local))
            .filter(|row| {
                world
                    .get_entity(*row)
                    .is_ok_and(|row| !row.contains::<Expanded>())
            })
            .filter_map(|row| Some((owning_tree(world, row)?, row)))
            .collect();
        if collapsed.is_empty() {
            break;
        }
        for (tree, item) in collapsed {
            world.trigger(TreeItemExpandChange {
                tree,
                item,
                expanded: true,
            });
        }
        world.flush();
        sync_rows(world);
        expanded = true;
    }
    if expanded {
        return;
    }

    let Some(row) = world
        .resource::<TreeRowIndex>()
        .row(target, InspectorSource::Local)
    else {
        return;
    };
    let header = child_with::<FeathersTreeItemHeader>(world, row).unwrap_or(row);
    if world
        .get::<ComputedNode>(header)
        .is_some_and(ComputedNode::is_empty)
    {
        return;
    }
    world.resource_mut::<TreeReveal>().0 = None;
    world.trigger(ScrollIntoView::new(header));
}

fn owning_tree(world: &World, row: Entity) -> Option<Entity> {
    let mut current = row;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        current = child_of.parent();
        if world
            .get_entity(current)
            .is_ok_and(|entity| entity.contains::<InspectorTreeView>())
        {
            return Some(current);
        }
    }
    None
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
    hidden: bool,
    eye: Option<bool>,
}

struct EyeUpdate {
    row: Entity,
    inspected: Entity,
    hidden: Option<bool>,
}

/// The set of changes a single synchronization pass needs to apply.
///
/// [`plan_sync`] reads the world without mutating it, diffing each header row against the current
/// root entities of its source and each populated row's children container against that row's
/// entity's current children (see [`diff_container`]). Rows that were never expanded are not
/// diffed. The result records which rows to despawn, which to spawn, which labels to update or
/// dim, which `expandable` flags to flip, which rows to expand or collapse and which containers to
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
    dim: Vec<(Entity, bool)>,
    expandable: Vec<(Entity, bool)>,
    eyes: Vec<EyeUpdate>,
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
/// `tree_view` and `roots` are the same for every pass of a synchronization, so the caller
/// computes them once.
///
/// The rows live in `world`, while the entities they display are read from the inspected world,
/// see [`crate::world_to_inspect`].
fn plan_sync(
    world: &mut World,
    tree_view: Entity,
    roots: &[(InspectorSource, Vec<InspectableEntity>)],
    filter: Option<&FilteredTree>,
    query_changed: bool,
    restore: Option<&HashSet<InspectableEntity>>,
) -> SyncPlan {
    let mut plan = SyncPlan::default();
    let mut rows = Vec::new();
    let mut row_query = world.query::<(
        Entity,
        &InspectorEntityRow,
        Has<InspectorRowPopulated>,
        Has<Expanded>,
    )>();
    for (row, inspected, populated, expanded) in row_query.iter(world) {
        rows.push(RowState {
            row,
            inspected: InspectableEntity::new(inspected.inspected, inspected.source),
            populated,
            expanded,
        });
    }

    let index = world.resource::<TreeRowIndex>();

    let sources = Sources::new(world);
    diff_header_container(
        world,
        rows_container(world, tree_view),
        &sources.ordered(),
        &mut plan,
    );

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

    for (source, roots) in roots {
        let Some(container) = index
            .source_to_row
            .get(source)
            .and_then(|header| child_with::<FeathersTreeItemChildren>(world, *header))
        else {
            continue;
        };
        let roots: Vec<InspectableEntity> = roots
            .iter()
            .filter(|root| shown.keeps(root))
            .cloned()
            .collect();
        diff_container(world, &shown, container, &roots, &mut plan);
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
    fn new(
        world: &World,
        roots: &[(InspectorSource, Vec<InspectableEntity>)],
        pattern: &FuzzyPattern,
    ) -> Self {
        let sources = Sources::new(world);
        let mut filter = Self {
            kept: HashSet::new(),
            parents: HashSet::new(),
            highlights: HashMap::new(),
            total: 0,
        };
        let mut matches = Vec::new();
        let mut component_matches = HashMap::new();
        let mut stack: Vec<InspectableEntity> = roots
            .iter()
            .flat_map(|(_, roots)| roots.iter().cloned())
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

/// The [`TreeItemChildren`] container inside `tree_view` that holds its root rows, or `tree_view`
/// itself when it has none.
pub(crate) fn rows_container(world: &World, tree_view: Entity) -> Entity {
    child_with::<TreeItemChildren>(world, tree_view).unwrap_or(tree_view)
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
        let hidden = is_hidden(inspected_world, inspected.entity);
        let eye = (inspected.source == InspectorSource::Local)
            .then(|| inspected_world.get::<Visibility>(inspected.entity))
            .flatten()
            .map(|visibility| *visibility == Visibility::Hidden);
        let Some(row) = existing.get(inspected).copied() else {
            plan.spawn.push(RowSpawn {
                container,
                inspected: inspected.entity,
                source: inspected.source,
                label,
                expandable,
                expanded: expandable && shown.expands(inspected),
                hidden,
                eye,
            });
            continue;
        };

        if eye_state(world, row) != eye {
            plan.eyes.push(EyeUpdate {
                row,
                inspected: inspected.entity,
                hidden: eye,
            });
        }

        if let Some(label_entity) = row_label_entity(world, row) {
            if world.get::<InspectorRowLabel>(label_entity) != Some(&label) {
                plan.relabel.push((label_entity, label));
            }
            if world
                .get::<ThemeTextColor>(label_entity)
                .is_some_and(|color| color.0 != row_label_color(hidden))
            {
                plan.dim.push((label_entity, hidden));
            }
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

    for (label_entity, hidden) in plan.dim {
        if let Ok(mut label) = world.get_entity_mut(label_entity) {
            label.insert(ThemeTextColor(row_label_color(hidden)));
        }
    }

    for update in plan.eyes {
        if let Some(eye) = row_eye(world, update.row) {
            world.entity_mut(eye).despawn();
        }
        if let Some(hidden) = update.hidden {
            spawn_eye(world, update.row, update.inspected, hidden);
        }
    }

    for (row, expandable) in plan.expandable {
        if let Some(mut item) = world.get_mut::<TreeItem>(row) {
            item.expandable = expandable;
        }
        show_leaf_dot(world, row, !expandable);
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
        if let Some(mut node) = world.get_mut::<Node>(spawn.container) {
            node.row_gap = px(2);
        }
        let row = match world.spawn_scene(header_row()) {
            Ok(mut row) => {
                row.insert((
                    InspectorHeaderRow {
                        source: spawn.source,
                    },
                    Expanded,
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
        style_row(world, row, true);
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
        let row = match world.spawn_scene(entity_row(spawn.expandable, spawn.hidden)) {
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
        style_row(world, row, spawn.expandable);
        if let Some(hidden) = spawn.eye {
            spawn_eye(world, row, spawn.inspected, hidden);
        }
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
    if let Some(mut state) = world.get_resource_mut::<EntityTreeFilterState>() {
        state.expanded_before = None;
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

/// Points the tree views' [`SelectedTreeItem`] at the row of the [`InspectorSelection`].
///
/// Rows are despawned whenever their entity is filtered out or misses the filter, so the tree's
/// [`SelectedTreeItem`] is cleared then and set again once the row is back.
/// [`InspectorSelection`] itself is left alone, and a selected header row stays selected while
/// nothing is inspected.
fn restore_selection(world: &mut World) {
    let selection = world
        .get_resource::<InspectorSelection>()
        .and_then(|selection| selection.0.clone());
    let row = selection.as_ref().and_then(|selected| {
        world
            .resource::<TreeRowIndex>()
            .row(selected.entity, selected.source)
    });
    let trees: Vec<Entity> = world
        .query_filtered::<Entity, With<InspectorTreeView>>()
        .iter(world)
        .collect();
    for tree in trees {
        let header_selected = world
            .get::<SelectedTreeItem>(tree)
            .and_then(|selected| selected.0)
            .is_some_and(|row| world.get::<InspectorHeaderRow>(row).is_some());
        if selection.is_none() && header_selected {
            continue;
        }
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
            @label: bsn_list! { @row_label(String::new()) },
        }
    }
}

fn entity_row(expandable: bool, hidden: bool) -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @expandable: {expandable},
            @label: bsn_list! {
                @row_label(String::new())
                ThemeTextColor({row_label_color(hidden)})
            },
        }
    }
}

pub(crate) fn row_label(text: String) -> impl Scene {
    bsn! {
        InspectorRowLabel { text: {text.clone()} }
        @caption(text)
        Node {
            min_width: px(0),
            overflow: Overflow::clip(),
        }
    }
}

/// The text color of the label of a row, dimmed for an entity that is hidden.
fn row_label_color(hidden: bool) -> ThemeToken {
    if hidden {
        tokens::LISTROW_TEXT_DISABLED
    } else {
        tokens::LISTROW_TEXT
    }
}

/// Fits the header line of `row` to the compact tree layout, and adds the line drawn alongside its
/// child rows and the dot shown while it has none.
fn style_row(world: &mut World, row: Entity, expandable: bool) {
    if let Some(header) = child_with::<FeathersTreeItemHeader>(world, row) {
        if let Some(mut node) = world.get_mut::<Node>(header) {
            node.min_height = px(ROW_HEIGHT);
            node.padding.top = px(3);
            node.padding.bottom = px(3);
            node.padding.right = px(3);
            node.border_radius = BorderRadius::all(px(4));
        }
        world.entity_mut(header).insert(InheritableFont {
            font: FontSource::Families(fonts::SANS.into()),
            font_size: size::SMALL_FONT,
            weight: FontWeight::NORMAL,
        });
        if let Some(toggle) = child_with::<FeathersTreeItemToggle>(world, header) {
            world.spawn((
                InspectorLeafDot,
                Node {
                    display: if expandable {
                        Display::None
                    } else {
                        Display::Flex
                    },
                    width: px(4),
                    height: px(4),
                    border_radius: BorderRadius::all(px(2)),
                    ..Default::default()
                },
                ThemeBackgroundColor(tokens::TEXT_DIM),
                Pickable::IGNORE,
                ChildOf(toggle),
            ));
        }
    }
    if let Some(children) = child_with::<FeathersTreeItemChildren>(world, row) {
        if let Some(mut node) = world.get_mut::<Node>(children) {
            node.row_gap = px(2);
            node.padding.top = px(2);
        }
        world.spawn((
            InspectorTreeGuide,
            Node {
                position_type: PositionType::Absolute,
                top: px(2),
                bottom: px(0),
                width: px(GUIDE_WIDTH),
                ..Default::default()
            },
            ThemeBackgroundColor(tokens::TREE_GUIDE),
            Pickable::IGNORE,
            ChildOf(children),
        ));
    }
}

/// Shows or hides the [`InspectorLeafDot`] of `row`.
fn show_leaf_dot(world: &mut World, row: Entity, show: bool) {
    let Some(dot) = child_with::<FeathersTreeItemHeader>(world, row)
        .and_then(|header| child_with::<FeathersTreeItemToggle>(world, header))
        .and_then(|toggle| child_with::<InspectorLeafDot>(world, toggle))
    else {
        return;
    };
    let display = if show { Display::Flex } else { Display::None };
    if let Some(mut node) = world.get_mut::<Node>(dot)
        && node.display != display
    {
        node.display = display;
    }
}

/// Lines each [`InspectorTreeGuide`] up under the disclosure chevron of its row, and draws it in
/// the accent color while one of the rows beside it is selected.
///
/// Runs only when a guide is added, or the selection, a row or a row container changes.
pub fn update_tree_guides(
    changed: Query<
        (),
        Or<(
            Added<InspectorTreeGuide>,
            Added<Selected>,
            Changed<SelectedTreeItem>,
            (With<FeathersTreeItemChildren>, Changed<Children>),
            Changed<TreeItem>,
        )>,
    >,
    guides: Query<(Entity, &ChildOf, &ThemeBackgroundColor), With<InspectorTreeGuide>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    selected: Query<(), (With<Selected>, With<InspectorEntityRow>)>,
    headers: Query<&Children, With<FeathersTreeItemHeader>>,
    toggles: Query<(), With<FeathersTreeItemToggle>>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    if changed.is_empty() {
        return;
    }
    for (guide, child_of, color) in &guides {
        let container = child_of.parent();
        let token = match children
            .get(container)
            .is_ok_and(|rows| rows.iter().any(|row| selected.contains(*row)))
        {
            true => tokens::TREE_GUIDE_SELECTED,
            false => tokens::TREE_GUIDE,
        };
        if color.0 != token {
            commands.entity(guide).insert(ThemeBackgroundColor(token));
        }

        let Some((header, toggle)) = parents
            .get(container)
            .ok()
            .and_then(|row| children.get(row.parent()).ok())
            .and_then(|row_children| {
                row_children.iter().find_map(|header| {
                    let toggle = headers
                        .get(*header)
                        .ok()?
                        .iter()
                        .copied()
                        .find(|child| toggles.contains(*child))?;
                    Some((*header, toggle))
                })
            })
        else {
            continue;
        };
        let (Ok(header), Ok(toggle)) = (nodes.get(header), nodes.get(toggle)) else {
            continue;
        };

        let left = header.margin.left + header.padding.left + 0.5 * toggle.width - px(GUIDE_WIDTH);

        if let Ok(mut node) = nodes.get_mut(guide)
            && node.left != left
        {
            node.left = left;
        }
    }
}

/// Whether `entity` is hidden, by its own [`Visibility`] or by that of an ancestor.
fn is_hidden(world: &World, entity: Entity) -> bool {
    let mut current = entity;
    loop {
        match world.get::<Visibility>(current) {
            Some(Visibility::Hidden) => return true,
            Some(Visibility::Visible) => return false,
            _ => {}
        }
        match world.get::<ChildOf>(current) {
            Some(child_of) => current = child_of.parent(),
            None => return false,
        }
    }
}

fn is_excluded(world: &World, entity: Entity) -> bool {
    let Ok(entity_ref) = world.get_entity(entity) else {
        return true;
    };
    if entity_ref.contains::<IsResource>()
        || entity_ref.contains::<SystemIdMarker>()
        || entity_ref.contains::<Observer>()
        || entity_ref.contains::<InspectorUi>()
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

fn row_eye(world: &World, row: Entity) -> Option<Entity> {
    let header = child_with::<FeathersTreeItemHeader>(world, row)?;
    child_with::<InspectorVisibilityToggle>(world, header)
}

fn eye_state(world: &World, row: Entity) -> Option<bool> {
    row_eye(world, row)
        .and_then(|eye| world.get::<InspectorVisibilityToggle>(eye))
        .map(|toggle| toggle.hidden)
}

fn eye(hidden: bool) -> impl Scene {
    let image = if hidden { EYE_CLOSED } else { EYE };
    bsn! {
        @icon(image)
        Node {
            margin: UiRect::left(Val::Auto),
            flex_shrink: 0.0,
        }
    }
}

fn spawn_eye(world: &mut World, row: Entity, inspected: Entity, hidden: bool) {
    let Some(header) = child_with::<FeathersTreeItemHeader>(world, row) else {
        return;
    };
    match world.spawn_scene(eye(hidden)) {
        Ok(mut eye) => {
            eye.insert((
                InspectorVisibilityToggle { inspected, hidden },
                ChildOf(header),
            ))
            .observe(toggle_visibility);
        }
        Err(error) => warn!("failed to spawn an inspector visibility toggle: {error}"),
    }
}

fn toggle_visibility(
    mut click: On<PointerClick>,
    toggles: Query<&InspectorVisibilityToggle>,
    mut visibilities: Query<&mut Visibility>,
    mut sync: ResMut<EntityTreeSync>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    click.propagate(false);
    let Ok(toggle) = toggles.get(click.entity) else {
        return;
    };
    let Ok(mut visibility) = visibilities.get_mut(toggle.inspected) else {
        return;
    };
    *visibility = if *visibility == Visibility::Hidden {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    sync.set_dirty();
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

/// Returns true if the component's name is its real type name. Unregistered types have no real
/// name unless the `debug` feature is on.
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

    fn header_rows(world: &World, tree: Entity) -> Vec<(Entity, InspectorSource)> {
        world
            .get::<Children>(rows_container(world, tree))
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

    fn row_sources(world: &World, row: Entity) -> Vec<Entity> {
        child_with::<FeathersTreeItemChildren>(world, row)
            .and_then(|container| world.get::<Children>(container))
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
    fn excludes_inspector_ui_and_its_descendants() {
        let mut world = World::new();
        let panel = world.spawn(InspectorUi).id();
        let inner = world.spawn(ChildOf(panel)).id();
        let subject = world.spawn_empty().id();

        let nested_panel = world.spawn((InspectorUi, ChildOf(subject))).id();

        assert!(is_excluded(&world, panel));
        assert!(is_excluded(&world, inner));
        assert!(!is_excluded(&world, subject));
        assert!(is_excluded(&world, nested_panel));
    }

    #[test]
    fn nested_inspector_ui_entities_stay_hidden() {
        let mut world = World::new();
        let panel = world.spawn(InspectorUi).id();
        let nested = world.spawn((InspectorUi, ChildOf(panel))).id();
        let nested_inner = world.spawn(ChildOf(nested)).id();

        assert!(is_excluded(&world, panel));
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
    fn revealing_a_row_scrolls_it_into_view() {
        #[derive(Resource, Default)]
        struct Scrolled(usize);

        let mut app = test_app();
        let panel = app.world_mut().spawn(InspectorUi).id();
        app.world_mut().spawn((InspectorTreeView, ChildOf(panel)));
        let target = app.world_mut().spawn(Name::new("Target")).id();
        app.init_resource::<Scrolled>();
        app.world_mut()
            .add_observer(|_: On<ScrollIntoView>, mut scrolled: ResMut<Scrolled>| scrolled.0 += 1);
        app.update();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let row = app
            .world()
            .resource::<TreeRowIndex>()
            .row(target, InspectorSource::Local)
            .unwrap();
        let header = child_with::<FeathersTreeItemHeader>(app.world(), row).unwrap_or(row);
        app.world_mut().entity_mut(header).insert(ComputedNode {
            size: bevy_math::Vec2::splat(10.0),
            ..ComputedNode::default()
        });
        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(target, InspectorSource::Local));
        app.world_mut().resource_mut::<TreeReveal>().0 = Some(target);
        app.update();

        assert_eq!(app.world().resource::<Scrolled>().0, 1);
        assert!(app.world().resource::<TreeReveal>().0.is_none());
    }

    #[test]
    fn revealing_an_entity_without_a_row_stops_asking() {
        let mut app = test_app();
        let panel = app.world_mut().spawn(InspectorUi).id();
        app.world_mut().spawn((InspectorTreeView, ChildOf(panel)));
        let hidden_ui = app.world_mut().spawn(InspectorUi).id();
        let excluded = app
            .world_mut()
            .spawn((Name::new("Excluded"), ChildOf(hidden_ui)))
            .id();
        app.update();

        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(excluded, InspectorSource::Local));
        app.world_mut().resource_mut::<TreeReveal>().0 = Some(excluded);
        app.update();

        assert!(app.world().resource::<TreeReveal>().0.is_none());
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
        assert_eq!(row_sources(app.world(), local_header).len(), 500);

        for root in roots.iter().step_by(2) {
            app.world_mut().entity_mut(*root).despawn();
        }
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();

        let sources = row_sources(app.world(), local_header);
        assert_eq!(sources.len(), 250);
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
        assert_eq!(sources.len(), 2);
        assert!(!sources.contains(&panel));
        assert!(sources.contains(&parent));
        assert!(sources.contains(&sibling));
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
        assert_eq!(sources.len(), 1);
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
        app.world_mut().spawn((TreeItemChildren, ChildOf(tree)));
        app.world_mut()
            .entity_mut(tree)
            .observe(inspector_tree_selected)
            .observe(inspector_tree_expanded);
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
        row_sources(app.world(), row_of(app, inspected))
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
        let (_, tree) = spawn_tree(&mut app);
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
        assert_eq!(local_rows(&app, tree), [player, level]);

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
        let (_, tree) = spawn_tree(&mut app);
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
        assert_eq!(local_rows(&app, tree), [parent, other]);
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
        let (_, tree) = spawn_tree(&mut app);
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        app.world_mut().spawn((Name::new("Child"), ChildOf(parent)));
        sync(&mut app);
        expand(&mut app, parent);

        set_filter(&mut app, "child");
        app.world_mut().entity_mut(parent).despawn();
        set_filter(&mut app, "");
        assert!(local_rows(&app, tree).is_empty());
    }

    #[test]
    fn rows_the_user_expands_while_filtering_are_kept() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let level = app.world_mut().spawn(Name::new("Level")).id();
        let room = app
            .world_mut()
            .spawn((Name::new("Room"), ChildOf(level)))
            .id();
        app.world_mut().spawn((Name::new("Target"), ChildOf(room)));
        sync(&mut app);
        expand(&mut app, level);
        assert!(has_row(&app, room));
        assert!(!is_expanded(&app, room));

        set_filter(&mut app, "target");
        let level_row = row_of(&app, level);
        app.world_mut().entity_mut(level_row).remove::<Expanded>();

        // The filter expanded Room's row before Level was collapsed, so it is still indexed.
        let room_row = row_of(&app, room);
        app.world_mut().trigger(TreeItemExpandChange {
            tree,
            item: room_row,
            expanded: true,
        });
        app.update();

        set_filter(&mut app, "");
        assert!(is_expanded(&app, level));
        assert!(is_expanded(&app, room));
    }

    #[test]
    fn clearing_the_filter_follows_entities_that_changed_parents() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
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
        assert_eq!(local_rows(&app, tree), [first, second]);
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
    fn the_selection_is_found_when_it_starts_matching() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let entity = app.world_mut().spawn(Name::new("Alpha")).id();
        sync(&mut app);
        select(&mut app, tree, entity);

        set_filter(&mut app, "bravo");
        assert_eq!(selected_row(&app, tree), None);

        app.world_mut()
            .entity_mut(entity)
            .insert(Name::new("Bravo"));
        sync(&mut app);
        assert_eq!(selected_row(&app, tree), Some(row_of(&app, entity)));
    }

    #[test]
    fn clearing_the_filter_restores_the_row_order() {
        let mut app = test_app();
        let (_, tree) = spawn_tree(&mut app);
        let first = app.world_mut().spawn(Name::new("First")).id();
        let second = app.world_mut().spawn(Name::new("Second")).id();
        sync(&mut app);

        set_filter(&mut app, "second");
        assert_eq!(local_rows(&app, tree), [second]);
        set_filter(&mut app, "");
        assert_eq!(local_rows(&app, tree), [first, second]);
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
            .observe(inspector_tree_filter_changed)
            .id();
        app.update();

        set_filter(&mut app, "player");
        assert_eq!(
            app.world().get::<EditableText>(input).unwrap().value(),
            "player"
        );
        set_filter(&mut app, "");
        assert_eq!(app.world().get::<EditableText>(input).unwrap().value(), "");

        app.world_mut()
            .get_mut::<EditableText>(input)
            .unwrap()
            .queue_edit(TextEdit::Insert("leaf".into()));
        app.update();
        assert_eq!(app.world().resource::<EntityTreeFilter>().0, "leaf");
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
        let remote_children =
            |app: &App, inspected: Entity| row_sources(app.world(), remote_row(app, inspected));

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

    fn tree_app() -> (App, Entity) {
        let mut app = test_app();
        app.add_plugins(bevy_ui_widgets::TreePlugin);
        let panel = app.world_mut().spawn(InspectorUi).id();
        let tree = app
            .world_mut()
            .spawn((
                InspectorTreeView,
                bevy_ui_widgets::TreeView::default(),
                ChildOf(panel),
            ))
            .id();
        app.world_mut().spawn((TreeItemChildren, ChildOf(tree)));
        (app, tree)
    }

    #[test]
    fn child_rows_are_nested_one_level_deeper() {
        let (mut app, tree) = tree_app();
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        let child = app
            .world_mut()
            .spawn((Name::new("Child"), ChildOf(parent)))
            .id();
        app.update();
        sync(&mut app);
        expand(&mut app, parent);
        app.update();

        let header = local_header(&header_rows(app.world(), tree)).unwrap();
        assert!(app.world().entity(header).contains::<Expanded>());
        assert!(row_sources(app.world(), header).contains(&parent));
        let level = |app: &App, entity| {
            app.world()
                .get::<TreeItem>(row_of(app, entity))
                .unwrap()
                .level
        };
        assert_eq!(level(&app, parent), 1);
        assert_eq!(level(&app, child), 2);
        assert_eq!(row_sources(app.world(), row_of(&app, parent)), [child]);
    }

    fn label_color(app: &App, entity: Entity) -> ThemeToken {
        let label = row_label_entity(app.world(), row_of(app, entity)).unwrap();
        app.world().get::<ThemeTextColor>(label).unwrap().0.clone()
    }

    fn eye_of(app: &App, entity: Entity) -> Option<InspectorVisibilityToggle> {
        row_eye(app.world(), row_of(app, entity))
            .and_then(|eye| app.world().get::<InspectorVisibilityToggle>(eye))
            .copied()
    }

    fn click_eye(app: &mut App, entity: Entity) {
        click_eye_with(app, entity, PointerButton::Primary);
    }

    fn click_eye_with(app: &mut App, entity: Entity, button: PointerButton) {
        use bevy_camera::NormalizedRenderTarget;
        use bevy_picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerId},
        };

        app.world_mut().register_component::<bevy_window::Window>();
        let eye = row_eye(app.world(), row_of(app, entity)).unwrap();
        let location = Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: bevy_math::Vec2::ZERO,
        };
        app.world_mut().trigger(PointerClick {
            entity: eye,
            pointer: Pointer::new(PointerId::Mouse, location),
            button,
            hit: HitData::new(eye, 0.0, None, None),
            duration: core::time::Duration::from_millis(10),
            count: 1,
        });
        app.update();
    }

    #[test]
    fn rows_with_visibility_show_an_eye() {
        let (mut app, _) = tree_app();
        let shown = app.world_mut().spawn(Visibility::Inherited).id();
        let hidden = app.world_mut().spawn(Visibility::Hidden).id();
        let plain = app.world_mut().spawn(Name::new("Plain")).id();
        sync(&mut app);

        assert!(eye_of(&app, shown).is_some_and(|eye| !eye.hidden && eye.inspected == shown));
        assert!(eye_of(&app, hidden).is_some_and(|eye| eye.hidden));
        assert!(eye_of(&app, plain).is_none());
    }

    #[test]
    fn clicking_the_eye_toggles_visibility_without_selecting_the_row() {
        let (mut app, tree) = tree_app();
        app.world_mut()
            .entity_mut(tree)
            .observe(tree_view_self_update)
            .observe(inspector_tree_selected);
        let entity = app.world_mut().spawn(Visibility::Visible).id();
        sync(&mut app);

        click_eye_with(&mut app, entity, PointerButton::Secondary);
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Visible)
        );

        click_eye(&mut app, entity);
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Hidden)
        );
        assert_eq!(app.world().resource::<InspectorSelection>().0, None);
        sync(&mut app);
        assert!(eye_of(&app, entity).is_some_and(|eye| eye.hidden));
        assert_eq!(label_color(&app, entity), tokens::LISTROW_TEXT_DISABLED);

        click_eye(&mut app, entity);
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Inherited)
        );
        sync(&mut app);
        assert!(eye_of(&app, entity).is_some_and(|eye| !eye.hidden));
    }

    #[test]
    fn the_eye_follows_visibility_added_or_changed_elsewhere() {
        let (mut app, _) = tree_app();
        let entity = app.world_mut().spawn(Name::new("Later")).id();
        sync(&mut app);
        assert!(eye_of(&app, entity).is_none());

        app.world_mut()
            .entity_mut(entity)
            .insert(Visibility::Hidden);
        sync(&mut app);
        assert!(eye_of(&app, entity).is_some_and(|eye| eye.hidden));

        app.world_mut().entity_mut(entity).remove::<Visibility>();
        sync(&mut app);
        assert!(eye_of(&app, entity).is_none());
    }

    #[test]
    fn hidden_entities_are_dimmed() {
        let (mut app, _) = tree_app();
        let hidden = app.world_mut().spawn(Visibility::Hidden).id();
        let under_hidden = app.world_mut().spawn(ChildOf(hidden)).id();
        let visible_under_hidden = app
            .world_mut()
            .spawn((Visibility::Visible, ChildOf(hidden)))
            .id();
        let inherited = app.world_mut().spawn(Visibility::Inherited).id();
        let shown = app.world_mut().spawn(Name::new("Shown")).id();
        app.update();
        sync(&mut app);
        expand(&mut app, hidden);
        sync(&mut app);

        assert_eq!(label_color(&app, hidden), tokens::LISTROW_TEXT_DISABLED);
        assert_eq!(
            label_color(&app, under_hidden),
            tokens::LISTROW_TEXT_DISABLED
        );
        assert_eq!(
            label_color(&app, visible_under_hidden),
            tokens::LISTROW_TEXT
        );
        assert_eq!(label_color(&app, inherited), tokens::LISTROW_TEXT);
        assert_eq!(label_color(&app, shown), tokens::LISTROW_TEXT);

        app.world_mut().entity_mut(shown).insert(Visibility::Hidden);
        sync(&mut app);
        assert_eq!(label_color(&app, shown), tokens::LISTROW_TEXT_DISABLED);
    }

    fn leaf_dot_shown(app: &App, entity: Entity) -> bool {
        let world = app.world();
        let header = child_with::<FeathersTreeItemHeader>(world, row_of(app, entity)).unwrap();
        let toggle = child_with::<FeathersTreeItemToggle>(world, header).unwrap();
        let dot = child_with::<InspectorLeafDot>(world, toggle).unwrap();
        world.get::<Node>(dot).unwrap().display != Display::None
    }

    #[test]
    fn rows_without_children_show_a_dot() {
        let (mut app, _) = tree_app();
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        let leaf = app.world_mut().spawn(Name::new("Leaf")).id();
        app.world_mut().spawn(ChildOf(parent));
        app.update();
        sync(&mut app);

        assert!(!leaf_dot_shown(&app, parent));
        assert!(leaf_dot_shown(&app, leaf));

        app.world_mut().spawn(ChildOf(leaf));
        sync(&mut app);
        assert!(!leaf_dot_shown(&app, leaf));
    }

    #[test]
    fn the_guide_beside_the_selected_row_uses_the_accent_color() {
        let (mut app, _) = tree_app();
        let parent = app.world_mut().spawn(Name::new("Parent")).id();
        let child = app.world_mut().spawn(ChildOf(parent)).id();
        app.update();
        sync(&mut app);
        expand(&mut app, parent);

        let guide = |app: &App| {
            let container =
                child_with::<FeathersTreeItemChildren>(app.world(), row_of(app, parent)).unwrap();
            let guide = child_with::<InspectorTreeGuide>(app.world(), container).unwrap();
            app.world()
                .get::<ThemeBackgroundColor>(guide)
                .unwrap()
                .0
                .clone()
        };
        assert_eq!(guide(&app), tokens::TREE_GUIDE);

        let select = |app: &mut App, entity| {
            app.world_mut().resource_mut::<InspectorSelection>().0 =
                Some(InspectableEntity::new(entity, InspectorSource::Local));
            app.update();
            app.update();
        };
        select(&mut app, child);
        assert_eq!(guide(&app), tokens::TREE_GUIDE_SELECTED);

        select(&mut app, parent);
        assert_eq!(guide(&app), tokens::TREE_GUIDE);
    }
}
