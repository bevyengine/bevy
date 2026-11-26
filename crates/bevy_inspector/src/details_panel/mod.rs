//! A panel showing the components of the selected entity, and the systems that keep it in sync.

mod annotations;
mod edit;
mod rows;
mod sync;
mod walk;

#[cfg(test)]
mod tests;

pub use annotations::FieldAnnotations;
pub use sync::sync_details_panel;
pub use walk::field_entries;

pub(crate) use edit::apply_field_edit;
#[cfg(test)]
pub(crate) use sync::inspect_components;

use core::borrow::Borrow;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use bevy_color::Color;
use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    event::EntityEvent,
    hierarchy::Children,
    observer::On,
    query::Changed,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Query, ResMut},
    world::World,
};
use bevy_feathers::{
    containers::{subpane, subpane_body, subpane_header},
    controls::{FeathersScrollbar, ScrollbarGutter},
    display::caption,
    theme::ThemedText,
};
use bevy_platform::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, Scene};
use bevy_ui::{
    percent, px, AlignItems, Display, FlexDirection, Node, Overflow, PositionType, UiRect,
};
use bevy_ui_widgets::{ControlOrientation, NumericRange, NumericValue, ScrollArea, ValueChange};
use bevy_utils::prelude::DebugName;

use self::walk::read_only;
use crate::{
    column_split::ColumnSplit, entity_tree::InspectorUi, InspectableEntity, InspectorSource,
};

/// The deepest nesting level whose fields are rendered.
///
/// This bounds how many widgets one selection spawns: without a limit, deeply nested values
/// would build thousands of rows on every rebuild.
const MAX_DEPTH: usize = 4;
/// The number of items rendered for a list, array, map or set.
///
/// This bounds how many widgets one selection spawns: without a limit, a large collection such
/// as a mesh's vertex data would build thousands of rows on every rebuild.
const MAX_ITEMS: usize = 16;
/// The horizontal indent of a field row per nesting level, in logical pixels.
const INDENT: f32 = 12.0;
/// The corner radius of a component card and of the blocks inside it, in logical pixels.
const CARD_RADIUS: f32 = 5.0;

/// Marker for the scrollable column holding the component groups of the details panel.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorDetailsBody;

/// Marker for the body of a component group, which holds the field rows of that component.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorDetailsFields;

/// Identifies a component in the details panel, independently of the world it lives in.
///
/// This is the full type path of the component, so it is the same for every entity and across
/// runs. Components without a type registration use [`ComponentInfo::name`] instead, or their
/// [`ComponentId`] when the `debug` feature of `bevy_utils` is disabled and no name is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct ComponentKey(pub Arc<str>);

impl ComponentKey {
    /// The key of the component `id` in `world`.
    pub fn of(world: &World, id: ComponentId) -> Self {
        let Some(info) = world.components().get_info(id) else {
            return Self::from(format!("{id:?}").as_str());
        };
        let registered = info.type_id().and_then(|type_id| {
            let registry = world.get_resource::<AppTypeRegistry>()?.read();
            Some(Self::from(registry.get(type_id)?.type_info().type_path()))
        });
        registered.unwrap_or_else(|| {
            if DebugName::ENABLED {
                Self::from(&*ComponentInfo::name(info))
            } else {
                Self::from(format!("{id:?}").as_str())
            }
        })
    }
}

impl From<&str> for ComponentKey {
    fn from(path: &str) -> Self {
        Self(Arc::from(path))
    }
}

impl Borrow<str> for ComponentKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// The body of a block grouping the fields of a struct, which holds the rows of those fields.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorFieldGroupBody;

/// The toggle that collapses a block grouping the fields of a struct, and the body it hides.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorFieldGroupToggle(pub Entity);

/// A button of a segmented variant control, and the index of the variant it selects.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorSegment(pub usize);

/// The [`ComponentKey`] of the component a group's toggle or field container belongs to.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorDetailsComponent(pub ComponentKey);

/// The kind of widget a field value is rendered with, as returned by [`FieldValue::kind`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum FieldKind {
    /// A checkbox.
    Bool,
    /// A number input.
    Number,
    /// A text input.
    Text,
    /// A color picker and a text input holding its hexadecimal value.
    Color,
    /// A select listing the variants an enum can switch to.
    Variant,
    /// A plain text label.
    #[default]
    Label,
}

/// The value of a field, in the form the panel renders it.
///
/// [`FieldValue::kind`] gives the matching [`FieldKind`].
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// A boolean value.
    Bool(bool),
    /// A numeric value, in the format of the widget that displays it.
    Number(NumericValue),
    /// A string value.
    Text(String),
    /// A color value.
    Color(Color),
    /// The variants an enum can switch to, and the index of the current one.
    Variant {
        /// The names of the variants the enum can switch to.
        variants: Vec<String>,
        /// The index of the current variant in `variants`.
        selected: usize,
    },
    /// A read-only value with no dedicated widget, such as a container summary.
    Label(String),
}

