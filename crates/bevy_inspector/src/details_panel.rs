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
    hierarchy::{ChildOf, Children},
    name::Name,
    observer::On,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Query, ResMut},
    world::World,
};
use bevy_feathers::{
    containers::{group, group_body, group_header, subpane, subpane_body, subpane_header},
    controls::{
        list_rows_from_strings, ColorSwatchValue, FeathersCheckbox, FeathersColorSwatch,
        FeathersDisclosureToggle, FeathersNumberInput, FeathersScrollbar, FeathersSelect,
        FeathersTextInput, FeathersTextInputContainer, ScrollbarGutter,
    },
    display::caption,
    theme::ThemedText,
};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{
    enums::VariantType, prelude::ReflectDefault, PartialReflect, Reflect, ReflectRef, TypeInfo,
};
use bevy_scene::{bsn, on, Scene, WorldSceneExt};
use bevy_text::{EditableText, LineBreak, TextEdit, TextLayout};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{
    percent, px, widget::Text, AlignItems, Checked, Display, FlexDirection, InteractionDisabled,
    Node, Overflow, PositionType, UiRect,
};
use bevy_ui_widgets::{ControlOrientation, NumericValue, ScrollArea, ValueChange};
use bevy_utils::prelude::ShortName;

use crate::{entity_tree::InspectorUi, InspectorSelection};

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

