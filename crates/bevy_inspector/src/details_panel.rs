//! A panel showing the components of the selected entity, and the systems that keep it in sync.

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};

use bevy_color::{Color, LinearRgba, Srgba};
use bevy_dev_tools::inspection::{
    component_inspection::ComponentInspectionSettings, entity_inspection::EntityInspectionSettings,
    extension_methods::WorldInspectionExtensionTrait,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    name::Name,
    observer::On,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    world::World,
};
use bevy_feathers::{
    containers::{group, group_body, group_header, subpane, subpane_body, subpane_header},
    controls::{
        list_rows_from_strings, ColorSwatchValue, FeathersCheckbox, FeathersColorSwatch,
        FeathersDisclosureToggle, FeathersNumberInput, FeathersScrollbar, FeathersSelect,
        FeathersTextInput, FeathersTextInputContainer, OptionIndex, ScrollbarGutter,
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
    percent, px, widget::Text, AlignItems, Checked, Display, FlexDirection, InteractionDisabled,
    Node, Overflow, PositionType, UiRect,
};
use bevy_ui_widgets::{ControlOrientation, NumericValue, ScrollArea, ValueChange};
use bevy_utils::prelude::ShortName;

use crate::{entity_tree::InspectorUi, InspectorSelection, InspectorSource};

/// The deepest nesting level whose fields are rendered.
/// This bounds how many widgets one selection spawns: without a limit, deeply nested values
/// would build thousands of rows on every rebuild.
const MAX_DEPTH: usize = 4;
/// The number of items rendered for a list, array, map or set.
/// This bounds how many widgets one selection spawns: without a limit, a large collection such
/// as a mesh's vertex data would build thousands of rows on every rebuild.
const MAX_ITEMS: usize = 16;
/// The horizontal indent of a field row per nesting level, in logical pixels.
const INDENT: f32 = 12.0;
/// The width of a field row's label column, in logical pixels, at zero depth.
///
/// Deeper rows shrink their label column by `depth * INDENT` so that widgets line up at the
/// same x position regardless of nesting, since the row's own left padding already grows by
/// `depth * INDENT`.
const FIELD_LABEL_WIDTH: f32 = 96.0;
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

/// The short name of the component a group header belongs to.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorDetailsComponent(pub String);

/// The kind of widget a field value is rendered with.
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
    /// The kind of widget this value is rendered with.
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
#[derive(EntityEvent, Debug, Clone)]
pub struct FieldEdit {
    /// The inspected entity holding the component.
    #[event_target]
    pub entity: Entity,
    /// The full type path of the component.
    pub component: String,
    /// The path of the field within the component, in [`bevy_reflect::GetPath`] syntax.
    pub path: String,
    /// The value to write into the field.
    pub value: FieldValue,
}

/// The component field that a details panel widget edits.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorField {
    /// The short name of the component, as used to key [`DetailsIndex`].
    pub component: String,
    /// The full type path of the component.
    pub type_path: String,
    /// The path of the field within the component.
    pub path: String,
}

/// A single field of a component, as one row of the details panel.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEntry {
    /// The path of the field within its component, in [`bevy_reflect::GetPath`] syntax.
    pub path: String,
    /// The name shown at the start of the row.
    pub label: String,
    /// The nesting level of the field, used to indent the row.
    pub depth: usize,
    /// The value of the field.
    pub value: FieldValue,
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
struct ComponentDetails {
    name: String,
    type_path: String,
    memory: String,
    fields: Vec<FieldEntry>,
}

/// Maps the fields shown by the details panel to the widgets displaying them.
#[derive(Resource, Debug, Default)]
pub struct DetailsIndex {
    fields: HashMap<(String, String), FieldWidget>,
    body: Option<Entity>,
    selection: Option<Entity>,
    signature: Vec<String>,
    collapsed: Vec<String>,
}

impl DetailsIndex {
    /// The widget displaying the field at `path` of `component`, if one exists.
    pub fn widget(&self, component: &str, path: &str) -> Option<Entity> {
        self.fields
            .get(&(component.to_string(), path.to_string()))
            .map(|widget| widget.entity)
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

/// The components whose field lists are collapsed, by short name.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DetailsCollapsed(pub HashSet<String>);

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
            height: percent(100),
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
                flex_grow: 1.0,
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
        collapsed.0.insert(component.0.clone());
    }
    sync.set_dirty();
}

