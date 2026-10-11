//! A panel showing the components of the selected entity, and the systems that keep it in sync.

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};

use bevy_color::{Color, LinearRgba, Srgba};
use bevy_dev_tools::inspection::{
    component_inspection::{ComponentDetailLevel, ComponentInspectionSettings},
    entity_inspection::EntityInspectionSettings,
    extension_methods::WorldInspectionExtensionTrait,
};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    name::Name,
    observer::On,
    query::{Changed, With},
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, ResMut},
    world::World,
};
use bevy_feathers::{
    containers::{group, group_body, group_header, subpane, subpane_body, subpane_header},
    controls::{
        list_rows_from_strings, ColorSwatchValue, FeathersCheckbox, FeathersColorSwatch,
        FeathersDisclosureToggle, FeathersNumberInput, FeathersScrollbar, FeathersSelect,
        FeathersTextInput, FeathersTextInputContainer, HardLimit, OptionIndex, ScrollbarGutter,
    },
    display::caption,
    theme::ThemedText,
};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{
    enums::{DynamicEnum, DynamicVariant, VariantType},
    prelude::ReflectDefault,
    GetPath, PartialReflect, Reflect, ReflectFromReflect, ReflectRef, TypeInfo, TypeRegistry,
};
use bevy_scene::{bsn, on, Scene, WorldSceneExt};
use bevy_text::{EditableText, LineBreak, TextEdit, TextEditChange, TextLayout};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{
    percent, px, widget::Text, AlignItems, Checked, Display, FlexDirection, Node, Overflow,
    PositionType, UiRect,
};
use bevy_ui_widgets::{ControlOrientation, NumericRange, NumericValue, ScrollArea, ValueChange};
use bevy_utils::prelude::ShortName;

use crate::{
    _InspectorSelection,
    column_split::{ColumnSplit, ColumnSplitLeading},
    entity_tree::InspectorUi,
    InspectorSelection, InspectorSource,
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
/// The width of the widget editing a field value, in logical pixels.
const FIELD_WIDGET_WIDTH: f32 = 160.0;

/// Marker for the scrollable column holding the component groups of the details panel.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorDetailsBody;

/// Marker for the body of a component group, which holds the field rows of that component.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorDetailsFields;

/// The [`ComponentId`] of the component a group's toggle or field container belongs to.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorDetailsComponent(pub ComponentId);

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
    /// A color swatch and its hexadecimal value.
    Color,
    /// A select listing the variants of a unit-only enum.
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
    /// The variants of a unit-only enum, and the index of the current one.
    Variant {
        /// The names of every variant of the enum.
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
/// [`InspectorSource::Local`]; it is ignored otherwise. [`FieldValue::Color`] and
/// [`FieldValue::Label`] values are not supported. An edit that cannot be applied logs a warning
/// and leaves the component unchanged.
#[derive(EntityEvent, Debug, Clone)]
pub struct FieldEdit {
    /// The inspected entity holding the component.
    #[event_target]
    pub entity: Entity,
    /// Whether the inspected entity is a main entity or render entity.
    pub is_main: bool,
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
    /// The component, as used to key [`DetailsIndex`].
    pub component: ComponentId,
    /// The full type path of the component.
    pub type_path: String,
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
    /// The range of an integer field whose type is narrower than its number input.
    pub limit: Option<NumericRange>,
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
    pub(crate) id: ComponentId,
    /// The name shown in the group header, a [`ShortName`] of the component type.
    pub(crate) name: String,
    pub(crate) type_path: String,
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
}

impl RowLayout {
    fn new(entry: &FieldEntry) -> Self {
        Self {
            path: entry.path.clone(),
            label: entry.label.clone(),
            depth: entry.depth,
            kind: entry.value.kind(),
            limit: entry.limit.clone(),
        }
    }

    fn matches(&self, entry: &FieldEntry) -> bool {
        self.path == entry.path
            && self.label == entry.label
            && self.depth == entry.depth
            && self.kind == entry.value.kind()
            && self.limit == entry.limit
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
    fields: HashMap<(ComponentId, String), FieldWidget>,
    groups: HashMap<ComponentId, GroupWidget>,
    body: Option<Entity>,
    selection: Option<_InspectorSelection>,
    empty: Option<EmptyState>,
}

impl DetailsIndex {
    /// The widget displaying the field at `path` of `component`, if one exists.
    pub fn widget(&self, component: ComponentId, path: &str) -> Option<Entity> {
        self.fields
            .get(&(component, path.to_string()))
            .map(|widget| widget.entity)
    }

    /// The group entity of `component`, if one is shown.
    pub(crate) fn group(&self, component: ComponentId) -> Option<Entity> {
        self.groups.get(&component).map(|group| group.entity)
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

/// The components whose field lists are collapsed.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DetailsCollapsed(pub HashSet<ComponentId>);

/// The [`ColumnSplit::fraction`] of each component's field rows, kept while groups respawn.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DetailsColumnSplits(pub HashMap<ComponentId, f32>);

/// Records the column split of each component group in [`DetailsColumnSplits`].
pub fn store_column_splits(
    splits: Query<(&ColumnSplit, &InspectorDetailsComponent), Changed<ColumnSplit>>,
    mut stored: ResMut<DetailsColumnSplits>,
) {
    for (split, component) in &splits {
        stored.0.insert(component.0, split.fraction);
    }
}

/// Pacing of the details panel synchronization pass.
#[derive(Resource, Debug)]
pub struct DetailsPanelSync {
    /// Time between synchronization passes.
    pub timer: Timer,
}

impl Default for DetailsPanelSync {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.25, TimerMode::Repeating);
        timer.set_elapsed(timer.duration());
        Self { timer }
    }
}

impl DetailsPanelSync {
    /// Forces a synchronization pass on the next tick.
    pub fn set_dirty(&mut self) {
        self.timer.almost_finish();
    }
}

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
                    width: px(6),
                }
            ]
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
    if change.value {
        collapsed.0.remove(&component.0);
    } else {
        collapsed.0.insert(component.0);
    }
    sync.set_dirty();
}

/// Records a widget's new value, writes it back into the widget and emits a [`FieldEdit`] for the
/// entity the panel was built for.
///
/// `value` maps the value the widget last showed to the new one, or returns `None` to skip the
/// change. The feathers controls do not update themselves when the user changes them, so the
/// widget only shows the new value once it is written back.
fn emit_field_edit(
    widget: Entity,
    value: impl FnOnce(&FieldValue) -> Option<FieldValue>,
    fields: &Query<&InspectorField>,
    index: &mut DetailsIndex,
    commands: &mut Commands,
) {
    let Ok(field) = fields.get(widget) else {
        return;
    };
    let Some(selection) = index.selection else {
        return;
    };
    let Some(stored) = index.fields.get_mut(&(field.component, field.path.clone())) else {
        return;
    };
    let Some(value) = value(&stored.value) else {
        return;
    };

    stored.value = value.clone();
    let displayed = stored.clone();
    commands.queue(move |world: &mut World| apply_value(world, &displayed, &displayed.value));
    commands.trigger(FieldEdit {
        entity: selection.entity,
        is_main: selection.is_main,
        component: field.type_path.clone(),
        path: field.path.clone(),
        value,
    });
}

/// Observer that turns a details panel checkbox change into a [`FieldEdit`].
pub(crate) fn inspector_field_bool_changed(
    change: On<ValueChange<bool>>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let value = FieldValue::Bool(change.value);
    emit_field_edit(
        change.source,
        |_| Some(value),
        &fields,
        &mut index,
        &mut commands,
    );
}

macro_rules! number_changed_observer {
    ($name:ident, $number:ty, $variant:ident) => {
        /// Observer that turns a details panel number input change into a [`FieldEdit`].
        pub(crate) fn $name(
            change: On<ValueChange<$number>>,
            fields: Query<&InspectorField>,
            mut index: ResMut<DetailsIndex>,
            mut commands: Commands,
        ) {
            let value = FieldValue::Number(NumericValue::$variant(change.value));
            emit_field_edit(
                change.source,
                |_| Some(value),
                &fields,
                &mut index,
                &mut commands,
            );
        }
    };
}

number_changed_observer!(inspector_field_f32_changed, f32, F32);
number_changed_observer!(inspector_field_f64_changed, f64, F64);
number_changed_observer!(inspector_field_i32_changed, i32, I32);
number_changed_observer!(inspector_field_i64_changed, i64, I64);

/// Observer that turns a details panel text input edit into a [`FieldEdit`].
///
/// Text equal to the value the input last showed is skipped, since it is the panel writing that
/// value into the input.
pub(crate) fn inspector_field_text_changed(
    change: On<TextEditChange>,
    texts: Query<&EditableText>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let widget = change.event_target();
    let Ok(editable) = texts.get(widget) else {
        return;
    };
    let text = editable.value().to_string();
    emit_field_edit(
        widget,
        |current| match current {
            FieldValue::Text(current) if *current == text => None,
            _ => Some(FieldValue::Text(text)),
        },
        &fields,
        &mut index,
        &mut commands,
    );
}

/// Observer that turns a details panel variant selection into a [`FieldEdit`].
pub(crate) fn inspector_field_variant_changed(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let Ok(option) = options.get(change.value) else {
        return;
    };
    let selected = option.0;
    emit_field_edit(
        change.source,
        |current| match current {
            FieldValue::Variant { variants, .. } => Some(FieldValue::Variant {
                variants: variants.clone(),
                selected,
            }),
            _ => None,
        },
        &fields,
        &mut index,
        &mut commands,
    );
}

/// Observer that writes a [`FieldEdit`] into the component of the inspected entity.
pub(crate) fn apply_field_edit(edit: On<FieldEdit>, mut commands: Commands) {
    let edit = edit.event().clone();
    commands.queue(move |world: &mut World| write_field_edit(world, &edit));
}

/// Writes `edit` into its component, marking the component changed only if its value changes.
///
/// The details panel is refreshed on the next tick whenever the field does not end up holding the
/// edited value, so that its widget is reverted.
fn write_field_edit(world: &mut World, edit: &FieldEdit) {
    if world.get_resource::<InspectorSource>() != Some(&InspectorSource::Local) {
        return;
    }
    let Some(registry) = world.get_resource::<AppTypeRegistry>().cloned() else {
        return;
    };
    let registry = registry.read();
    let Some((registration, reflect_component)) = registry
        .get_with_type_path(&edit.component)
        .and_then(|registration| Some((registration, registration.data::<ReflectComponent>()?)))
    else {
        warn!("the inspector cannot edit `{}`", edit.component);
        return;
    };
    if world.get_entity(edit.entity).is_err() {
        return;
    }
    let mutable = world
        .components()
        .get_id(registration.type_id())
        .and_then(|id| world.components().get_info(id))
        .map(ComponentInfo::mutable);
    if mutable == Some(false) {
        warn!("`{}` is immutable and cannot be edited", edit.component);
        return;
    }

    let component =
        mutable.and_then(|_| reflect_component.reflect_mut(world.entity_mut(edit.entity)));
    let Some(mut component) = component else {
        warn!("`{}` is no longer on the entity", edit.component);
        refresh_details_panel(world);
        return;
    };

    let target = component.bypass_change_detection();
    let field = if edit.path.is_empty() {
        Ok(target.as_partial_reflect_mut())
    } else {
        target.reflect_path_mut(edit.path.as_str())
    };
    let (write, held, typing_a_char) = match field {
        Ok(field) => {
            let write = write_field_value(field, &edit.value);
            let held = write != FieldWrite::Rejected && holds_value(field, &edit.value);
            let typing_a_char = matches!(edit.value, FieldValue::Text(_))
                && field.try_downcast_ref::<char>().is_some();
            (write, held, typing_a_char)
        }
        Err(_) => (FieldWrite::Rejected, false, false),
    };
    if write == FieldWrite::Written {
        component.set_changed();
    }
    if write == FieldWrite::Rejected && !typing_a_char {
        warn!(
            "the inspector cannot edit `{}` at `{}`",
            edit.component, edit.path
        );
    }
    if !held {
        refresh_details_panel(world);
    }
}

/// Forces a details panel synchronization pass on the next tick, if the panel is set up.
fn refresh_details_panel(world: &mut World) {
    if let Some(mut sync) = world.get_resource_mut::<DetailsPanelSync>() {
        sync.set_dirty();
    }
}

/// Whether `field` holds `value`, so that a widget showing `value` is up to date.
fn holds_value(field: &dyn PartialReflect, value: &FieldValue) -> bool {
    scalar_value(field).is_some_and(|current| same_value(&current, value))
}

/// Whether two field values are equal, comparing floats bit for bit so that a NaN equals itself.
fn same_value(left: &FieldValue, right: &FieldValue) -> bool {
    use FieldValue::Number;
    match (left, right) {
        (Number(NumericValue::F32(left)), Number(NumericValue::F32(right))) => {
            left.to_bits() == right.to_bits()
        }
        (Number(NumericValue::F64(left)), Number(NumericValue::F64(right))) => {
            left.to_bits() == right.to_bits()
        }
        _ => left == right,
    }
}

/// Describes the outcome of a field write attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldWrite {
    /// The field now holds a new value.
    Written,
    /// The field already held the value, so nothing was written.
    Unchanged,
    /// The value cannot be written into the field.
    Rejected,
}