impl FieldValue {
    /// The [`FieldKind`] of widget this value is rendered with.
    pub fn kind(&self) -> FieldKind {
        match self {
            FieldValue::Bool(_) => FieldKind::Bool,
            FieldValue::Number(_) => FieldKind::Number,
            FieldValue::Text(_) => FieldKind::Text,
            FieldValue::Color(_) => FieldKind::Color,
            FieldValue::Variant { .. } => FieldKind::Variant,
            FieldValue::Label(_) => FieldKind::Label,
        }
    }
}

/// A request to write a new value into one field of a component on an inspected entity.
///
/// The edit is applied through commands, and only when the [`InspectorSource`] is
/// [`InspectorSource::Local`]; it is ignored otherwise. [`FieldValue::Label`] values are not
/// supported. An edit that cannot be applied logs a warning and leaves the component unchanged.
#[derive(EntityEvent, Debug, Clone)]
pub struct FieldEdit {
    /// The inspected entity holding the component.
    #[event_target]
    pub entity: Entity,
    /// Whether the inspected entity is from.
    pub source: InspectorSource,
    /// The full type path of the component.
    pub component: String,
    /// The path of the field within the component, in [`bevy_reflect::GetPath`] syntax.
    pub path: String,
    /// The value to write into the field.
    pub value: FieldValue,
}

/// The component field that a details panel widget edits.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorField {
    /// The component, whose key is its full type path.
    pub component: ComponentKey,
    /// The path of the field within the component, in [`bevy_reflect::GetPath`] syntax.
    pub path: String,
}

/// A single field of a component, as one row of the details panel.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEntry {
    /// The path of the field within its component, in [`bevy_reflect::GetPath`] syntax.
    ///
    /// Map and set entries whose keys have no path syntax use an unresolvable `[#n]` placeholder.
    pub path: String,
    /// The name shown at the start of the row.
    pub label: String,
    /// The nesting level of the field, used to indent the row.
    pub depth: usize,
    /// The value of the field.
    pub value: FieldValue,
    /// The range a number input is limited to: that of an integer type narrower than the input,
    /// narrowed to the [`ValueRange`](bevy_annotations::ValueRange) annotation of the field.
    pub limit: Option<NumericRange>,
    /// The annotations of the field.
    pub annotations: FieldAnnotations,
    /// Whether the field is an enum whose variants are all unit variants.
    pub unit_enum: bool,
}

impl FieldEntry {
    /// This entry in its read-only caption form, with the unit of a number appended and the
    /// display name of a variant.
    fn into_read_only(self) -> Self {
        let value = match (self.value, self.annotations.unit) {
            (FieldValue::Number(number), Some(unit)) => {
                FieldValue::Label(format!("{number} {unit}"))
            }
            (FieldValue::Variant { variants, selected }, _) => FieldValue::Label(
                variants
                    .get(selected)
                    .map(|name| self.annotations.variant_label(name).to_string())
                    .unwrap_or_default(),
            ),
            (value, _) => read_only(value),
        };
        Self {
            value,
            limit: None,
            annotations: FieldAnnotations {
                read_only: true,
                ..self.annotations
            },
            ..self
        }
    }
}

/// The widgets rendering one field, and the value they were last given.
#[derive(Debug, Clone)]
struct FieldWidget {
    entity: Entity,
    text: Option<Entity>,
    value: FieldValue,
}

/// A component of the inspected entity, with its fields flattened into rows.
#[derive(Debug, Clone)]
pub(crate) struct ComponentDetails {
    pub(crate) key: ComponentKey,
    /// The name shown in the group header, a [`ShortName`](bevy_utils::prelude::ShortName) of the component type.
    pub(crate) name: String,
    pub(crate) memory: String,
    pub(crate) fields: Vec<FieldEntry>,
}

/// The group spawned for one component, whether its fields were spawned, and the rows they were
/// spawned for.
#[derive(Debug, Clone)]
struct GroupWidget {
    entity: Entity,
    expanded: bool,
    layout: Vec<RowLayout>,
}

/// The parts of a field row that decide which widgets it is spawned with.
#[derive(Debug, Clone, PartialEq)]
struct RowLayout {
    path: String,
    label: String,
    depth: usize,
    kind: FieldKind,
    limit: Option<NumericRange>,
    annotations: FieldAnnotations,
    unit_enum: bool,
}