/// Records a widget's new value, writes it back into the widget and emits a [`FieldEdit`], unless
/// the value is already current.
///
/// The feathers controls do not update themselves when the user changes them, so the widget only
/// shows the new value once it is written back.
fn emit_field_edit(
    widget: Entity,
    value: impl FnOnce(&FieldValue) -> Option<FieldValue>,
    fields: &Query<&InspectorField>,
    selection: &InspectorSelection,
    index: &mut DetailsIndex,
    sync: &mut DetailsPanelSync,
    commands: &mut Commands,
) {
    let Ok(field) = fields.get(widget) else {
        return;
    };
    let Some(entity) = selection.0 else {
        return;
    };
    let Some(stored) = index
        .fields
        .get_mut(&(field.component.clone(), field.path.clone()))
    else {
        return;
    };
    let Some(value) = value(&stored.value) else {
        return;
    };
    if stored.value == value {
        return;
    }

    stored.value = value.clone();
    let displayed = stored.clone();
    sync.set_dirty();
    commands.queue(move |world: &mut World| apply_value(world, &displayed, &displayed.value));
    commands.trigger(FieldEdit {
        entity,
        component: field.type_path.clone(),
        path: field.path.clone(),
        value,
    });
}

/// Observer that turns a details panel checkbox change into a [`FieldEdit`].
pub fn inspector_field_bool_changed(
    change: On<ValueChange<bool>>,
    fields: Query<&InspectorField>,
    selection: Res<InspectorSelection>,
    mut index: ResMut<DetailsIndex>,
    mut sync: ResMut<DetailsPanelSync>,
    mut commands: Commands,
) {
    let value = FieldValue::Bool(change.value);
    emit_field_edit(
        change.source,
        |_| Some(value),
        &fields,
        &selection,
        &mut index,
        &mut sync,
        &mut commands,
    );
}