/// Assigns `value` to `target`, reporting the change based on previous data.
fn assign<T: PartialEq>(target: &mut T, value: T) -> FieldWrite {
    if *target == value {
        FieldWrite::Unchanged
    } else {
        *target = value;
        FieldWrite::Written
    }
}

fn write_field_value(field: &mut dyn PartialReflect, value: &FieldValue) -> FieldWrite {
    match value {
        FieldValue::Bool(new) => match field.try_downcast_mut::<bool>() {
            Some(target) => assign(target, *new),
            None => FieldWrite::Rejected,
        },
        FieldValue::Number(number) => write_number_field(field, *number),
        FieldValue::Text(text) => write_text_field(field, text),
        FieldValue::Variant { variants, selected } => match variants.get(*selected) {
            Some(name) => write_variant_field(field, name),
            None => FieldWrite::Rejected,
        },
        FieldValue::Color(_) | FieldValue::Label(_) => FieldWrite::Rejected,
    }
}

/// Writes text into a string or `char` field, allocating only when the text differs.
///
/// A `char` field takes the last character of the text, so that typing replaces it.
fn write_text_field(field: &mut dyn PartialReflect, text: &str) -> FieldWrite {
    if let Some(target) = field.try_downcast_mut::<String>() {
        if target == text {
            return FieldWrite::Unchanged;
        }
        *target = text.to_string();
        return FieldWrite::Written;
    }
    if let Some(target) = field.try_downcast_mut::<Cow<'static, str>>() {
        if target == text {
            return FieldWrite::Unchanged;
        }
        *target = Cow::Owned(text.to_string());
        return FieldWrite::Written;
    }
    if let Some(target) = field.try_downcast_mut::<char>()
        && let Some(last) = text.chars().last()
    {
        return assign(target, last);
    }
    FieldWrite::Rejected
}

/// Writes a number into a numeric field, rejecting non-finite values and integers outside the
/// field's range.
///
/// Floats are compared bit for bit, so that writing `-0.0` over `0.0` counts as a change.
fn write_number_field(field: &mut dyn PartialReflect, value: NumericValue) -> FieldWrite {
    let float = match value {
        NumericValue::F32(value) => value as f64,
        NumericValue::F64(value) => value,
        NumericValue::I32(value) => value as f64,
        NumericValue::I64(value) => value as f64,
    };
    if !float.is_finite() {
        return FieldWrite::Rejected;
    }
    if let Some(target) = field.try_downcast_mut::<f32>() {
        let narrowed = match value {
            NumericValue::F32(value) => value,
            _ => float as f32,
        };
        if !narrowed.is_finite() {
            return FieldWrite::Rejected;
        }
        return assign_bits(target, narrowed, f32::to_bits);
    }
    if let Some(target) = field.try_downcast_mut::<f64>() {
        return assign_bits(target, float, f64::to_bits);
    }

    let integer = match value {
        NumericValue::I32(value) => i128::from(value),
        NumericValue::I64(value) => i128::from(value),
        _ if float.fract() == 0.0 => float as i128,
        _ => return FieldWrite::Rejected,
    };

    if let Some(target) = field.try_downcast_mut::<i128>() {
        return assign(target, integer);
    }

    macro_rules! write_as {
        ($($type:ty),*) => {
            $(
                if let Some(target) = field.try_downcast_mut::<$type>() {
                    return match <$type>::try_from(integer) {
                        Ok(value) => assign(target, value),
                        Err(_) => FieldWrite::Rejected,
                    };
                }
            )*
        };
    }

    write_as!(i8, i16, i32, i64, isize, u8, u16, u32, u64, u128, usize);
    FieldWrite::Rejected
}

/// Assigns a float to `target`, comparing the `bits` of both values rather than using `PartialEq`.
fn assign_bits<T: Copy, B: PartialEq>(target: &mut T, value: T, bits: fn(T) -> B) -> FieldWrite {
    if bits(*target) == bits(value) {
        FieldWrite::Unchanged
    } else {
        *target = value;
        FieldWrite::Written
    }
}

/// Switches a unit-only enum field to the variant `name`.
fn write_variant_field(field: &mut dyn PartialReflect, name: &str) -> FieldWrite {
    let ReflectRef::Enum(current) = field.reflect_ref() else {
        return FieldWrite::Rejected;
    };
    if current.variant_name() == name {
        return FieldWrite::Unchanged;
    }
    let Some(TypeInfo::Enum(info)) = field.get_represented_type_info() else {
        return FieldWrite::Rejected;
    };
    if info.variant(name).is_none()
        || info
            .iter()
            .any(|variant| variant.variant_type() != VariantType::Unit)
    {
        return FieldWrite::Rejected;
    }
    match field.try_apply(&DynamicEnum::new(name, DynamicVariant::Unit)) {
        Ok(()) => FieldWrite::Written,
        Err(_) => FieldWrite::Rejected,
    }
}

/// Rebuilds or refreshes the details panel so that it matches the selected entity.
///
/// Only the groups of components that were added, removed, collapsed or expanded are respawned.
/// The whole panel is rebuilt when the selection changes.
pub fn sync_details_panel(world: &mut World) {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();

    let run = world
        .resource_mut::<DetailsPanelSync>()
        .timer
        .tick(delta)
        .just_finished();

    let selection = world.resource::<InspectorSelection>().0;
    let selection_changed = world.resource::<DetailsIndex>().selection != selection;
    if !run && !selection_changed {
        return;
    }

    let Some(body) = find_body(world) else {
        return;
    };

    let is_main = selection.map_or(true, |s| s.is_main);
    let maybe_selected = selection.map_or(None, |s| Some(s.entity));

    let inspected = crate::world_to_inspect(world, is_main);
    let components = inspect_components(inspected, selection);
    let empty = empty_state(inspected, maybe_selected, &components);

    let index = world.resource::<DetailsIndex>();
    if selection_changed || index.body != Some(body) || index.empty != empty {
        reset_body(world, body, selection, empty);
    }
    if empty.is_none() {
        sync_groups(world, body, &components);
    }
}

/// The body of the details panel, found through its [`InspectorDetailsBody`] marker.
fn find_body(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<InspectorDetailsBody>>()
        .iter(world)
        .next()
}

/// The component groups of `selection` in the inspected `world`, sorted in display order.
pub(crate) fn inspect_components(
    world: &World,
    selection: Option<_InspectorSelection>,
) -> Vec<ComponentDetails> {
    let Some(selection) = selection else {
        return Vec::new();
    };

    let settings = EntityInspectionSettings {
        include_components: true,
        component_settings: ComponentInspectionSettings {
            store_reflected_value: true,
            detail_level: ComponentDetailLevel::Names,
            ..Default::default()
        },
    };

    let Ok(inspection) = world.inspect(selection.entity, settings) else {
        return Vec::new();
    };
    let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
        return Vec::new();
    };
    let registry = registry.read();
    let local = world.get_resource::<InspectorSource>() == Some(&InspectorSource::Local);

    let mut components: Vec<ComponentDetails> = inspection
        .components
        .unwrap_or_default()
        .iter()
        .map(|component| {
            let mut fields = component
                .reflected_value
                .as_deref()
                .map(|value| field_entries(value, &registry))
                .unwrap_or_default();
            let mutable = world
                .components()
                .get_info(component.component_id)
                .is_some_and(ComponentInfo::mutable);
            if !(local && mutable) {
                fields = fields
                    .into_iter()
                    .map(|entry| FieldEntry {
                        value: read_only(entry.value),
                        limit: None,
                        ..entry
                    })
                    .collect();
            }
            ComponentDetails {
                id: component.component_id,
                name: crate::component_short_name(world, component.component_id),
                type_path: component
                    .reflected_value
                    .as_deref()
                    .and_then(PartialReflect::get_represented_type_info)
                    .map(|info| info.type_path().to_string())
                    .unwrap_or_default(),
                memory: component.memory_size.to_string(),
                fields,
            }
        })
        .collect();
    #[cfg(feature = "remote")]
    {
        let inspected = crate::world_to_inspect(world, selection.is_main);
        if let Some(record) = inspected.get::<crate::remote::RemoteComponents>(selection.entity) {
            return crate::remote::details::annotate(inspected, record, components);
        }
    }
    components.sort_by(|left, right| (&left.name, left.id).cmp(&(&right.name, right.id)));
    components
}

/// The message to show instead of component groups, or `None` if there are groups to show.
fn empty_state(
    world: &World,
    selection: Option<Entity>,
    components: &[ComponentDetails],
) -> Option<EmptyState> {
    match selection {
        None => Some(EmptyState::NoSelection),
        Some(entity) if world.get_entity(entity).is_err() => Some(EmptyState::Despawned),
        Some(_) if components.is_empty() => Some(EmptyState::NoComponents),
        Some(_) => None,
    }
}

/// Clears the panel body, showing the message for `empty` if there is one.
fn reset_body(
    world: &mut World,
    body: Entity,
    selection: Option<_InspectorSelection>,
    empty: Option<EmptyState>,
) {
    let children: Vec<Entity> = world
        .get::<Children>(body)
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();
    for child in children {
        if let Ok(child) = world.get_entity_mut(child) {
            child.despawn();
        }
    }

    if let Some(empty) = empty {
        spawn_child_scene(world, body, caption(empty.message()));
    }

    *world.resource_mut::<DetailsIndex>() = DetailsIndex {
        body: Some(body),
        selection,
        empty,
        ..Default::default()
    };
}

/// Brings the component groups in line with `components`, which is sorted in display order.
///
/// Groups of removed components, and groups whose collapsed state or field layout changed, are
/// respawned or despawned. The remaining groups have their widget values updated in place.
fn sync_groups(world: &mut World, body: Entity, components: &[ComponentDetails]) {
    let collapsed = world.resource::<DetailsCollapsed>().0.clone();
    let present: HashSet<ComponentId> = components.iter().map(|component| component.id).collect();
    let stale: Vec<ComponentId> = world
        .resource::<DetailsIndex>()
        .groups
        .iter()
        .filter(|(id, group)| !present.contains(*id) || group.expanded == collapsed.contains(*id))
        .map(|(id, _)| *id)
        .collect();

    let mut reordered = !stale.is_empty();
    for id in stale {
        despawn_group(world, id);
    }

    for component in components {
        if world
            .resource::<DetailsIndex>()
            .groups
            .contains_key(&component.id)
        {
            if update_in_place(world, component) {
                continue;
            }
            despawn_group(world, component.id);
        }
        let expanded = !collapsed.contains(&component.id);
        spawn_group(world, body, component, expanded);
        reordered = true;
    }

    if reordered {
        order_groups(world, body, components);
    }
}

/// Writes the current field values of `component` into its existing widgets.
///
/// Returns `false`, leaving every widget untouched, if rows were added, removed, relabeled or
/// changed kind, and the group must be respawned instead.
fn update_in_place(world: &mut World, component: &ComponentDetails) -> bool {
    let mut updates = Vec::new();
    {
        let index = world.resource::<DetailsIndex>();
        let Some(group) = index.groups.get(&component.id) else {
            return false;
        };
        if !group.expanded {
            return true;
        }
        if group.layout.len() != component.fields.len()
            || !group
                .layout
                .iter()
                .zip(&component.fields)
                .all(|(row, entry)| row.matches(entry))
        {
            return false;
        }
        for entry in &component.fields {
            let key = (component.id, entry.path.clone());
            let Some(widget) = index.fields.get(&key) else {
                return false;
            };
            if same_value(&widget.value, &entry.value) {
                continue;
            }
            if matches!(entry.value, FieldValue::Variant { .. }) {
                return false;
            }
            updates.push((key, widget.clone(), entry.value.clone()));
        }
    }

    for (key, widget, value) in updates {
        apply_value(world, &widget, &value);
        if let Some(stored) = world.resource_mut::<DetailsIndex>().fields.get_mut(&key) {
            stored.value = value;
        }
    }
    true
}

/// Despawns the group of `component` and forgets its field widgets.
fn despawn_group(world: &mut World, component: ComponentId) {
    let mut index = world.resource_mut::<DetailsIndex>();
    let group = index.groups.remove(&component);
    index.fields.retain(|(id, _), _| *id != component);
    if let Some(group) = group
        && let Ok(entity) = world.get_entity_mut(group.entity)
    {
        entity.despawn();
    }
}

/// Sorts the children of the panel body so that the groups follow the order of `components`.
fn order_groups(world: &mut World, body: Entity, components: &[ComponentDetails]) {
    let index = world.resource::<DetailsIndex>();
    let order: HashMap<Entity, usize> = components
        .iter()
        .enumerate()
        .filter_map(|(position, component)| Some((index.group(component.id)?, position)))
        .collect();
    if let Some(mut children) = world.get_mut::<Children>(body) {
        children.sort_by_key(|child| order.get(child).copied().unwrap_or(usize::MAX));
    }
}