/// Rebuilds or refreshes the details panel so that it matches the selected entity.
pub fn sync_details_panel(world: &mut World) {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();

    let run = {
        let mut sync = world.resource_mut::<DetailsPanelSync>();
        sync.timer.tick(delta);
        sync.timer.just_finished()
    };

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

    let mut components: Vec<ComponentDetails> = inspection
        .components
        .unwrap_or_default()
        .iter()
        .map(|component| ComponentDetails {
            name: crate::component_short_name(world, component.component_id),
            memory: component.memory_size.to_string(),
            fields: component
                .reflected_value
                .as_deref()
                .map(field_entries)
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
                if matches!(entry.value, FieldValue::Variant { .. }) {
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

    let mut fields = HashMap::new();
    if components.is_empty() {
        spawn_child_scene(world, body, caption("No entity selected"));
    } else {
        for component in components {
            let expanded = !collapsed.contains(&component.name);
            spawn_group(world, body, component, expanded, &mut fields);
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
        if let Some(widget) = spawn_field_row(world, container, entry) {
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
                    InteractionDisabled
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
                    InteractionDisabled
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
            let container = spawn_child_scene(
                world,
                row,
                bsn! {
                    InspectorUi
                    @FeathersTextInputContainer
                    Node {
                        width: px(FIELD_WIDGET_WIDTH),
                        flex_grow: 0.0,
                    }
                    Children [
                        @FeathersTextInput
                        InteractionDisabled
                    ]
                },
            )?;
            let entity = descendant_with::<EditableText>(world, container)?;
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
                    InteractionDisabled
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
        FieldValue::Text(text) => {
            if let Some(mut editable) = world.get_mut::<EditableText>(widget.entity) {
                editable.queue_edit(TextEdit::SelectAll);
                editable.queue_edit(TextEdit::Insert(text.as_str().into()));
            }
        }
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
pub fn field_entries(value: &dyn PartialReflect) -> Vec<FieldEntry> {
    let mut entries = Vec::new();
    if let Some(name) = value.try_downcast_ref::<Name>() {
        entries.push(FieldEntry {
            path: String::new(),
            label: "value".to_string(),
            depth: 0,
            value: FieldValue::Label(name.as_str().to_string()),
        });
        return entries;
    }
    let (value, path) = unwrap_newtype(value, String::new());
    if let Some(scalar) = scalar_value(value) {
        entries.push(FieldEntry {
            path,
            label: "value".to_string(),
            depth: 0,
            value: scalar,
        });
    } else if summary(value).is_some() {
        walk_children(value, &path, 0, &mut entries);
    } else {
        entries.push(FieldEntry {
            path,
            label: "value".to_string(),
            depth: 0,
            value: FieldValue::Label(format_fallback(value)),
        });
    }
    entries
}

fn walk(
    value: &dyn PartialReflect,
    path: String,
    label: String,
    depth: usize,
    out: &mut Vec<FieldEntry>,
) {
    let (value, path) = unwrap_newtype(value, path);

    if depth >= MAX_DEPTH {
        out.push(FieldEntry {
            path,
            label,
            depth,
            value: FieldValue::Label("...".to_string()),
        });
        return;
    }

    if let Some(scalar) = scalar_value(value) {
        out.push(FieldEntry {
            path,
            label,
            depth,
            value: scalar,
        });
        return;
    }

    let Some(summary) = summary(value) else {
        out.push(FieldEntry {
            path,
            label,
            depth,
            value: FieldValue::Label(format_fallback(value)),
        });
        return;
    };

    out.push(FieldEntry {
        path: path.clone(),
        label,
        depth,
        value: FieldValue::Label(summary),
    });
    walk_children(value, &path, depth + 1, out);
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

fn walk_children(
    value: &dyn PartialReflect,
    prefix: &str,
    depth: usize,
    out: &mut Vec<FieldEntry>,
) {
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
                walk(field, join(prefix, &name), name, depth, out);
            }
        }
        ReflectRef::TupleStruct(value) => {
            for index in 0..value.field_len() {
                let Some(field) = value.field(index) else {
                    continue;
                };
                let name = index.to_string();
                walk(field, join(prefix, &name), name, depth, out);
            }
        }
        ReflectRef::Tuple(value) => {
            for index in 0..value.field_len() {
                let Some(field) = value.field(index) else {
                    continue;
                };
                let name = index.to_string();
                walk(field, join(prefix, &name), name, depth, out);
            }
        }
        ReflectRef::List(value) => {
            for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                walk(
                    item,
                    format!("{prefix}[{index}]"),
                    index.to_string(),
                    depth,
                    out,
                );
            }
        }
        ReflectRef::Array(value) => {
            for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                walk(
                    item,
                    format!("{prefix}[{index}]"),
                    index.to_string(),
                    depth,
                    out,
                );
            }
        }
        ReflectRef::Map(value) => {
            for (index, (key, item)) in value.iter().take(MAX_ITEMS).enumerate() {
                walk(
                    item,
                    format!("{prefix}[{index}]"),
                    display_value(key),
                    depth,
                    out,
                );
            }
        }
        ReflectRef::Set(value) => {
            for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                walk(
                    item,
                    format!("{prefix}[{index}]"),
                    index.to_string(),
                    depth,
                    out,
                );
            }
        }
        ReflectRef::Enum(value) => match value.variant_type() {
            VariantType::Unit => {}
            VariantType::Tuple => {
                for index in 0..value.field_len() {
                    let Some(field) = value.field_at(index) else {
                        continue;
                    };
                    let name = index.to_string();
                    walk(field, join(prefix, &name), name, depth, out);
                }
            }
            VariantType::Struct => {
                for index in 0..value.field_len() {
                    let Some(field) = value.field_at(index) else {
                        continue;
                    };
                    let name = value
                        .name_at(index)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string());
                    walk(field, join(prefix, &name), name, depth, out);
                }
            }
        },
        _ => {}
    }
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
    if let Some(value) = value.try_downcast_ref::<String>() {
        return Some(FieldValue::Text(value.clone()));
    }
    if let Some(value) = value.try_downcast_ref::<Cow<'static, str>>() {
        return Some(FieldValue::Text(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<&'static str>() {
        return Some(FieldValue::Text(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<Entity>() {
        return Some(FieldValue::Label(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<Color>() {
        return Some(FieldValue::Color(*value));
    }
    if let Some(value) = value.try_downcast_ref::<Srgba>() {
        return Some(FieldValue::Color(Color::from(*value)));
    }
    if let Some(value) = value.try_downcast_ref::<LinearRgba>() {
        return Some(FieldValue::Color(Color::from(*value)));
    }
    unit_enum_value(value)
}

fn integer_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    macro_rules! narrow {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(FieldValue::Number(NumericValue::I32(*value as i32)));
                }
            )*
        };
    }
    macro_rules! wide {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    let value = i64::try_from(*value).unwrap_or(i64::MAX);
                    return Some(FieldValue::Number(NumericValue::I64(value)));
                }
            )*
        };
    }

    narrow!(i8, i16, i32, u8, u16);
    wide!(i64, isize, u32, u64, usize);
    None
}

fn unit_enum_value(value: &dyn PartialReflect) -> Option<FieldValue> {
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

    let variants: Vec<String> = info
        .variant_names()
        .iter()
        .map(ToString::to_string)
        .collect();
    let selected = variants
        .iter()
        .position(|name| name == reflected.variant_name())?;
    Some(FieldValue::Variant { variants, selected })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entity_tree::InspectorTreeView, InspectorPlugin};
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::component::Component;
    use bevy_platform::sync::Arc;

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

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    struct Subject {
        enabled: bool,
        scale: f32,
        name: String,
        nested: Nested,
        values: Vec<u32>,
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
        };

        let entries = field_entries(&subject);

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
            ]
        );
        assert_eq!(entries[4].depth, 1);
        assert_eq!(entries[5].value, FieldValue::Label("2 items".to_string()));
    }

    #[test]
    fn collapses_newtype_tuple_structs() {
        let entries = field_entries(&Wrapper(Nested { depth: 3 }));

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
        let entries = field_entries(&Name::new("Left Cube"));

        assert_eq!(
            entries.len(),
            1,
            "expected one caption row, got {entries:?}"
        );
        assert_eq!(entries[0].value, FieldValue::Label("Left Cube".to_string()));
    }

    #[test]
    fn shortens_opaque_type_paths() {
        let entries = field_entries(&Holder(Arc::new(StrongHandle)));

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
}