macro_rules! number_changed_observer {
    ($name:ident, $number:ty, $variant:ident) => {
        /// Observer that turns a details panel number input change into a [`FieldEdit`].
        pub fn $name(
            change: On<ValueChange<$number>>,
            fields: Query<&InspectorField>,
            selection: Res<InspectorSelection>,
            mut index: ResMut<DetailsIndex>,
            mut sync: ResMut<DetailsPanelSync>,
            mut commands: Commands,
        ) {
            let value = FieldValue::Number(NumericValue::$variant(change.value));
            emit_field_edit(
                change.source,
                |_| Some(value),
                &fields,
                &selection,
                &mut index,
                &mut sync,
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
pub fn inspector_field_text_changed(
    change: On<TextEditChange>,
    texts: Query<&EditableText>,
    fields: Query<&InspectorField>,
    selection: Res<InspectorSelection>,
    mut index: ResMut<DetailsIndex>,
    mut sync: ResMut<DetailsPanelSync>,
    mut commands: Commands,
) {
    let widget = change.event_target();
    let Ok(editable) = texts.get(widget) else {
        return;
    };
    let text = editable.value().to_string();
    emit_field_edit(
        widget,
        |_| Some(FieldValue::Text(text)),
        &fields,
        &selection,
        &mut index,
        &mut sync,
        &mut commands,
    );
}

/// Observer that turns a details panel variant selection into a [`FieldEdit`].
pub fn inspector_field_variant_changed(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    fields: Query<&InspectorField>,
    selection: Res<InspectorSelection>,
    mut index: ResMut<DetailsIndex>,
    mut sync: ResMut<DetailsPanelSync>,
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
        &selection,
        &mut index,
        &mut sync,
        &mut commands,
    );
}

/// Observer that writes a [`FieldEdit`] into the component of the inspected entity.
pub fn apply_field_edit(edit: On<FieldEdit>, mut commands: Commands) {
    let edit = edit.event().clone();
    commands.queue(move |world: &mut World| write_field_edit(world, &edit));
}

fn write_field_edit(world: &mut World, edit: &FieldEdit) {
    if world.get_resource::<InspectorSource>() != Some(&InspectorSource::Local) {
        return;
    }
    let Some(registry) = world.get_resource::<AppTypeRegistry>().cloned() else {
        return;
    };
    let registry = registry.read();
    let Some(reflect_component) = registry
        .get_with_type_path(&edit.component)
        .and_then(|registration| registration.data::<ReflectComponent>())
    else {
        warn!("the inspector cannot edit `{}`", edit.component);
        return;
    };
    if world.get_entity(edit.entity).is_err() {
        return;
    }

    let Some(mut component) = reflect_component.reflect_mut(world.entity_mut(edit.entity)) else {
        warn!("the inspector cannot edit `{}`", edit.component);
        return;
    };

    let field = if edit.path.is_empty() {
        Ok(component.as_partial_reflect_mut())
    } else {
        component.reflect_path_mut(edit.path.as_str())
    };
    let written = match field {
        Ok(field) => write_field_value(field, &edit.value),
        Err(_) => false,
    };
    if !written {
        warn!(
            "the inspector cannot edit `{}` at `{}`",
            edit.component, edit.path
        );
    }
}

fn write_field_value(field: &mut dyn PartialReflect, value: &FieldValue) -> bool {
    match value {
        FieldValue::Bool(new) => match field.try_downcast_mut::<bool>() {
            Some(target) => {
                *target = *new;
                true
            }
            None => false,
        },
        FieldValue::Number(number) => write_number_field(field, *number),
        FieldValue::Text(text) => write_text_field(field, text),
        FieldValue::Variant { variants, selected } => variants
            .get(*selected)
            .is_some_and(|name| write_variant_field(field, name)),
        FieldValue::Color(_) | FieldValue::Label(_) => false,
    }
}

fn write_text_field(field: &mut dyn PartialReflect, text: &str) -> bool {
    if let Some(target) = field.try_downcast_mut::<String>() {
        *target = text.to_string();
        return true;
    }
    if let Some(target) = field.try_downcast_mut::<Cow<'static, str>>() {
        *target = Cow::Owned(text.to_string());
        return true;
    }
    if let Some(target) = field.try_downcast_mut::<char>() {
        let mut chars = text.chars();
        if let (Some(new), None) = (chars.next(), chars.next()) {
            *target = new;
            return true;
        }
    }
    false
}

/// Writes a number into a numeric field, rejecting integers outside the field's range.
fn write_number_field(field: &mut dyn PartialReflect, value: NumericValue) -> bool {
    let float = match value {
        NumericValue::F32(value) => value as f64,
        NumericValue::F64(value) => value,
        NumericValue::I32(value) => value as f64,
        NumericValue::I64(value) => value as f64,
    };
    if let Some(target) = field.try_downcast_mut::<f32>() {
        *target = match value {
            NumericValue::F32(value) => value,
            _ => float as f32,
        };
        return true;
    }
    if let Some(target) = field.try_downcast_mut::<f64>() {
        *target = float;
        return true;
    }

    let integer = match value {
        NumericValue::I32(value) => i128::from(value),
        NumericValue::I64(value) => i128::from(value),
        _ if float.is_finite() && float.fract() == 0.0 => float as i128,
        _ => return false,
    };

    if let Some(target) = field.try_downcast_mut::<i128>() {
        *target = integer;
        return true;
    }

    macro_rules! write_as {
        ($($type:ty),*) => {
            $(
                if let Some(target) = field.try_downcast_mut::<$type>() {
                    let Ok(value) = <$type>::try_from(integer) else {
                        return false;
                    };
                    *target = value;
                    return true;
                }
            )*
        };
    }

    write_as!(i8, i16, i32, i64, isize, u8, u16, u32, u64, u128, usize);
    false
}

/// Switches a unit-only enum field to the variant `name`.
fn write_variant_field(field: &mut dyn PartialReflect, name: &str) -> bool {
    let ReflectRef::Enum(current) = field.reflect_ref() else {
        return false;
    };
    if current.variant_name() == name {
        return true;
    }
    let Some(TypeInfo::Enum(info)) = field.get_represented_type_info() else {
        return false;
    };
    if info.variant(name).is_none()
        || info
            .iter()
            .any(|variant| variant.variant_type() != VariantType::Unit)
    {
        return false;
    }
    field
        .try_apply(&DynamicEnum::new(name, DynamicVariant::Unit))
        .is_ok()
}

/// Rebuilds or refreshes the details panel so that it matches the selected entity.
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

    let components = inspect_components(world, selection);
    let signature: Vec<String> = components
        .iter()
        .map(|component| component.name.clone())
        .collect();
    let mut collapsed: Vec<String> = world
        .resource::<DetailsCollapsed>()
        .0
        .iter()
        .cloned()
        .collect();
    collapsed.sort_unstable();

    let index = world.resource::<DetailsIndex>();
    let rebuild = selection_changed
        || index.body != Some(body)
        || index.signature != signature
        || index.collapsed != collapsed;

    if !rebuild && update_in_place(world, &components) {
        return;
    }

    rebuild_body(world, body, selection, &components, signature, collapsed);
}

fn find_body(world: &mut World) -> Option<Entity> {
    world
        .iter_entities()
        .find(bevy_ecs::world::EntityRef::contains::<InspectorDetailsBody>)
        .map(|entity| entity.id())
}

fn inspect_components(world: &World, selection: Option<Entity>) -> Vec<ComponentDetails> {
    let Some(entity) = selection else {
        return Vec::new();
    };

    let settings = EntityInspectionSettings {
        include_components: true,
        component_settings: ComponentInspectionSettings {
            store_reflected_value: true,
            ..Default::default()
        },
    };

    let Ok(inspection) = world.inspect(entity, settings) else {
        return Vec::new();
    };
    let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
        return Vec::new();
    };
    let registry = registry.read();

    let mut components: Vec<ComponentDetails> = inspection
        .components
        .unwrap_or_default()
        .iter()
        .map(|component| ComponentDetails {
            name: crate::component_short_name(world, component.component_id),
            type_path: component
                .reflected_value
                .as_deref()
                .and_then(PartialReflect::get_represented_type_info)
                .map(|info| info.type_path().to_string())
                .unwrap_or_default(),
            memory: component.memory_size.to_string(),
            fields: component
                .reflected_value
                .as_deref()
                .map(|value| field_entries(value, &registry))
                .unwrap_or_default(),
        })
        .collect();
    components.sort_by(|left, right| left.name.cmp(&right.name));
    components
}