/// Spawns the group of `component` at the end of the panel body, with its field rows if
/// `expanded` is set.
fn spawn_group(world: &mut World, body: Entity, component: &ComponentDetails, expanded: bool) {
    let Some(group) = spawn_child_scene(
        world,
        body,
        component_group(component.name.clone(), component.memory.clone()),
    ) else {
        return;
    };
    world.resource_mut::<DetailsIndex>().groups.insert(
        component.id,
        GroupWidget {
            entity: group,
            expanded,
            layout: component.fields.iter().map(RowLayout::new).collect(),
        },
    );

    if let Some(toggle) = descendant_with::<FeathersDisclosureToggle>(world, group) {
        let mut toggle = world.entity_mut(toggle);
        toggle.insert(InspectorDetailsComponent(component.id));
        if expanded {
            toggle.insert(Checked);
        }
    }

    if !expanded {
        return;
    }

    let Some(container) = descendant_with::<InspectorDetailsFields>(world, group) else {
        return;
    };
    if !component.fields.is_empty() {
        let mut split = ColumnSplit::default();
        if let Some(&fraction) = world.resource::<DetailsColumnSplits>().0.get(&component.id) {
            split.fraction = fraction;
        }
        world
            .entity_mut(container)
            .insert((InspectorDetailsComponent(component.id), split));
    }

    for entry in &component.fields {
        if let Some(widget) = spawn_field_row(world, container, component, entry) {
            world
                .resource_mut::<DetailsIndex>()
                .fields
                .insert((component.id, entry.path.clone()), widget);
        }
    }
}

fn component_group(name: String, memory: String) -> impl Scene {
    bsn! {
        InspectorUi
        @group()
        Children [
            @group_header() Children [
                @FeathersDisclosureToggle
                on(inspector_details_toggled)
                --
                @caption(name)
                --
                @caption(memory)
            ]
            --
            @group_body()
            InspectorDetailsFields
        ]
    }
}

fn spawn_field_row(
    world: &mut World,
    container: Entity,
    component: &ComponentDetails,
    entry: &FieldEntry,
) -> Option<FieldWidget> {
    let row = world
        .spawn((
            InspectorUi,
            ThemedText,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::left(px(entry.depth as f32 * INDENT)),
                min_width: px(0),
                flex_shrink: 1.0,
                overflow: Overflow::clip_x(),
                ..Default::default()
            },
            ChildOf(container),
        ))
        .id();

    let leading = world
        .spawn((
            ColumnSplitLeading,
            ThemedText,
            Node {
                flex_shrink: 0.0,
                overflow: Overflow::clip_x(),
                ..Default::default()
            },
            ChildOf(row),
        ))
        .id();
    let label = spawn_child_scene(world, leading, caption(entry.label.clone()))?;
    world.entity_mut(label).insert(TextLayout {
        linebreak: LineBreak::NoWrap,
        ..Default::default()
    });

    let widget = spawn_widget(world, row, entry)?;
    if !matches!(entry.value, FieldValue::Variant { .. }) {
        apply_value(world, &widget, &entry.value);
    }

    let input = match entry.value {
        FieldValue::Bool(_)
        | FieldValue::Number(_)
        | FieldValue::Text(_)
        | FieldValue::Variant { .. } => Some(widget.entity),
        FieldValue::Color(_) | FieldValue::Label(_) => None,
    };
    if let Some(input) = input
        && let Ok(mut entity) = world.get_entity_mut(input)
    {
        entity.insert(InspectorField {
            component: component.id,
            type_path: component.type_path.clone(),
            path: entry.path.clone(),
        });
    }

    Some(widget)
}

fn spawn_widget(world: &mut World, row: Entity, entry: &FieldEntry) -> Option<FieldWidget> {
    let value = &entry.value;
    let widget = match value {
        FieldValue::Bool(_) => {
            let entity = spawn_child_scene(
                world,
                row,
                bsn! {
                    InspectorUi
                    @FeathersCheckbox
                    on(inspector_field_bool_changed)
                },
            )?;
            FieldWidget {
                entity,
                text: None,
                value: value.clone(),
            }
        }
        FieldValue::Number(_) => {
            let entity = spawn_child_scene(
                world,
                row,
                bsn! {
                    InspectorUi
                    @FeathersNumberInput
                    on(inspector_field_f32_changed)
                    on(inspector_field_f64_changed)
                    on(inspector_field_i32_changed)
                    on(inspector_field_i64_changed)
                    Node {
                        width: px(FIELD_WIDGET_WIDTH),
                        flex_grow: 0.0,
                    }
                },
            )?;
            if let Some(limit) = &entry.limit {
                world.entity_mut(entity).insert(HardLimit(limit.clone()));
            }
            FieldWidget {
                entity,
                text: None,
                value: value.clone(),
            }
        }
        FieldValue::Text(_) => {
            let entity = spawn_text_input(world, row, FIELD_WIDGET_WIDTH)?;
            FieldWidget {
                entity,
                text: None,
                value: value.clone(),
            }
        }
        FieldValue::Color(_) => {
            let cell = world
                .spawn((
                    InspectorUi,
                    ThemedText,
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        width: px(FIELD_WIDGET_WIDTH),
                        ..Default::default()
                    },
                    ChildOf(row),
                ))
                .id();
            let entity = spawn_child_scene(
                world,
                cell,
                bsn! {
                    InspectorUi
                    @FeathersColorSwatch
                    Node {
                        width: px(24),
                        height: px(14),
                        flex_grow: 0.0,
                    }
                },
            )?;
            let text = spawn_value_caption(world, cell, String::new())?;
            FieldWidget {
                entity,
                text: Some(text),
                value: value.clone(),
            }
        }
        FieldValue::Variant {
            variants,
            selected: index,
        } => {
            let options = list_rows_from_strings(variants.clone(), Some(*index));
            let entity = spawn_child_scene(
                world,
                row,
                bsn! {
                    InspectorUi
                    @FeathersSelect {
                        @options: {options},
                    }
                    on(inspector_field_variant_changed)
                    Node {
                        width: px(FIELD_WIDGET_WIDTH),
                        flex_grow: 0.0,
                    }
                },
            )?;
            FieldWidget {
                entity,
                text: None,
                value: value.clone(),
            }
        }
        FieldValue::Label(text) => {
            let entity = spawn_value_caption(world, row, text.clone())?;
            FieldWidget {
                entity,
                text: None,
                value: value.clone(),
            }
        }
    };
    Some(widget)
}

/// Spawns a text input of the given width, returning the entity holding its [`EditableText`].
fn spawn_text_input(world: &mut World, row: Entity, width: f32) -> Option<Entity> {
    let container = spawn_child_scene(
        world,
        row,
        bsn! {
            InspectorUi
            @FeathersTextInputContainer
            Node {
                width: px(width),
                flex_grow: 0.0,
            }
            Children [
                @FeathersTextInput
                on(inspector_field_text_changed)
            ]
        },
    )?;
    descendant_with::<EditableText>(world, container)
}

/// Spawns the caption showing a field value, wrapped so that long values stay inside the panel.
fn spawn_value_caption(world: &mut World, row: Entity, text: String) -> Option<Entity> {
    let entity = spawn_child_scene(world, row, caption(text))?;
    world.entity_mut(entity).insert((
        TextLayout {
            linebreak: LineBreak::AnyCharacter,
            ..Default::default()
        },
        Node {
            min_width: px(0),
            max_width: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            ..Default::default()
        },
    ));
    Some(entity)
}

/// Shows `value` in the widgets of `widget`.
///
/// Variant selects are left untouched, since a changed variant respawns its group instead.
fn apply_value(world: &mut World, widget: &FieldWidget, value: &FieldValue) {
    match value {
        FieldValue::Bool(checked) => {
            let Ok(mut entity) = world.get_entity_mut(widget.entity) else {
                return;
            };
            if *checked {
                entity.insert(Checked);
            } else {
                entity.remove::<Checked>();
            }
        }
        FieldValue::Number(number) => {
            if let Ok(mut entity) = world.get_entity_mut(widget.entity) {
                entity.insert(*number);
            }
        }
        FieldValue::Text(text) => replace_text(world, Some(widget.entity), text),
        FieldValue::Color(color) => {
            if let Ok(mut entity) = world.get_entity_mut(widget.entity) {
                entity.insert(ColorSwatchValue(*color));
            }
            set_text(world, widget.text, color_to_hex(*color));
        }
        FieldValue::Variant { .. } => {}
        FieldValue::Label(text) => set_text(world, Some(widget.entity), text.clone()),
    }
}

/// Replaces the contents of the [`EditableText`] on `entity` with `text`, unless it already holds `text`.
fn replace_text(world: &mut World, entity: Option<Entity>, text: &str) {
    if let Some(mut editable) = entity.and_then(|entity| world.get_mut::<EditableText>(entity))
        && editable.value() != text
    {
        editable.queue_edit(TextEdit::SelectAll);
        editable.queue_edit(TextEdit::Insert(text.into()));
    }
}

/// Replaces the [`Text`] of `entity` with `value`, unless it already shows `value`.
fn set_text(world: &mut World, entity: Option<Entity>, value: String) {
    let Some(mut text) = entity.and_then(|entity| world.get_mut::<Text>(entity)) else {
        return;
    };
    if text.0 == value {
        return;
    }
    text.0 = value;
}

/// The sRGB hexadecimal code of `color`, such as `#FF8000`.
fn color_to_hex(color: Color) -> String {
    Srgba::from(color).to_hex()
}

/// Spawns `scene` as the last child of `parent`, logging a warning if it fails to spawn.
fn spawn_child_scene(world: &mut World, parent: Entity, scene: impl Scene) -> Option<Entity> {
    match world.spawn_scene(scene) {
        Ok(mut entity) => {
            entity.insert(ChildOf(parent));
            Some(entity.id())
        }
        Err(error) => {
            warn!("failed to spawn an inspector details widget: {error}");
            None
        }
    }
}

/// The first strict descendant of `root` that has a `C` component.
///
/// Descendants are visited depth first in child order, so a child and its own descendants are
/// searched before the next sibling. `root` itself is never returned.
fn descendant_with<C: Component>(world: &World, root: Entity) -> Option<Entity> {
    let mut stack = alloc::vec![root];
    while let Some(entity) = stack.pop() {
        let Ok(entity_ref) = world.get_entity(entity) else {
            continue;
        };
        if entity != root && entity_ref.contains::<C>() {
            return Some(entity);
        }
        if let Some(children) = entity_ref.get::<Children>() {
            stack.extend(children.iter().rev().copied());
        }
    }
    None
}

/// Flattens a reflected component value into the rows the details panel renders.
///
/// `registry` rebuilds dynamic values into their concrete types so that their fields can be read.
pub fn field_entries(value: &dyn PartialReflect, registry: &TypeRegistry) -> Vec<FieldEntry> {
    let concrete = value
        .try_as_reflect()
        .is_none()
        .then(|| {
            let info = value.get_represented_type_info()?;
            registry
                .get_type_data::<ReflectFromReflect>(info.type_id())?
                .from_reflect(value)
        })
        .flatten();
    let value = concrete
        .as_deref()
        .map(PartialReflect::as_partial_reflect)
        .unwrap_or(value);
    let mut walk = Walk {
        entries: Vec::new(),
    };
    if let Some(name) = value.try_downcast_ref::<Name>() {
        walk.push(
            String::new(),
            "value".to_string(),
            0,
            FieldValue::Label(name.as_str().to_string()),
        );
        return walk.entries;
    }
    let (value, path) = flatten_newtypes(value, String::new());
    if let Some(scalar) = scalar_value(value) {
        walk.push_scalar(path, "value".to_string(), 0, value, scalar);
    } else if let Some(variant) = variant_value(value) {
        walk.push(path.clone(), "variant".to_string(), 0, variant);
        walk.children(value, &path, 0, true);
    } else if let Some(summary) = summary(value) {
        if !summary.is_empty() {
            walk.push(
                path.clone(),
                "value".to_string(),
                0,
                FieldValue::Label(summary),
            );
        }
        walk.children(value, &path, 0, true);
    } else {
        walk.push(
            path,
            "value".to_string(),
            0,
            FieldValue::Label(format_fallback(value)),
        );
    }
    walk.entries
}

/// The state of a walk over a reflected value.
struct Walk {
    entries: Vec<FieldEntry>,
}

impl Walk {
    fn push(&mut self, path: String, label: String, depth: usize, value: FieldValue) {
        self.entries.push(FieldEntry {
            path,
            label,
            depth,
            value,
            limit: None,
        });
    }

    /// Adds the row of `scalar`, the value read from `value`, keeping the range of an integer.
    fn push_scalar(
        &mut self,
        path: String,
        label: String,
        depth: usize,
        value: &dyn PartialReflect,
        scalar: FieldValue,
    ) {
        let limit = match scalar {
            FieldValue::Number(_) => integer_limit(value),
            _ => None,
        };
        self.entries.push(FieldEntry {
            path,
            label,
            depth,
            value: scalar,
            limit,
        });
    }

    /// Adds the rows of `value`, rendering them read-only unless `editable`.
    fn field(
        &mut self,
        value: &dyn PartialReflect,
        path: String,
        label: String,
        depth: usize,
        editable: bool,
    ) {
        let (value, path) = flatten_newtypes(value, path);
        let lock = |value| if editable { value } else { read_only(value) };

        if depth >= MAX_DEPTH {
            self.push(path, label, depth, FieldValue::Label("...".to_string()));
            return;
        }

        if let Some(scalar) = scalar_value(value) {
            self.push_scalar(path, label, depth, value, lock(scalar));
            return;
        }

        if let Some(variant) = variant_value(value) {
            self.push(path.clone(), label, depth, lock(variant));
            self.children(value, &path, depth + 1, editable);
            return;
        }

        let Some(summary) = summary(value) else {
            self.push(
                path,
                label,
                depth,
                FieldValue::Label(format_fallback(value)),
            );
            return;
        };

        self.push(path.clone(), label, depth, FieldValue::Label(summary));
        self.children(value, &path, depth + 1, editable);
    }