impl RowLayout {
    fn new(entry: &FieldEntry) -> Self {
        Self {
            path: entry.path.clone(),
            label: entry.label.clone(),
            depth: entry.depth,
            kind: entry.value.kind(),
            limit: entry.limit.clone(),
            annotations: entry.annotations.clone(),
            unit_enum: entry.unit_enum,
        }
    }

    fn matches(&self, entry: &FieldEntry) -> bool {
        self.path == entry.path
            && self.label == entry.label
            && self.depth == entry.depth
            && self.kind == entry.value.kind()
            && self.limit == entry.limit
            && self.annotations == entry.annotations
            && self.unit_enum == entry.unit_enum
    }
}

/// Why the details panel shows a message instead of component groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmptyState {
    NoSelection,
    Despawned,
    NoComponents,
}

impl EmptyState {
    fn message(self) -> &'static str {
        match self {
            EmptyState::NoSelection => "No entity selected",
            EmptyState::Despawned => "The selected entity no longer exists",
            EmptyState::NoComponents => "The selected entity has no components",
        }
    }
}

/// Maps the components and fields shown by the details panel to the widgets displaying them.
#[derive(Resource, Debug, Default)]
pub struct DetailsIndex {
    fields: HashMap<(ComponentKey, String), FieldWidget>,
    groups: HashMap<ComponentKey, GroupWidget>,
    body: Option<Entity>,
    selection: Option<InspectableEntity>,
    empty: Option<EmptyState>,
}

impl DetailsIndex {
    /// The widget displaying the field at `path` of the component with the type path
    /// `component`, if one exists.
    pub fn widget(&self, component: &str, path: &str) -> Option<Entity> {
        self.fields
            .get(&(ComponentKey::from(component), path.to_string()))
            .map(|widget| widget.entity)
    }

    /// The group entity of the component with the type path `component`, if one is shown.
    pub(crate) fn group(&self, component: &str) -> Option<Entity> {
        self.groups.get(component).map(|group| group.entity)
    }

    /// The number of fields currently tracked.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether no fields are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Which components have their field lists collapsed.
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DetailsCollapsed {
    /// Whether a component starts collapsed.
    pub by_default: bool,
    /// The components toggled away from [`by_default`](Self::by_default).
    pub toggled: HashSet<ComponentKey>,
}

impl DetailsCollapsed {
    /// Returns `true` if the field list of the component is collapsed.
    pub fn is_collapsed(&self, key: &ComponentKey) -> bool {
        self.by_default != self.toggled.contains(key)
    }
}

/// The [`ColumnSplit::fraction`] of each component's field rows, kept while groups respawn.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DetailsColumnSplits(pub HashMap<ComponentKey, f32>);

/// Records the column split of each component group in [`DetailsColumnSplits`].
pub fn store_column_splits(
    splits: Query<(&ColumnSplit, &InspectorDetailsComponent), Changed<ColumnSplit>>,
    mut stored: ResMut<DetailsColumnSplits>,
) {
    for (split, component) in &splits {
        stored.0.insert(component.0.clone(), split.fraction);
    }
}

/// Pacing of the details panel synchronization pass.
pub type DetailsPanelSync = crate::PanelSync<InspectorDetailsBody>;

/// A panel showing the components of the selected entity, rendered from reflection.
pub fn details_panel() -> impl Scene {
    bsn! {
        InspectorUi
        @subpane()
        Node {
            width: px(320),
            max_height: percent(100),
        }
        Children [
            @subpane_header() Children [
                @caption("Details")
            ]
            --
            @details_body()
        ]
    }
}

/// The scrolling contents of [`details_panel`], without its header.
pub fn details_body() -> impl Scene {
    bsn! {
        @subpane_body()
        Node {
            padding: UiRect {
                top: px(6),
                left: px(6),
                right: px(14),
                bottom: px(6),
            },
            min_height: px(0),
        }
        ScrollbarGutter(px(14))
        Children [
            #inner
            InspectorDetailsBody
            ThemedText
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
            }
            ScrollArea
            --
            @FeathersScrollbar {
                @target: #inner,
                @orientation: {ControlOrientation::Vertical}
            }
            Node {
                position_type: PositionType::Absolute,
                right: px(4),
                top: px(0),
                bottom: px(0),
                width: px(4),
            }
        ]
    }
}

/// Observer that records whether a component group is collapsed when its toggle changes.
pub fn inspector_details_toggled(
    change: On<ValueChange<bool>>,
    toggles: Query<&InspectorDetailsComponent>,
    mut collapsed: ResMut<DetailsCollapsed>,
    mut sync: ResMut<DetailsPanelSync>,
) {
    let Ok(component) = toggles.get(change.source) else {
        return;
    };
    if change.value == collapsed.by_default {
        collapsed.toggled.insert(component.0.clone());
    } else {
        collapsed.toggled.remove(&component.0);
    }
    sync.set_dirty();
}