fn update_in_place(world: &mut World, components: &[ComponentDetails]) -> bool {
    let mut updates = Vec::new();
    {
        let index = world.resource::<DetailsIndex>();
        for component in components {
            if index.collapsed.contains(&component.name) {
                continue;
            }
            for entry in &component.fields {
                let key = (component.name.clone(), entry.path.clone());
                let Some(widget) = index.fields.get(&key) else {
                    return false;
                };
                if widget.value == entry.value {
                    continue;
                }
                if widget.value.kind() != entry.value.kind()
                    || matches!(entry.value, FieldValue::Variant { .. })
                {
                    return false;
                }
                updates.push((key, widget.clone(), entry.value.clone()));
            }
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

fn rebuild_body(
    world: &mut World,
    body: Entity,
    selection: Option<Entity>,
    components: &[ComponentDetails],
    signature: Vec<String>,
    collapsed: Vec<String>,
) {
    let children: Vec<Entity> = world
        .get::<Children>(body)
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();
    for child in children {
        despawn_leaves_first(world, child);
    }

    let editable = world.get_resource::<InspectorSource>() == Some(&InspectorSource::Local);
    let mut fields = HashMap::new();
    if components.is_empty() {
        spawn_child_scene(world, body, caption("No entity selected"));
    } else {
        for component in components {
            let expanded = !collapsed.contains(&component.name);
            spawn_group(world, body, component, expanded, editable, &mut fields);
        }
    }

    let mut index = world.resource_mut::<DetailsIndex>();
    index.fields = fields;
    index.body = Some(body);
    index.selection = selection;
    index.signature = signature;
    index.collapsed = collapsed;
}

fn spawn_group(
    world: &mut World,
    body: Entity,
    component: &ComponentDetails,
    expanded: bool,
    editable: bool,
    fields: &mut HashMap<(String, String), FieldWidget>,
) {
    let Some(group) = spawn_child_scene(
        world,
        body,
        component_group(component.name.clone(), component.memory.clone()),
    ) else {
        return;
    };

    if let Some(toggle) = descendant_with::<FeathersDisclosureToggle>(world, group) {
        let mut toggle = world.entity_mut(toggle);
        toggle.insert(InspectorDetailsComponent(component.name.clone()));
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

    for entry in &component.fields {
        if let Some(widget) = spawn_field_row(world, container, component, entry, editable) {
            fields.insert((component.name.clone(), entry.path.clone()), widget);
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
    editable: bool,
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

    let label = spawn_child_scene(world, row, caption(entry.label.clone()))?;
    let label_width = (FIELD_LABEL_WIDTH - entry.depth as f32 * INDENT).max(0.0);
    world.entity_mut(label).insert(Node {
        min_width: px(label_width),
        width: px(label_width),
        flex_shrink: 0.0,
        overflow: Overflow::clip_x(),
        ..Default::default()
    });

    let widget = spawn_widget(world, row, &entry.value)?;
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
            component: component.name.clone(),
            type_path: component.type_path.clone(),
            path: entry.path.clone(),
        });
        if !editable {
            entity.insert(InteractionDisabled);
        }
    }

    Some(widget)
}

fn spawn_widget(world: &mut World, row: Entity, value: &FieldValue) -> Option<FieldWidget> {
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
            set_text(world, widget.text, color_text(*color));
        }
        FieldValue::Variant { .. } => {}
        FieldValue::Label(text) if text.is_empty() => {}
        FieldValue::Label(text) => set_text(world, Some(widget.entity), text.clone()),
    }
}

fn replace_text(world: &mut World, entity: Option<Entity>, text: &str) {
    if let Some(mut editable) = entity.and_then(|entity| world.get_mut::<EditableText>(entity))
        && editable.value() != text
    {
        editable.queue_edit(TextEdit::SelectAll);
        editable.queue_edit(TextEdit::Insert(text.into()));
    }
}

fn set_text(world: &mut World, entity: Option<Entity>, value: String) {
    let Some(entity) = entity else {
        return;
    };
    if let Some(mut text) = world.get_mut::<Text>(entity)
        && text.0 != value
    {
        text.0 = value;
    }
}

fn color_text(color: Color) -> String {
    Srgba::from(color).to_hex()
}

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

/// Despawns a subtree from its leaves upwards.
///
/// Widgets whose removal observers reach for a descendant, such as the feathers number input,
/// queue commands for entities the recursive despawn is about to remove; despawning the leaves
/// first leaves those observers with nothing to find.
fn despawn_leaves_first(world: &mut World, root: Entity) {
    let mut order = Vec::new();
    let mut stack = alloc::vec![root];
    while let Some(entity) = stack.pop() {
        let Ok(entity_ref) = world.get_entity(entity) else {
            continue;
        };
        if let Some(children) = entity_ref.get::<Children>() {
            stack.extend(children.iter().copied());
        }
        order.push(entity);
    }

    for entity in order.into_iter().rev() {
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
}

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
            stack.extend(children.iter().copied());
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
    let (value, path) = unwrap_newtype(value, String::new());
    if let Some(scalar) = scalar_value(value) {
        walk.push(path, "value".to_string(), 0, scalar);
    } else if let Some(variant) = variant_value(value) {
        walk.push(path.clone(), "variant".to_string(), 0, variant);
        walk.children(value, &path, 0, true);
    } else if summary(value).is_some() {
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
        let (value, path) = unwrap_newtype(value, path);
        let lock = |value| if editable { value } else { read_only(value) };

        if depth >= MAX_DEPTH {
            self.push(path, label, depth, FieldValue::Label("...".to_string()));
            return;
        }

        if let Some(scalar) = scalar_value(value) {
            self.push(path, label, depth, lock(scalar));
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
                    let path = format!("{prefix}[{index}]");
                    self.field(item, path, display_value(key), depth, false);
                }
            }
            ReflectRef::Set(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = format!("{prefix}[{index}]");
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
fn read_only(value: FieldValue) -> FieldValue {
    FieldValue::Label(match value {
        FieldValue::Bool(value) => value.to_string(),
        FieldValue::Number(NumericValue::F32(value)) => value.to_string(),
        FieldValue::Number(NumericValue::F64(value)) => value.to_string(),
        FieldValue::Number(NumericValue::I32(value)) => value.to_string(),
        FieldValue::Number(NumericValue::I64(value)) => value.to_string(),
        FieldValue::Text(text) | FieldValue::Label(text) => text,
        FieldValue::Color(color) => color_text(color),
        FieldValue::Variant { variants, selected } => {
            variants.get(selected).cloned().unwrap_or_default()
        }
    })
}

/// Unwraps single-field tuple structs so that newtypes do not add a nesting level of their own.
fn unwrap_newtype(value: &dyn PartialReflect, mut path: String) -> (&dyn PartialReflect, String) {
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
    use crate::{entity_tree::InspectorTreeView, InspectorPlugin};
    use alloc::collections::VecDeque;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::component::Component;
    use bevy_platform::sync::Arc;
    use bevy_reflect::TypePath;

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

        let index = app.world().resource::<DetailsIndex>();
        let component = index.fields.keys().next().unwrap().0.clone();
        let scale = index.widget(&component, "scale").unwrap();
        let enabled = index.widget(&component, "enabled").unwrap();
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
        assert_eq!(index.widget(&component, "scale"), Some(scale));
        assert_eq!(index.widget(&component, "enabled"), Some(enabled));
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
        assert_eq!(
            entry(&entries, "map[0]").value,
            FieldValue::Label("1".to_string())
        );
        assert_eq!(
            entry(&entries, "set[0]").value,
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
    fn edits_chars_and_rejects_other_lengths() {
        let (mut app, entity) = kinds_app();

        edit_kinds(
            &mut app,
            entity,
            "letter",
            FieldValue::Text("q".to_string()),
        );
        edit_kinds(
            &mut app,
            entity,
            "letter",
            FieldValue::Text("ab".to_string()),
        );
        edit_kinds(&mut app, entity, "letter", FieldValue::Text(String::new()));

        assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'q');
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
}