    fn children(&mut self, value: &dyn PartialReflect, prefix: &str, depth: usize, editable: bool) {
        match value.reflect_ref() {
            ReflectRef::Struct(value) => {
                for index in 0..value.field_len() {
                    let Some(field) = value.field_at(index) else {
                        continue;
                    };
                    let name = value
                        .name_at(index)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string());
                    self.field(field, join(prefix, &name), name, depth, editable);
                }
            }
            ReflectRef::TupleStruct(value) => {
                for (index, field) in value.iter_fields().enumerate() {
                    let name = index.to_string();
                    self.field(field, join(prefix, &name), name, depth, editable);
                }
            }
            ReflectRef::Tuple(value) => {
                for (index, field) in value.iter_fields().enumerate() {
                    let name = index.to_string();
                    self.field(field, join(prefix, &name), name, depth, editable);
                }
            }
            ReflectRef::List(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = format!("{prefix}[{index}]");
                    self.field(item, path, index.to_string(), depth, editable);
                }
            }
            ReflectRef::Array(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = format!("{prefix}[{index}]");
                    self.field(item, path, index.to_string(), depth, editable);
                }
            }
            ReflectRef::Map(value) => {
                for (index, (key, item)) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = entry_path(prefix, key, index);
                    let editable = editable && key_access(key).is_some();
                    self.field(item, path, display_value(key), depth, editable);
                }
            }
            ReflectRef::Set(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = entry_path(prefix, item, index);
                    self.field(item, path, index.to_string(), depth, false);
                }
            }
            ReflectRef::Enum(value) => {
                for index in 0..value.field_len() {
                    let Some(field) = value.field_at(index) else {
                        continue;
                    };
                    let name = value
                        .name_at(index)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string());
                    self.field(field, join(prefix, &name), name, depth, editable);
                }
            }
            _ => {}
        }
    }
}

/// The read-only caption form of a field value.
pub(crate) fn read_only(value: FieldValue) -> FieldValue {
    FieldValue::Label(match value {
        FieldValue::Bool(value) => value.to_string(),
        FieldValue::Number(NumericValue::F32(value)) => value.to_string(),
        FieldValue::Number(NumericValue::F64(value)) => value.to_string(),
        FieldValue::Number(NumericValue::I32(value)) => value.to_string(),
        FieldValue::Number(NumericValue::I64(value)) => value.to_string(),
        FieldValue::Text(text) | FieldValue::Label(text) => text,
        FieldValue::Color(color) => color_to_hex(color),
        FieldValue::Variant { variants, selected } => {
            variants.get(selected).cloned().unwrap_or_default()
        }
    })
}

/// Follows single-field tuple structs down to their inner value, returning it and its path.
///
/// This keeps newtypes from adding a nesting level of their own.
fn flatten_newtypes(value: &dyn PartialReflect, mut path: String) -> (&dyn PartialReflect, String) {
    let mut value = value;
    while let ReflectRef::TupleStruct(tuple) = value.reflect_ref() {
        if tuple.field_len() != 1 {
            break;
        }
        let Some(field) = tuple.field(0) else {
            break;
        };
        path = join(&path, "0");
        value = field;
    }
    (value, path)
}

/// The caption shown on the row of a container value, or `None` if the value has no fields.
fn summary(value: &dyn PartialReflect) -> Option<String> {
    Some(match value.reflect_ref() {
        ReflectRef::Struct(_) | ReflectRef::TupleStruct(_) | ReflectRef::Tuple(_) => String::new(),
        ReflectRef::List(value) => format!("{} items", value.len()),
        ReflectRef::Array(value) => format!("{} items", value.len()),
        ReflectRef::Map(value) => format!("{} items", value.len()),
        ReflectRef::Set(value) => format!("{} items", value.len()),
        ReflectRef::Enum(value) => value.variant_name().to_string(),
        _ => return None,
    })
}

/// Formats a value that has no dedicated widget, preferring the reflected [`Display`] output.
///
/// Opaque types without a [`Display`] implementation fall back to their shortened type path.
///
/// [`Display`]: core::fmt::Display
fn format_fallback(value: &dyn PartialReflect) -> String {
    let displayed = format!("{value}");
    let displayed = if displayed.trim().is_empty() {
        format!("{value:?}")
    } else {
        displayed
    };
    let type_path = value.reflect_type_path();
    if displayed.trim() == format!("Reflect({type_path})") {
        return ShortName(type_path).to_string();
    }
    displayed
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

/// The path of the map or set entry at `index` with the given key.
///
/// Keys that [`key_access`] cannot express get a `[#index]` placeholder, which keeps the row
/// tracked but never resolves.
fn entry_path(prefix: &str, key: &dyn PartialReflect, index: usize) -> String {
    match key_access(key) {
        Some(access) => format!("{prefix}{access}"),
        None => format!("{prefix}[#{index}]"),
    }
}

/// The [`bevy_reflect::GetPath`] access selecting `key` in a map or set.
///
/// Returns `None` for keys other than strings and primitive integers.
fn key_access(key: &dyn PartialReflect) -> Option<String> {
    if let Some(key) = key.try_downcast_ref::<String>() {
        return Some(quoted_key(key));
    }
    if let Some(key) = key.try_downcast_ref::<Cow<'static, str>>() {
        return Some(quoted_key(key));
    }
    integer_key(key).map(|integer| quoted_key(&integer))
}

fn quoted_key(key: &str) -> String {
    let mut quoted = String::with_capacity(key.len() + 4);
    quoted.push_str("[\"");
    for c in key.chars() {
        if matches!(c, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push_str("\"]");
    quoted
}

fn integer_key(key: &dyn PartialReflect) -> Option<String> {
    macro_rules! integers {
        ($($type:ty),*) => {
            $(
                if let Some(key) = key.try_downcast_ref::<$type>() {
                    return Some(key.to_string());
                }
            )*
        };
    }

    integers!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    None
}

fn display_value(value: &dyn PartialReflect) -> String {
    format!("{value:?}")
}

fn scalar_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    if let Some(value) = value.try_downcast_ref::<bool>() {
        return Some(FieldValue::Bool(*value));
    }
    if let Some(value) = value.try_downcast_ref::<f32>() {
        return Some(FieldValue::Number(NumericValue::F32(*value)));
    }
    if let Some(value) = value.try_downcast_ref::<f64>() {
        return Some(FieldValue::Number(NumericValue::F64(*value)));
    }
    if let Some(value) = integer_value(value) {
        return Some(value);
    }
    if let Some(value) = value.try_downcast_ref::<char>() {
        return Some(FieldValue::Text(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<String>() {
        return Some(FieldValue::Text(value.clone()));
    }
    if let Some(value) = value.try_downcast_ref::<Cow<'static, str>>() {
        return Some(FieldValue::Text(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<&'static str>() {
        return Some(FieldValue::Label(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<Entity>() {
        return Some(FieldValue::Label(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<Color>() {
        return Some(FieldValue::Color(*value));
    }
    color_value(value)
}

fn color_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    macro_rules! color {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(FieldValue::Color(Color::from(*value)));
                }
            )*
        };
    }

    color!(Srgba, LinearRgba);
    None
}

/// Classifies an integer, falling back to a caption when it does not fit the number input.
fn integer_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    macro_rules! narrow {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(FieldValue::Number(NumericValue::I32(i32::from(*value))));
                }
            )*
        };
    }
    macro_rules! wide {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(match i64::try_from(*value) {
                        Ok(value) => FieldValue::Number(NumericValue::I64(value)),
                        Err(_) => FieldValue::Label(value.to_string()),
                    });
                }
            )*
        };
    }

    narrow!(i8, i16, i32, u8, u16);
    wide!(i64, i128, isize, u32, u64, u128, usize);
    None
}

/// The range of an integer type that is narrower than the number input showing it.
fn integer_limit(value: &dyn PartialReflect) -> Option<NumericRange> {
    macro_rules! limit {
        ($variant:ident, $repr:ty, $($type:ty),*) => {
            $(
                if value.try_downcast_ref::<$type>().is_some() {
                    return Some(NumericRange::$variant(
                        <$repr>::try_from(<$type>::MIN).unwrap_or(<$repr>::MIN)
                            ..=<$repr>::try_from(<$type>::MAX).unwrap_or(<$repr>::MAX),
                    ));
                }
            )*
        };
    }

    limit!(I32, i32, i8, i16, u8, u16);
    limit!(I64, i64, isize, u32, u64, usize);
    None
}

/// The variants of a unit-only enum, and the index of the current one.
///
/// Returns `None` for an enum with data, whose variant is shown as a caption instead.
fn variant_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    let ReflectRef::Enum(reflected) = value.reflect_ref() else {
        return None;
    };
    let TypeInfo::Enum(info) = value.get_represented_type_info()? else {
        return None;
    };
    if info
        .iter()
        .any(|variant| variant.variant_type() != VariantType::Unit)
    {
        return None;
    }
    let current = reflected.variant_name();
    let variants: Vec<String> = info
        .iter()
        .map(|variant| variant.name().to_string())
        .collect();
    let selected = variants.iter().position(|name| name == current)?;
    Some(FieldValue::Variant { variants, selected })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{column_split::ColumnSplitHandle, entity_tree::InspectorTreeView, InspectorPlugin};
    use alloc::collections::VecDeque;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::change_detection::DetectChanges;
    use bevy_ecs::{component::Component, query::With};
    use bevy_platform::sync::Arc;
    use bevy_reflect::TypePath;
    use core::time::Duration;

    #[derive(Reflect, Debug, Default)]
    struct Nested {
        depth: u32,
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Wrapper(Nested);

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Tagged {
        label: Name,
    }

    #[derive(Reflect, Debug, Default)]
    struct StrongHandle;

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Holder(Arc<StrongHandle>);

    #[derive(Reflect, Debug, Default, Clone, Copy, PartialEq)]
    enum Mode {
        #[default]
        Idle,
        Running,
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Subject {
        enabled: bool,
        scale: f32,
        name: String,
        nested: Nested,
        values: Vec<u32>,
        mode: Mode,
    }

    #[derive(Reflect, Debug, Default, PartialEq)]
    struct Locked(u8);

    #[derive(Reflect, Debug, Default, PartialEq)]
    enum Shape {
        #[default]
        Empty,
        Circle {
            radius: f32,
        },
        Custom(Locked),
    }

    #[derive(Reflect, Debug, Default, PartialEq)]
    struct Point(f32, f32);

    #[derive(Reflect, Debug, Default)]
    struct Outer {
        inner: Nested,
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Kinds {
        byte: u8,
        short: u16,
        word: u32,
        long: u64,
        huge: u128,
        size: usize,
        tiny: i8,
        small: i16,
        int: i32,
        big: i64,
        giant: i128,
        signed_size: isize,
        single: f32,
        double: f64,
        letter: char,
        text: String,
        cow: Cow<'static, str>,
        fixed: &'static str,
        color: Color,
        srgba: Srgba,
        linear: LinearRgba,
        shape: Shape,
        maybe: Option<u32>,
        pair: (u32, f32),
        point: Point,
        outer: Outer,
        tags: Vec<u32>,
        array: [f32; 3],
        queue: VecDeque<u32>,
        map: HashMap<String, u32>,
        set: HashSet<u32>,
    }

    #[derive(Reflect, Debug)]
    struct Linked {
        target: Entity,
    }

    #[derive(Resource, Debug, Default)]
    struct EditCount(usize);

    fn edit(app: &mut App, entity: Entity, path: &str, value: FieldValue) {
        edit_component::<Subject>(app, entity, path, value);
    }

    fn edit_component<C: TypePath>(app: &mut App, entity: Entity, path: &str, value: FieldValue) {
        app.world_mut().trigger(FieldEdit {
            entity,
            component: C::type_path().to_string(),
            path: path.to_string(),
            value,
        });
        app.world_mut().flush();
    }

    fn edit_kinds(app: &mut App, entity: Entity, path: &str, value: FieldValue) {
        edit_component::<Kinds>(app, entity, path, value);
    }

    fn kinds_app() -> (App, Entity) {
        let mut app = test_app();
        app.register_type::<Kinds>();
        let entity = app.world_mut().spawn(Kinds::default()).id();
        (app, entity)
    }

    fn int(value: i64) -> FieldValue {
        FieldValue::Number(NumericValue::I64(value))
    }

    fn variant(names: &[&str], selected: usize) -> FieldValue {
        FieldValue::Variant {
            variants: names.iter().map(ToString::to_string).collect(),
            selected,
        }
    }

    fn entry<'a>(entries: &'a [FieldEntry], path: &str) -> &'a FieldEntry {
        entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("no entry at `{path}` in {entries:?}"))
    }

    mod first {
        use bevy_ecs::{component::Component, reflect::ReflectComponent};
        use bevy_reflect::{prelude::ReflectDefault, Reflect};

        #[derive(Component, Reflect, Debug, Default)]
        #[reflect(Component, Default)]
        pub struct Duplicate {
            pub first: f32,
        }

        #[derive(Component, Reflect, Debug, Default)]
        #[reflect(Component)]
        pub struct Same {
            pub value: f32,
        }
    }

    mod second {
        use bevy_ecs::{component::Component, reflect::ReflectComponent};
        use bevy_reflect::{prelude::ReflectDefault, Reflect};

        #[derive(Component, Reflect, Debug, Default)]
        #[reflect(Component, Default)]
        pub struct Duplicate {
            pub second: bool,
        }

        #[derive(Component, Reflect, Debug, Default)]
        #[reflect(Component)]
        pub struct Same {
            pub value: f32,
        }
    }

    #[derive(Component)]
    struct Marker;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin,
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            InspectorPlugin,
        ));
        let mut units = bevy_feathers::controls::UnitsRegistry::default();
        units.insert(&bevy_feathers::controls::Dimensionless);
        app.insert_resource(units);
        app.insert_resource(bevy_feathers::theme::UiTheme(
            bevy_feathers::dark_theme::create_dark_theme(),
        ));
        app.init_resource::<bevy_input_focus::InputFocus>();
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app.register_type::<Subject>();
        app
    }

    fn kinds(entries: &[FieldEntry]) -> Vec<(String, FieldKind)> {
        entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.value.kind()))
            .collect()
    }

    #[test]
    fn walks_a_struct_into_field_entries() {
        let subject = Subject {
            enabled: true,
            scale: 2.0,
            name: "subject".to_string(),
            nested: Nested { depth: 3 },
            values: alloc::vec![7, 8],
            mode: Mode::Idle,
        };

        let entries = field_entries(&subject, &TypeRegistry::new());

        assert_eq!(
            kinds(&entries),
            alloc::vec![
                ("enabled".to_string(), FieldKind::Bool),
                ("scale".to_string(), FieldKind::Number),
                ("name".to_string(), FieldKind::Text),
                ("nested".to_string(), FieldKind::Label),
                ("nested.depth".to_string(), FieldKind::Number),
                ("values".to_string(), FieldKind::Label),
                ("values[0]".to_string(), FieldKind::Number),
                ("values[1]".to_string(), FieldKind::Number),
                ("mode".to_string(), FieldKind::Variant),
            ]
        );
        assert_eq!(entries[4].depth, 1);
        assert_eq!(entries[5].value, FieldValue::Label("2 items".to_string()));
    }

    #[test]
    fn field_edits_write_into_the_component() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Subject::default()).id();

        edit(
            &mut app,
            subject,
            "scale",
            FieldValue::Number(NumericValue::F32(2.5)),
        );
        edit(&mut app, subject, "enabled", FieldValue::Bool(true));
        edit(
            &mut app,
            subject,
            "name",
            FieldValue::Text("edited".to_string()),
        );
        edit(
            &mut app,
            subject,
            "nested.depth",
            FieldValue::Number(NumericValue::I32(7)),
        );
        edit(
            &mut app,
            subject,
            "mode",
            FieldValue::Variant {
                variants: alloc::vec!["Idle".to_string(), "Running".to_string()],
                selected: 1,
            },
        );

        let subject = app.world().get::<Subject>(subject).unwrap();
        assert_eq!(subject.scale, 2.5);
        assert!(subject.enabled);
        assert_eq!(subject.name, "edited");
        assert_eq!(subject.nested.depth, 7);
        assert_eq!(subject.mode, Mode::Running);
    }

    #[test]
    fn a_bad_field_path_leaves_the_component_unchanged() {
        let mut app = test_app();
        let subject = app
            .world_mut()
            .spawn(Subject {
                scale: 1.0,
                ..Default::default()
            })
            .id();

        edit(
            &mut app,
            subject,
            "missing.field",
            FieldValue::Number(NumericValue::F32(9.0)),
        );
        edit(
            &mut app,
            subject,
            "scale",
            FieldValue::Text("not a number".to_string()),
        );

        assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 1.0);
    }

    #[test]
    fn an_edit_is_not_echoed_back_by_the_sync_pass() {
        let mut app = test_app();
        app.init_resource::<EditCount>();
        app.add_observer(|_: On<FieldEdit>, mut count: ResMut<EditCount>| count.0 += 1);
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app
            .world_mut()
            .spawn(Subject {
                name: "start".to_string(),
                ..Default::default()
            })
            .id();

        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
        app.update();
        assert_eq!(app.world().resource::<EditCount>().0, 0);

        edit(
            &mut app,
            subject,
            "name",
            FieldValue::Text("edited".to_string()),
        );
        assert_eq!(app.world().resource::<EditCount>().0, 1);

        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<DetailsPanelSync>()
                .set_dirty();
            app.update();
        }

        assert_eq!(app.world().resource::<EditCount>().0, 1);
        assert_eq!(app.world().get::<Subject>(subject).unwrap().name, "edited");
    }

    #[test]
    fn collapses_newtype_tuple_structs() {
        let entries = field_entries(&Wrapper(Nested { depth: 3 }), &TypeRegistry::new());

        assert_eq!(
            kinds(&entries),
            alloc::vec![("0.depth".to_string(), FieldKind::Number)]
        );
        assert_eq!(entries[0].depth, 0);
    }

    #[test]
    fn keeps_opaque_captions_stable() {
        let mut app = test_app();
        app.register_type::<Tagged>();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app
            .world_mut()
            .spawn(Tagged {
                label: Name::new("hello"),
            })
            .id();

        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
        app.update();

        fn captions(app: &App) -> Vec<(String, String)> {
            let index = app.world().resource::<DetailsIndex>();
            let mut rows: Vec<(String, String)> = index
                .fields
                .iter()
                .filter(|(_, widget)| matches!(widget.value, FieldValue::Label(_)))
                .map(|(key, widget)| {
                    let text = app
                        .world()
                        .get::<Text>(widget.entity)
                        .map(|text| text.0.clone())
                        .unwrap_or_default();
                    (key.1.clone(), text)
                })
                .collect();
            rows.sort();
            rows
        }

        let first = captions(&app);
        assert!(!first.is_empty());
        assert!(first.iter().all(|(_, text)| !text.is_empty()));

        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<DetailsPanelSync>()
                .set_dirty();
            app.update();
        }

        assert_eq!(captions(&app), first);
    }

    #[test]
    fn survives_repeated_selection_changes() {
        let mut app = test_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subjects: Vec<Entity> = (0..3)
            .map(|index| {
                app.world_mut()
                    .spawn(Subject {
                        scale: index as f32,
                        name: index.to_string(),
                        ..Default::default()
                    })
                    .id()
            })
            .collect();

        for subject in subjects {
            app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
            for _ in 0..5 {
                app.update();
            }

            let index = app.world().resource::<DetailsIndex>();
            assert!(!index.is_empty());
            let widgets: Vec<Entity> = index.fields.values().map(|field| field.entity).collect();
            for widget in widgets {
                assert!(app.world().get_entity(widget).is_ok());
            }
        }

        app.world_mut().resource_mut::<InspectorSelection>().0 = None;
        for _ in 0..5 {
            app.update();
        }
        assert!(app.world().resource::<DetailsIndex>().is_empty());
    }

    #[test]
    fn rebuilds_on_selection_and_updates_values_in_place() {
        let mut app = test_app();
        let ui_root = app.world_mut().spawn(InspectorUi).id();
        app.world_mut().spawn((InspectorTreeView, ChildOf(ui_root)));
        let panel = app
            .world_mut()
            .spawn((InspectorDetailsBody, ChildOf(ui_root)))
            .id();
        let subject = app
            .world_mut()
            .spawn(Subject {
                scale: 1.0,
                ..Default::default()
            })
            .id();

        app.update();
        assert!(app.world().resource::<DetailsIndex>().is_empty());
        assert_eq!(
            app.world()
                .get::<Children>(panel)
                .map(|children| children.iter().count()),
            Some(1)
        );

        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
        app.update();

        let component = app.world().component_id::<Subject>().unwrap();
        let index = app.world().resource::<DetailsIndex>();
        let scale = index.widget(component, "scale").unwrap();
        let enabled = index.widget(component, "enabled").unwrap();
        assert_eq!(
            app.world().get::<NumericValue>(scale),
            Some(&NumericValue::F32(1.0))
        );

        app.world_mut().get_mut::<Subject>(subject).unwrap().scale = 4.0;
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();

        let index = app.world().resource::<DetailsIndex>();
        assert_eq!(index.widget(component, "scale"), Some(scale));
        assert_eq!(index.widget(component, "enabled"), Some(enabled));
        assert_eq!(
            app.world().get::<NumericValue>(scale),
            Some(&NumericValue::F32(4.0))
        );
    }

    #[test]
    fn renders_a_name_as_a_single_caption() {
        let entries = field_entries(&Name::new("Left Cube"), &TypeRegistry::new());

        assert_eq!(
            entries.len(),
            1,
            "expected one caption row, got {entries:?}"
        );
        assert_eq!(entries[0].value, FieldValue::Label("Left Cube".to_string()));
    }

    #[test]
    fn shortens_opaque_type_paths() {
        let entries = field_entries(&Holder(Arc::new(StrongHandle)), &TypeRegistry::new());

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].value,
            FieldValue::Label("Arc<StrongHandle>".to_string())
        );
    }

    #[test]
    fn wraps_long_value_captions() {
        let mut app = test_app();
        app.register_type::<Holder>();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app.world_mut().spawn(Holder(Arc::new(StrongHandle))).id();

        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
        app.update();

        let index = app.world().resource::<DetailsIndex>();
        let captions: Vec<Entity> = index
            .fields
            .values()
            .filter(|widget| matches!(widget.value, FieldValue::Label(_)))
            .map(|widget| widget.entity)
            .collect();
        assert!(!captions.is_empty());
        for caption in captions {
            let layout = app.world().get::<TextLayout>(caption).unwrap();
            assert_eq!(layout.linebreak, LineBreak::AnyCharacter);
            let node = app.world().get::<Node>(caption).unwrap();
            assert_eq!(node.min_width, px(0));
            assert_eq!(node.flex_shrink, 1.0);
        }
    }

    #[test]
    fn clips_labels_to_a_single_line() {
        let mut app = test_app();
        inspect(&mut app, Subject::default());

        let rows = row_entities(&app);
        assert!(!rows.is_empty());
        let world = app.world();
        for row in rows {
            let leading = world.get::<Children>(row).unwrap()[0];
            assert!(world.entity(leading).contains::<ColumnSplitLeading>());
            assert_eq!(
                world.get::<Node>(leading).unwrap().overflow,
                Overflow::clip_x()
            );
            let label = world.get::<Children>(leading).unwrap()[0];
            let layout = world.get::<TextLayout>(label).unwrap();
            assert_eq!(layout.linebreak, LineBreak::NoWrap);
        }
    }

    fn fields_container(app: &App, component: ComponentId) -> Entity {
        let world = app.world();
        let group = world.resource::<DetailsIndex>().group(component).unwrap();
        descendant_with::<InspectorDetailsFields>(world, group).unwrap()
    }

    #[test]
    fn keeps_each_group_column_split_across_respawns() {
        let mut app = test_app();
        app.register_type::<Wrapper>();
        inspect(&mut app, (Subject::default(), Wrapper::default()));
        let subject = app.world().component_id::<Subject>().unwrap();
        let wrapper = app.world().component_id::<Wrapper>().unwrap();

        let container = fields_container(&app, subject);
        app.world_mut()
            .get_mut::<ColumnSplit>(container)
            .unwrap()
            .fraction = 0.6;
        app.update();

        app.world_mut()
            .resource_mut::<DetailsCollapsed>()
            .0
            .insert(subject);
        refresh(&mut app);
        app.world_mut()
            .resource_mut::<DetailsCollapsed>()
            .0
            .remove(&subject);
        refresh(&mut app);

        let respawned = fields_container(&app, subject);
        assert_ne!(respawned, container);
        let fraction = |entity| app.world().get::<ColumnSplit>(entity).unwrap().fraction;
        assert_eq!(fraction(respawned), 0.6);
        assert_eq!(fraction(fields_container(&app, wrapper)), 0.4);
    }

    #[derive(Reflect, Debug, Clone, PartialEq)]
    enum Payload {
        Number(f32),
        Text(String),
    }

    impl Default for Payload {
        fn default() -> Self {
            Payload::Number(1.0)
        }
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Changing {
        items: Vec<u32>,
        maybe: Option<f32>,
        payload: Payload,
        lookup: HashMap<String, u32>,
    }

    #[derive(Component, Reflect, Debug)]
    #[reflect(Component)]
    enum Stage {
        A(f32),
        B(f32),
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Items(Vec<u32>);

    #[derive(Reflect, Debug, Default)]
    struct Large {
        unsigned: u64,
        size: usize,
    }

    #[derive(Reflect, Debug, Default)]
    struct Pair(u8, u8);

    #[derive(Reflect, Debug)]
    struct Everything {
        nested: Nested,
        wrapper: Wrapper,
        pair: Pair,
        tuple: (bool, f32),
        list: Vec<u32>,
        array: [u8; 2],
        option: Option<f32>,
        shape: Shape,
    }

    #[derive(Reflect, Debug)]
    struct Keyed {
        map: HashMap<String, u32>,
        numbers: HashMap<i32, f32>,
        set: HashSet<u32>,
    }

    fn inspect<B: bevy_ecs::bundle::Bundle>(app: &mut App, bundle: B) -> Entity {
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let entity = app.world_mut().spawn(bundle).id();
        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(entity);
        app.update();
        entity
    }

    fn select(app: &mut App, entity: Option<Entity>) {
        app.world_mut().resource_mut::<InspectorSelection>().0 = entity;
        app.update();
    }

    fn refresh(app: &mut App) {
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();
    }

    fn tracked_paths(app: &App) -> Vec<String> {
        let mut paths: Vec<String> = app
            .world()
            .resource::<DetailsIndex>()
            .fields
            .keys()
            .map(|(_, path)| path.clone())
            .collect();
        paths.sort();
        paths
    }

    fn widget_at(app: &App, path: &str) -> Option<Entity> {
        app.world()
            .resource::<DetailsIndex>()
            .fields
            .iter()
            .find(|((_, key), _)| key == path)
            .map(|(_, widget)| widget.entity)
    }

    /// The label text of every tracked row, read from the row's leading column.
    fn row_labels(app: &App) -> Vec<String> {
        let world = app.world();
        row_entities(app)
            .into_iter()
            .filter_map(|row| {
                let leading = *world.get::<Children>(row)?.first()?;
                let label = *world.get::<Children>(leading)?.first()?;
                Some(world.get::<Text>(label)?.0.clone())
            })
            .collect()
    }

    fn row_entities(app: &App) -> Vec<Entity> {
        let world = app.world();
        world
            .resource::<DetailsIndex>()
            .fields
            .values()
            .filter_map(|widget| {
                let mut row = widget.entity;
                loop {
                    let parent = world.get::<ChildOf>(row)?.parent();
                    if world.entity(parent).contains::<InspectorDetailsFields>() {
                        return Some(row);
                    }
                    row = parent;
                }
            })
            .collect()
    }

    fn displayed(value: &FieldValue) -> String {
        match value {
            FieldValue::Number(number) => number.to_string(),
            FieldValue::Text(text) | FieldValue::Label(text) => text.clone(),
            other => panic!("unexpected value {other:?}"),
        }
    }

    #[test]
    fn removes_rows_that_no_longer_exist() {
        let mut app = test_app();
        app.register_type::<Changing>();
        let subject = inspect(
            &mut app,
            Changing {
                items: alloc::vec![1, 2, 3],
                maybe: Some(2.0),
                ..Default::default()
            },
        );

        {
            let mut changing = app.world_mut().get_mut::<Changing>(subject).unwrap();
            changing.items.truncate(1);
            changing.maybe = None;
        }
        refresh(&mut app);

        let changing = app.world().get::<Changing>(subject).unwrap();
        let mut expected: Vec<String> = field_entries(changing, &TypeRegistry::new())
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        expected.sort();
        assert_eq!(tracked_paths(&app), expected);

        let rows: usize = app
            .world_mut()
            .query_filtered::<&Children, With<InspectorDetailsFields>>()
            .iter(app.world())
            .flat_map(|rows| rows.iter())
            .filter(|&&row| !app.world().entity(row).contains::<ColumnSplitHandle>())
            .count();
        assert_eq!(rows, expected.len());
    }

    #[test]
    fn replaces_widgets_when_a_field_changes_kind() {
        let mut app = test_app();
        app.register_type::<Changing>();
        let subject = inspect(&mut app, Changing::default());
        let before = widget_at(&app, "payload.0").unwrap();
        assert!(app.world().get::<NumericValue>(before).is_some());

        app.world_mut()
            .get_mut::<Changing>(subject)
            .unwrap()
            .payload = Payload::Text("hello".to_string());
        refresh(&mut app);

        let after = widget_at(&app, "payload.0").unwrap();
        assert!(app.world().get::<EditableText>(after).is_some());
        assert!(app.world().get::<NumericValue>(after).is_none());
    }

    #[test]
    fn relabels_rows_when_map_keys_change() {
        let mut app = test_app();
        app.register_type::<Changing>();
        let mut lookup = HashMap::default();
        lookup.insert("old".to_string(), 1);
        let subject = inspect(
            &mut app,
            Changing {
                lookup,
                ..Default::default()
            },
        );
        assert!(row_labels(&app).iter().any(|label| label.contains("old")));

        {
            let mut changing = app.world_mut().get_mut::<Changing>(subject).unwrap();
            changing.lookup.clear();
            changing.lookup.insert("new".to_string(), 1);
        }
        refresh(&mut app);

        let labels = row_labels(&app);
        assert!(
            labels.iter().any(|label| label.contains("new")),
            "{labels:?}"
        );
        assert!(
            !labels.iter().any(|label| label.contains("old")),
            "{labels:?}"
        );
    }

    #[test]
    fn distinguishes_variants_of_an_enum_component() {
        assert_ne!(
            field_entries(&Stage::A(1.0), &TypeRegistry::new()),
            field_entries(&Stage::B(1.0), &TypeRegistry::new())
        );
    }

    #[test]
    fn summarizes_a_list_component() {
        let entries = field_entries(&Items((0..20).collect()), &TypeRegistry::new());

        assert!(
            entries.iter().any(
                |entry| matches!(&entry.value, FieldValue::Label(text) if text.contains("20"))
            ),
            "{entries:?}"
        );
    }

    #[test]
    fn does_not_clamp_large_integers() {
        let entries = field_entries(
            &Large {
                unsigned: u64::MAX,
                size: usize::MAX,
            },
            &TypeRegistry::new(),
        );

        assert_eq!(displayed(&entries[0].value), u64::MAX.to_string());
        assert_eq!(displayed(&entries[1].value), usize::MAX.to_string());
    }

    fn unresolved_paths<T: Reflect>(value: &T) -> Vec<String> {
        field_entries(value, &TypeRegistry::new())
            .into_iter()
            .filter_map(|entry| match value.reflect_path(entry.path.as_str()) {
                Err(error) => Some(format!("{:?}: {error}", entry.path)),
                Ok(resolved) => match scalar_value(resolved) {
                    Some(scalar) if read_only(scalar.clone()) != read_only(entry.value.clone()) => {
                        Some(format!("{:?}: {scalar:?} != {:?}", entry.path, entry.value))
                    }
                    _ => None,
                },
            })
            .collect()
    }

    #[test]
    fn field_paths_resolve_to_their_values() {
        let everything = Everything {
            nested: Nested { depth: 1 },
            wrapper: Wrapper(Nested { depth: 2 }),
            pair: Pair(3, 4),
            tuple: (true, 5.0),
            list: alloc::vec![6],
            array: [8, 9],
            option: Some(11.0),
            shape: Shape::Circle { radius: 12.0 },
        };

        let mismatches = unresolved_paths(&everything);
        assert!(mismatches.is_empty(), "{mismatches:#?}");
    }

    #[test]
    fn map_and_set_paths_resolve_to_their_values() {
        let keyed = Keyed {
            map: [
                ("key".to_string(), 7),
                ("a \"quoted\" \\ key".to_string(), 8),
            ]
            .into_iter()
            .collect(),
            numbers: [(-3, 1.0), (4, 2.0)].into_iter().collect(),
            set: [10, 11].into_iter().collect(),
        };

        let paths = field_paths(&keyed);
        for expected in [
            "map[\"key\"]",
            "map[\"a \\\"quoted\\\" \\\\ key\"]",
            "numbers[\"-3\"]",
            "numbers[\"4\"]",
            "set[\"10\"]",
            "set[\"11\"]",
        ] {
            assert!(paths.iter().any(|path| path == expected), "{paths:?}");
        }
        let mismatches = unresolved_paths(&keyed);
        assert!(mismatches.is_empty(), "{mismatches:#?}");
    }

    fn field_paths<T: Reflect>(value: &T) -> Vec<String> {
        field_entries(value, &TypeRegistry::new())
            .into_iter()
            .map(|entry| entry.path)
            .collect()
    }

    #[test]
    fn gives_unexpressible_keys_distinct_unresolvable_paths() {
        #[derive(Reflect, Debug)]
        struct Flags {
            flags: HashMap<bool, u32>,
        }

        let flags = Flags {
            flags: [(false, 1), (true, 2)].into_iter().collect(),
        };
        let entries: Vec<FieldEntry> = field_entries(&flags, &TypeRegistry::new())
            .into_iter()
            .filter(|entry| entry.path.starts_with("flags["))
            .collect();

        assert_eq!(entries.len(), 2, "{entries:?}");
        assert_ne!(entries[0].path, entries[1].path);
        for entry in &entries {
            assert!(
                flags.reflect_path(entry.path.as_str()).is_err(),
                "{entry:?}"
            );
            assert_eq!(entry.value.kind(), FieldKind::Label, "{entry:?}");
        }
    }

    #[test]
    fn tracks_components_sharing_a_short_name() {
        let mut app = test_app();
        app.register_type::<first::Same>();
        app.register_type::<second::Same>();
        inspect(
            &mut app,
            (first::Same { value: 1.0 }, second::Same { value: 2.0 }),
        );

        let values = tracked_paths(&app)
            .into_iter()
            .filter(|path| path == "value")
            .count();
        assert_eq!(values, 2);
    }

    #[test]
    fn clears_a_label_that_becomes_empty() {
        let mut app = test_app();
        app.register_type::<Name>();
        let subject = inspect(&mut app, Name::new("hello"));

        *app.world_mut().get_mut::<Name>(subject).unwrap() = Name::new("");
        refresh(&mut app);

        let caption = widget_at(&app, "").unwrap();
        assert_eq!(app.world().get::<Text>(caption).unwrap().0, "");
    }

    fn body_children(app: &mut App) -> Vec<Entity> {
        let body = find_body(app.world_mut()).unwrap();
        app.world()
            .get::<Children>(body)
            .map(|children| children.to_vec())
            .unwrap_or_default()
    }

    fn body_message(app: &mut App) -> Option<String> {
        let children = body_children(app);
        let [caption] = children.as_slice() else {
            return None;
        };
        app.world().get::<Text>(*caption).map(|text| text.0.clone())
    }

    #[test]
    fn separates_components_sharing_a_short_name() {
        let mut app = test_app();
        app.register_type::<first::Duplicate>();
        app.register_type::<second::Duplicate>();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app
            .world_mut()
            .spawn((first::Duplicate::default(), second::Duplicate::default()))
            .id();
        let first = app.world().component_id::<first::Duplicate>().unwrap();
        let second = app.world().component_id::<second::Duplicate>().unwrap();
        assert_eq!(
            crate::component_short_name(app.world(), first),
            crate::component_short_name(app.world(), second)
        );

        select(&mut app, Some(subject));

        let index = app.world().resource::<DetailsIndex>();
        assert!(index.widget(first, "first").is_some());
        assert!(index.widget(second, "second").is_some());
        assert_ne!(index.group(first), index.group(second));
        assert_eq!(body_children(&mut app).len(), 2);

        app.world_mut()
            .resource_mut::<DetailsCollapsed>()
            .0
            .insert(first);
        refresh(&mut app);

        let index = app.world().resource::<DetailsIndex>();
        assert!(index.widget(first, "first").is_none());
        assert!(index.widget(second, "second").is_some());
        assert_eq!(body_children(&mut app).len(), 2);
    }

    #[test]
    fn shows_a_message_when_nothing_is_selected() {
        let mut app = test_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));

        app.update();

        assert_eq!(
            body_message(&mut app).as_deref(),
            Some("No entity selected")
        );
    }

    #[test]
    fn classifies_every_field_kind() {
        let mut app = test_app();
        app.register_type::<Kinds>();
        let registry = app.world().resource::<AppTypeRegistry>().read();
        let kinds = Kinds {
            long: u64::MAX,
            huge: 5,
            letter: 'x',
            fixed: "fixed",
            shape: Shape::Circle { radius: 1.0 },
            tags: alloc::vec![1, 2],
            map: [("a".to_string(), 1)].into_iter().collect(),
            set: [4].into_iter().collect(),
            ..Default::default()
        };

        let entries = field_entries(&kinds, &registry);

        assert_eq!(
            entry(&entries, "byte").value,
            FieldValue::Number(NumericValue::I32(0))
        );
        assert_eq!(entry(&entries, "huge").value, int(5));
        assert_eq!(
            entry(&entries, "long").value,
            FieldValue::Label(u64::MAX.to_string())
        );
        assert_eq!(
            entry(&entries, "letter").value,
            FieldValue::Text("x".to_string())
        );
        assert_eq!(entry(&entries, "cow").value.kind(), FieldKind::Text);
        assert_eq!(
            entry(&entries, "fixed").value,
            FieldValue::Label("fixed".to_string())
        );
        assert_eq!(entry(&entries, "linear").value.kind(), FieldKind::Color);
        assert_eq!(
            entry(&entries, "shape").value,
            FieldValue::Label("Circle".to_string())
        );
        assert_eq!(
            entry(&entries, "shape.radius").value.kind(),
            FieldKind::Number
        );
        assert_eq!(
            entry(&entries, "maybe").value,
            FieldValue::Label("None".to_string())
        );
        assert_eq!(
            entry(&entries, "outer.inner.depth").value.kind(),
            FieldKind::Number
        );
        assert_eq!(entry(&entries, "outer.inner.depth").depth, 2);
        assert_eq!(entry(&entries, "array[2]").value.kind(), FieldKind::Number);
        assert_eq!(
            entry(&entries, "queue").value,
            FieldValue::Label("0 items".to_string())
        );
        assert_eq!(entry(&entries, "map[\"a\"]").value, int(1));
        assert_eq!(
            entry(&entries, "set[\"4\"]").value,
            FieldValue::Label("4".to_string())
        );

        let linked = field_entries(
            &Linked {
                target: Entity::PLACEHOLDER,
            },
            &registry,
        );
        assert_eq!(linked[0].value.kind(), FieldKind::Label);
    }

    #[test]
    fn edits_every_integer_width() {
        let (mut app, entity) = kinds_app();

        for (path, value) in [
            ("byte", 200),
            ("short", 60_000),
            ("word", 4_000_000_000),
            ("long", i64::MAX),
            ("huge", 7),
            ("size", 9),
            ("tiny", -100),
            ("small", -30_000),
            ("int", -2_000_000_000),
            ("big", i64::MIN),
            ("giant", -7),
            ("signed_size", -9),
        ] {
            edit_kinds(&mut app, entity, path, int(value));
        }

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.byte, 200);
        assert_eq!(kinds.short, 60_000);
        assert_eq!(kinds.word, 4_000_000_000);
        assert_eq!(kinds.long, i64::MAX as u64);
        assert_eq!(kinds.huge, 7);
        assert_eq!(kinds.size, 9);
        assert_eq!(kinds.tiny, -100);
        assert_eq!(kinds.small, -30_000);
        assert_eq!(kinds.int, -2_000_000_000);
        assert_eq!(kinds.big, i64::MIN);
        assert_eq!(kinds.giant, -7);
        assert_eq!(kinds.signed_size, -9);
    }

    #[test]
    fn rejects_out_of_range_integers() {
        let (mut app, entity) = kinds_app();
        edit_kinds(&mut app, entity, "byte", int(12));

        edit_kinds(&mut app, entity, "byte", int(300));
        edit_kinds(&mut app, entity, "word", int(-1));
        edit_kinds(
            &mut app,
            entity,
            "tiny",
            FieldValue::Number(NumericValue::F32(1.5)),
        );

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.byte, 12);
        assert_eq!(kinds.word, 0);
        assert_eq!(kinds.tiny, 0);
    }

    #[test]
    fn edits_floats_without_losing_precision() {
        let (mut app, entity) = kinds_app();

        edit_kinds(
            &mut app,
            entity,
            "single",
            FieldValue::Number(NumericValue::F32(0.3)),
        );
        edit_kinds(
            &mut app,
            entity,
            "double",
            FieldValue::Number(NumericValue::F64(0.1)),
        );

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.single, 0.3);
        assert_eq!(kinds.double, 0.1);
    }

    #[test]
    fn edits_chars_from_the_last_typed_character() {
        let (mut app, entity) = kinds_app();

        edit_kinds(
            &mut app,
            entity,
            "letter",
            FieldValue::Text("q".to_string()),
        );
        edit_kinds(&mut app, entity, "letter", FieldValue::Text(String::new()));
        assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'q');

        edit_kinds(
            &mut app,
            entity,
            "letter",
            FieldValue::Text("qz".to_string()),
        );
        assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'z');
        edit_kinds(
            &mut app,
            entity,
            "letter",
            FieldValue::Text("xy".to_string()),
        );
        assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'y');
    }

    #[test]
    fn edits_strings() {
        let (mut app, entity) = kinds_app();

        edit_kinds(
            &mut app,
            entity,
            "text",
            FieldValue::Text("owned".to_string()),
        );
        edit_kinds(&mut app, entity, "cow", FieldValue::Text("cow".to_string()));
        edit_kinds(
            &mut app,
            entity,
            "fixed",
            FieldValue::Text("static".to_string()),
        );

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.text, "owned");
        assert_eq!(kinds.cow, "cow");
        assert_eq!(kinds.fixed, "");
    }

    #[test]
    fn edits_fields_of_the_current_variant_only() {
        let (mut app, entity) = kinds_app();
        {
            let mut kinds = app.world_mut().get_mut::<Kinds>(entity).unwrap();
            kinds.shape = Shape::Circle { radius: 1.0 };
            kinds.maybe = Some(1);
        }

        edit_kinds(
            &mut app,
            entity,
            "shape.radius",
            FieldValue::Number(NumericValue::F32(2.0)),
        );
        edit_kinds(&mut app, entity, "maybe.0", int(5));
        edit_kinds(
            &mut app,
            entity,
            "shape",
            variant(&["Empty", "Circle", "Custom"], 0),
        );
        edit_kinds(&mut app, entity, "maybe", variant(&["None", "Some"], 0));

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.shape, Shape::Circle { radius: 2.0 });
        assert_eq!(kinds.maybe, Some(5));
    }

    #[test]
    fn edits_nested_and_indexed_leaves() {
        let (mut app, entity) = kinds_app();
        {
            let mut kinds = app.world_mut().get_mut::<Kinds>(entity).unwrap();
            kinds.tags = alloc::vec![1, 2, 3];
            kinds.queue = [1, 2].into_iter().collect();
        }

        edit_kinds(&mut app, entity, "outer.inner.depth", int(4));
        edit_kinds(
            &mut app,
            entity,
            "pair.1",
            FieldValue::Number(NumericValue::F32(1.5)),
        );
        edit_kinds(
            &mut app,
            entity,
            "point.0",
            FieldValue::Number(NumericValue::F32(2.5)),
        );
        edit_kinds(&mut app, entity, "tags[1]", int(20));
        edit_kinds(
            &mut app,
            entity,
            "array[2]",
            FieldValue::Number(NumericValue::F32(3.5)),
        );
        edit_kinds(&mut app, entity, "queue[0]", int(10));

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.outer.inner.depth, 4);
        assert_eq!(kinds.pair.1, 1.5);
        assert_eq!(kinds.point, Point(2.5, 0.0));
        assert_eq!(kinds.tags, alloc::vec![1, 20, 3]);
        assert_eq!(kinds.array, [0.0, 0.0, 3.5]);
        assert_eq!(kinds.queue, VecDeque::from([10, 2]));
    }

    #[test]
    fn refreshing_every_kind_emits_no_edits() {
        let (mut app, entity) = kinds_app();
        app.init_resource::<EditCount>();
        app.add_observer(|_: On<FieldEdit>, mut count: ResMut<EditCount>| count.0 += 1);
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        app.world_mut().get_mut::<Kinds>(entity).unwrap().color = Color::hsla(0.0, 0.5, 0.5, 1.0);

        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(entity);
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<DetailsPanelSync>()
                .set_dirty();
            app.update();
        }
        app.world_mut().get_mut::<Kinds>(entity).unwrap().color = Color::hsla(120.0, 0.5, 0.5, 1.0);
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<DetailsPanelSync>()
                .set_dirty();
            app.update();
        }

        assert_eq!(app.world().resource::<EditCount>().0, 0);
    }

    fn widget_app() -> (App, Entity) {
        let mut app = test_app();
        app.add_plugins(bevy_text::TextPlugin);
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app
            .world_mut()
            .spawn(Subject {
                scale: 1.0,
                name: "start".to_string(),
                ..Default::default()
            })
            .id();
        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
        app.update();
        (app, subject)
    }

    fn field_widget(app: &mut App, path: &str) -> Entity {
        let mut query = app.world_mut().query::<(Entity, &InspectorField)>();
        query
            .iter(app.world())
            .find(|(_, field)| field.path == path)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("no widget for `{path}`"))
    }

    fn settle(app: &mut App) {
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<DetailsPanelSync>()
                .set_dirty();
            app.update();
        }
    }

    #[test]
    fn dragging_a_number_input_edits_the_field() {
        let (mut app, subject) = widget_app();
        let input = field_widget(&mut app, "scale");

        for (value, is_final) in [(1.5_f32, false), (2.0, false), (2.5, true)] {
            app.world_mut().trigger(ValueChange {
                source: input,
                value,
                is_final,
            });
            app.update();
            assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, value);
            assert_eq!(
                app.world().get::<NumericValue>(input),
                Some(&NumericValue::F32(value))
            );
        }

        settle(&mut app);
        assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 2.5);
        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(2.5))
        );
    }

    #[test]
    fn shows_a_message_for_an_entity_without_components() {
        let mut app = test_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app.world_mut().spawn_empty().id();

        select(&mut app, Some(subject));

        assert_eq!(
            body_message(&mut app).as_deref(),
            Some("The selected entity has no components")
        );
    }

    #[test]
    fn shows_a_message_when_the_selection_is_despawned() {
        let mut app = test_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app.world_mut().spawn(Subject::default()).id();
        select(&mut app, Some(subject));
        assert_eq!(body_message(&mut app), None);

        app.world_mut().entity_mut(subject).despawn();
        refresh(&mut app);

        assert_eq!(
            body_message(&mut app).as_deref(),
            Some("The selected entity no longer exists")
        );
        assert!(app.world().resource::<DetailsIndex>().is_empty());
    }

    #[test]
    fn keeps_unchanged_groups_when_components_change() {
        let mut app = test_app();
        app.register_type::<Holder>();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app.world_mut().spawn(Subject::default()).id();
        let subject_id = app.world().component_id::<Subject>().unwrap();
        select(&mut app, Some(subject));
        let group = app
            .world()
            .resource::<DetailsIndex>()
            .group(subject_id)
            .unwrap();
        let scale = app
            .world()
            .resource::<DetailsIndex>()
            .widget(subject_id, "scale")
            .unwrap();

        app.world_mut()
            .entity_mut(subject)
            .insert(Holder(Arc::new(StrongHandle)));
        refresh(&mut app);

        let holder_id = app.world().component_id::<Holder>().unwrap();
        let index = app.world().resource::<DetailsIndex>();
        let holder = index.group(holder_id).unwrap();
        assert_eq!(index.group(subject_id), Some(group));
        assert_eq!(index.widget(subject_id, "scale"), Some(scale));
        assert_eq!(body_children(&mut app), alloc::vec![holder, group]);

        app.world_mut().entity_mut(subject).remove::<Holder>();
        refresh(&mut app);

        let index = app.world().resource::<DetailsIndex>();
        assert_eq!(index.group(holder_id), None);
        assert_eq!(index.group(subject_id), Some(group));
        assert!(app.world().get_entity(holder).is_err());
        assert_eq!(body_children(&mut app), alloc::vec![group]);
    }

    #[test]
    fn finds_the_first_descendant_in_child_order() {
        let mut world = World::new();
        let root = world.spawn(Marker).id();
        let first = world.spawn(ChildOf(root)).id();
        let nested = world.spawn((Marker, ChildOf(first))).id();
        world.spawn((Marker, ChildOf(root)));

        assert_eq!(descendant_with::<Marker>(&world, root), Some(nested));
    }

    #[test]
    fn never_returns_the_root_itself() {
        let mut world = World::new();
        let root = world.spawn(Marker).id();
        world.spawn(ChildOf(root));

        assert_eq!(descendant_with::<Marker>(&world, root), None);
    }

    #[test]
    fn finds_nothing_under_a_missing_root() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        world.despawn(root);

        assert_eq!(descendant_with::<Marker>(&world, root), None);
    }

    #[test]
    fn dragging_a_number_input_edits_a_map_value() {
        #[derive(Component, Reflect, Debug, Default)]
        #[reflect(Component)]
        struct Weights(HashMap<String, f32>);

        let mut app = test_app();
        app.add_plugins(bevy_text::TextPlugin);
        app.register_type::<Weights>();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let subject = app
            .world_mut()
            .spawn(Weights(
                [("a \"b\"".to_string(), 1.0)].into_iter().collect(),
            ))
            .id();
        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(subject);
        app.update();
        let input = field_widget(&mut app, "0[\"a \\\"b\\\"\"]");

        app.world_mut().trigger(ValueChange {
            source: input,
            value: 2.5_f32,
            is_final: true,
        });
        app.update();
        settle(&mut app);

        assert_eq!(
            app.world()
                .get::<Weights>(subject)
                .unwrap()
                .0
                .get("a \"b\""),
            Some(&2.5)
        );
        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(2.5))
        );
    }

    #[test]
    fn clicking_a_checkbox_edits_the_field() {
        let (mut app, subject) = widget_app();
        let checkbox = field_widget(&mut app, "enabled");

        for value in [true, false, true] {
            app.world_mut().trigger(ValueChange {
                source: checkbox,
                value,
                is_final: true,
            });
            app.update();
            assert_eq!(app.world().get::<Subject>(subject).unwrap().enabled, value);
            assert_eq!(app.world().entity(checkbox).contains::<Checked>(), value);
        }

        settle(&mut app);
        assert!(app.world().get::<Subject>(subject).unwrap().enabled);
        assert!(app.world().entity(checkbox).contains::<Checked>());
    }

    #[test]
    fn typing_into_a_text_input_edits_the_field() {
        let (mut app, subject) = widget_app();
        let input = field_widget(&mut app, "name");
        settle(&mut app);

        app.world_mut()
            .get_mut::<EditableText>(input)
            .unwrap()
            .queue_edit(TextEdit::Insert("!".into()));
        app.update();
        let typed = app
            .world()
            .get::<EditableText>(input)
            .unwrap()
            .value()
            .to_string();
        assert_ne!(typed, "start");
        assert_eq!(app.world().get::<Subject>(subject).unwrap().name, typed);

        settle(&mut app);
        assert_eq!(app.world().get::<Subject>(subject).unwrap().name, typed);
        assert_eq!(
            app.world()
                .get::<EditableText>(input)
                .unwrap()
                .value()
                .to_string(),
            typed
        );
    }

    fn option_rows(app: &App, select: Entity) -> Vec<(Entity, usize, bool)> {
        let mut rows = Vec::new();
        let mut stack = alloc::vec![select];
        while let Some(entity) = stack.pop() {
            let entity_ref = app.world().entity(entity);
            if let Some(index) = entity_ref.get::<OptionIndex>() {
                rows.push((entity, index.0, entity_ref.contains::<bevy_ui::Selected>()));
            }
            if let Some(children) = entity_ref.get::<Children>() {
                stack.extend(children.iter().copied());
            }
        }
        rows.sort_by_key(|(_, index, _)| *index);
        rows
    }

    #[test]
    fn choosing_a_select_option_edits_the_field() {
        let (mut app, subject) = widget_app();
        let select = field_widget(&mut app, "mode");
        let listbox = descendant_with::<bevy_ui_widgets::ListBox>(app.world(), select).unwrap();
        let running = option_rows(&app, select)[1].0;

        app.world_mut().trigger(ValueChange {
            source: listbox,
            value: running,
            is_final: true,
        });
        app.update();
        assert_eq!(
            app.world().get::<Subject>(subject).unwrap().mode,
            Mode::Running
        );

        settle(&mut app);
        assert_eq!(
            app.world().get::<Subject>(subject).unwrap().mode,
            Mode::Running
        );
        let select = field_widget(&mut app, "mode");
        let selected: Vec<usize> = option_rows(&app, select)
            .into_iter()
            .filter(|(_, _, selected)| *selected)
            .map(|(_, index, _)| index)
            .collect();
        assert_eq!(selected, alloc::vec![1]);
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[component(immutable)]
    #[reflect(Component, Default)]
    struct Frozen {
        flag: bool,
        mode: Mode,
    }

    #[derive(Component, Reflect, Debug, Default, PartialEq)]
    #[reflect(Component, Default)]
    enum Level {
        #[default]
        Low,
        High,
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component)]
    struct Lookup(HashMap<String, f32>);

    fn trigger_change<T: Send + Sync + 'static>(app: &mut App, source: Entity, value: T) {
        app.world_mut().trigger(ValueChange {
            source,
            value,
            is_final: true,
        });
        app.world_mut().flush();
        app.update();
    }

    #[test]
    fn keeps_immutable_components_read_only() {
        let mut app = test_app();
        app.register_type::<Frozen>();
        let entity = inspect(&mut app, Frozen::default());
        let component = app.world().component_id::<Frozen>().unwrap();

        let index = app.world().resource::<DetailsIndex>();
        assert!(!index.is_empty());
        assert!(index
            .fields
            .values()
            .all(|widget| matches!(widget.value, FieldValue::Label(_))));
        let mut fields = app.world_mut().query::<&InspectorField>();
        assert!(fields
            .iter(app.world())
            .all(|field| field.component != component));

        edit_component::<Frozen>(&mut app, entity, "flag", FieldValue::Bool(true));
        refresh(&mut app);
        assert!(!app.world().get::<Frozen>(entity).unwrap().flag);
    }

    #[test]
    fn edits_the_entity_the_panel_was_built_for() {
        let (mut app, first) = widget_app();
        let input = field_widget(&mut app, "scale");
        let second = app
            .world_mut()
            .spawn(Subject {
                scale: 1.0,
                ..Default::default()
            })
            .id();

        app.world_mut().resource_mut::<InspectorSelection>().0 = Some(second);
        trigger_change(&mut app, input, 3.0_f32);

        assert_eq!(app.world().get::<Subject>(first).unwrap().scale, 3.0);
        assert_eq!(app.world().get::<Subject>(second).unwrap().scale, 1.0);
    }

    #[test]
    fn does_not_rewrite_a_nan_field_on_each_sync() {
        let mut app = test_app();
        inspect(
            &mut app,
            Subject {
                scale: f32::NAN,
                ..Default::default()
            },
        );
        let input = widget_at(&app, "scale").unwrap();
        let changed = |app: &App| {
            app.world()
                .entity(input)
                .get_ref::<NumericValue>()
                .unwrap()
                .last_changed()
        };
        let before = changed(&app);

        settle(&mut app);

        assert_eq!(changed(&app), before);
    }

    #[test]
    fn rejects_non_finite_numbers() {
        let (mut app, entity) = kinds_app();

        for value in [
            NumericValue::F32(f32::NAN),
            NumericValue::F32(f32::INFINITY),
            NumericValue::F64(1e300),
        ] {
            edit_kinds(&mut app, entity, "single", FieldValue::Number(value));
        }
        edit_kinds(
            &mut app,
            entity,
            "double",
            FieldValue::Number(NumericValue::F64(f64::NEG_INFINITY)),
        );

        let kinds = app.world().get::<Kinds>(entity).unwrap();
        assert_eq!(kinds.single, 0.0);
        assert_eq!(kinds.double, 0.0);
    }

    #[test]
    fn only_successful_edits_mark_the_component_changed() {
        let (mut app, entity) = kinds_app();
        let changed = |app: &App| {
            app.world()
                .entity(entity)
                .get_ref::<Kinds>()
                .unwrap()
                .last_changed()
        };
        app.world_mut().increment_change_tick();
        let before = changed(&app);

        edit_kinds(&mut app, entity, "byte", int(300));
        edit_kinds(&mut app, entity, "missing", int(1));
        assert_eq!(changed(&app), before);

        app.world_mut().increment_change_tick();
        edit_kinds(&mut app, entity, "byte", int(12));
        assert_ne!(changed(&app), before);
    }

    #[test]
    fn only_rejected_edits_force_a_sync() {
        let (mut app, entity) = kinds_app();
        let elapsed = |app: &mut App| {
            let mut sync = app.world_mut().resource_mut::<DetailsPanelSync>();
            let elapsed = sync.timer.elapsed();
            sync.timer.reset();
            elapsed
        };
        elapsed(&mut app);

        let byte = |value| FieldValue::Number(NumericValue::I32(value));
        edit_kinds(&mut app, entity, "byte", byte(12));
        assert_eq!(elapsed(&mut app), Duration::ZERO);

        edit_kinds(&mut app, entity, "byte", byte(300));
        assert_ne!(elapsed(&mut app), Duration::ZERO);
    }

    #[test]
    fn dragging_an_integer_to_the_same_value_does_not_mark_it_changed() {
        let (mut app, entity) = kinds_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        select(&mut app, Some(entity));
        let input = field_widget(&mut app, "byte");
        let changed = |app: &App| {
            app.world()
                .entity(entity)
                .get_ref::<Kinds>()
                .unwrap()
                .last_changed()
        };

        trigger_change(&mut app, input, 7_i32);
        assert_eq!(app.world().get::<Kinds>(entity).unwrap().byte, 7);
        let before = changed(&app);

        for _ in 0..3 {
            app.world_mut().increment_change_tick();
            trigger_change(&mut app, input, 7_i32);
        }
        assert_eq!(changed(&app), before);
    }

    #[test]
    fn an_unchanged_text_edit_keeps_the_string_and_change_tick() {
        let (mut app, entity) = kinds_app();
        edit_kinds(&mut app, entity, "text", FieldValue::Text("same".into()));
        let state = |app: &App| {
            let kinds = app.world().entity(entity).get_ref::<Kinds>().unwrap();
            (kinds.text.as_ptr(), kinds.last_changed())
        };
        let before = state(&app);

        app.world_mut().increment_change_tick();
        edit_kinds(&mut app, entity, "text", FieldValue::Text("same".into()));
        assert_eq!(state(&app), before);
    }

    #[test]
    fn an_unchanged_variant_choice_does_not_mark_it_changed() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Subject::default()).id();
        let changed = |app: &App| {
            app.world()
                .entity(subject)
                .get_ref::<Subject>()
                .unwrap()
                .last_changed()
        };
        app.world_mut().increment_change_tick();
        let before = changed(&app);

        edit(&mut app, subject, "mode", variant(&["Idle", "Running"], 0));
        assert_eq!(changed(&app), before);

        edit(&mut app, subject, "mode", variant(&["Idle", "Running"], 1));
        assert_ne!(changed(&app), before);
    }

    #[test]
    fn limits_integer_inputs_to_their_type() {
        fn limit(app: &mut App, path: &str) -> Option<NumericRange> {
            let input = field_widget(app, path);
            app.world()
                .get::<HardLimit>(input)
                .map(|limit| limit.0.clone())
        }

        let (mut app, entity) = kinds_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        select(&mut app, Some(entity));

        assert_eq!(limit(&mut app, "byte"), Some(NumericRange::I32(0..=255)));
        assert_eq!(limit(&mut app, "tiny"), Some(NumericRange::I32(-128..=127)));
        assert_eq!(
            limit(&mut app, "word"),
            Some(NumericRange::I64(0..=i64::from(u32::MAX)))
        );
        assert_eq!(
            limit(&mut app, "size"),
            Some(NumericRange::I64(0..=i64::MAX))
        );
        assert_eq!(limit(&mut app, "int"), None);
        assert_eq!(limit(&mut app, "big"), None);
        assert_eq!(limit(&mut app, "single"), None);
    }

    #[test]
    fn reverts_the_widget_after_a_rejected_edit() {
        let (mut app, entity) = kinds_app();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        select(&mut app, Some(entity));
        let byte = field_widget(&mut app, "byte");

        trigger_change(&mut app, byte, 300_i32);
        app.update();

        assert_eq!(app.world().get::<Kinds>(entity).unwrap().byte, 0);
        assert_eq!(
            app.world().get::<NumericValue>(byte),
            Some(&NumericValue::I32(0))
        );
    }

    #[test]
    fn applies_an_edit_back_to_a_value_the_game_replaced() {
        let (mut app, subject) = widget_app();
        let input = field_widget(&mut app, "scale");

        app.world_mut().get_mut::<Subject>(subject).unwrap().scale = 5.0;
        trigger_change(&mut app, input, 1.0_f32);

        assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 1.0);
    }

    #[test]
    fn refreshes_external_changes_on_the_timer() {
        let (mut app, subject) = widget_app();
        app.insert_resource(bevy_time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(100),
        ));
        let input = field_widget(&mut app, "scale");
        app.update();

        app.world_mut().get_mut::<Subject>(subject).unwrap().scale = 4.0;
        for _ in 0..4 {
            app.update();
        }

        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(4.0))
        );
    }

    #[test]
    fn edits_a_root_level_unit_enum_component() {
        let mut app = test_app();
        app.register_type::<Level>();
        let entity = inspect(&mut app, Level::Low);
        let component = app.world().component_id::<Level>().unwrap();
        assert_eq!(tracked_paths(&app), alloc::vec![String::new()]);

        edit_component::<Level>(&mut app, entity, "", variant(&["Low", "High"], 1));
        app.update();

        assert_eq!(app.world().get::<Level>(entity), Some(&Level::High));
        let index = app.world().resource::<DetailsIndex>();
        assert_eq!(
            index.fields[&(component, String::new())].value,
            variant(&["Low", "High"], 1)
        );
    }

    #[test]
    fn respawns_the_group_when_an_edited_map_key_is_removed() {
        let mut app = test_app();
        app.register_type::<Lookup>();
        let entity = inspect(
            &mut app,
            Lookup(
                [("a".to_string(), 1.0), ("b".to_string(), 2.0)]
                    .into_iter()
                    .collect(),
            ),
        );
        let component = app.world().component_id::<Lookup>().unwrap();
        let group = app
            .world()
            .resource::<DetailsIndex>()
            .group(component)
            .unwrap();
        let input = field_widget(&mut app, "0[\"a\"]");

        app.world_mut()
            .get_mut::<Lookup>(entity)
            .unwrap()
            .0
            .remove("a");
        trigger_change(&mut app, input, 3.0_f32);

        let lookup = &app.world().get::<Lookup>(entity).unwrap().0;
        assert_eq!(lookup.get("a"), None);
        assert_eq!(lookup.get("b"), Some(&2.0));
        let index = app.world().resource::<DetailsIndex>();
        assert_ne!(index.group(component), Some(group));
        assert_eq!(index.widget(component, "0[\"a\"]"), None);
        assert!(index.widget(component, "0[\"b\"]").is_some());
    }

    #[test]
    fn ignores_edits_outside_local_mode() {
        let mut app = test_app();
        app.world_mut().remove_resource::<InspectorSource>();
        let subject = inspect(
            &mut app,
            Subject {
                scale: 1.0,
                ..Default::default()
            },
        );

        edit(
            &mut app,
            subject,
            "scale",
            FieldValue::Number(NumericValue::F32(2.0)),
        );

        assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 1.0);
        let mut fields = app.world_mut().query::<&InspectorField>();
        assert_eq!(fields.iter(app.world()).count(), 0);
    }
}
