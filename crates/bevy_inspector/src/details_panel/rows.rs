//! Spawning and updating the field rows and widgets of a component group.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use bevy_color::{Color, Srgba};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    system::{Commands, Query, ResMut},
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    containers::group,
    controls::{
        list_rows_from_strings, ButtonVariant, ColorInputValue, FeathersButton, FeathersCheckbox,
        FeathersColorInput, FeathersDisclosureToggle, FeathersNumberInput, FeathersSelect,
        FeathersTextInput, FeathersTextInputContainer, HardLimit, NumberInputSnapToStep,
        NumberInputStep,
    },
    display::caption,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBorderColor, ThemeToken, ThemedText},
    tokens,
};
use bevy_log::warn;
use bevy_scene::{bsn, on, Scene, WorldSceneExt};
use bevy_text::{EditableText, FontSource, FontWeight, LineBreak, TextEdit, TextLayout};
use bevy_ui::{
    percent, px, widget::Text, AlignItems, Checked, Display, FlexDirection, Node, Overflow, UiRect,
    Val,
};
use bevy_ui_widgets::{Activate, ValueChange};

use super::{
    edit::{
        emit_field_edit, inspector_field_bool_changed, inspector_field_color_changed,
        inspector_field_f32_changed, inspector_field_f64_changed, inspector_field_hex_entered,
        inspector_field_hex_focus_lost, inspector_field_i32_changed, inspector_field_i64_changed,
        inspector_field_text_changed, inspector_field_variant_changed,
    },
    ticks::{UNWRITTEN_BAR, WRITE_BAR_WIDTH},
    walk::join,
    ComponentDetails, DetailsIndex, FieldEntry, FieldValue, FieldWidget, InspectorField,
    InspectorFieldGroupBody, InspectorFieldGroupToggle, InspectorSegment, CARD_PADDING,
    CARD_RADIUS, INDENT,
};
use crate::{column_split::ColumnSplitLeading, entity_tree::InspectorUi};

/// Spawns the rows of `entries` into `container` and tracks their widgets in [`DetailsIndex`].
///
/// The fields of a vector are shown side by side under one label, and the fields of any other
/// struct are grouped in a block of their own. `depth` is the nesting level of the entries spawned
/// directly into `container`, which are not indented.
pub(super) fn spawn_fields(
    world: &mut World,
    container: Entity,
    component: &ComponentDetails,
    entries: &[FieldEntry],
    depth: usize,
) {
    let mut index = 0;
    while index < entries.len() {
        let entry = &entries[index];
        let end = index
            + 1
            + entries[index + 1..]
                .iter()
                .take_while(|child| child.depth > entry.depth)
                .count();
        let indent = entry.depth.saturating_sub(depth);
        if let Some(axes) = vector_axes(entry, &entries[index + 1..end]) {
            spawn_vector_row(world, container, component, entry, axes, indent);
        } else if end > index + 1 && entry.value == FieldValue::Label(String::new()) {
            if let Some((block, body)) = spawn_field_group(world, container, &entry.label, indent) {
                track_widget(world, component, entry, block, None, None);
                spawn_fields(
                    world,
                    body,
                    component,
                    &entries[index + 1..end],
                    entry.depth + 1,
                );
            }
        } else {
            spawn_field_row(world, container, component, entry, indent);
            index += 1;
            continue;
        }
        index = end;
    }
}

/// Records `entity` as the widget showing `entry` in [`DetailsIndex`].
fn track_widget(
    world: &mut World,
    component: &ComponentDetails,
    entry: &FieldEntry,
    entity: Entity,
    text: Option<Entity>,
    row: Option<Entity>,
) {
    world.resource_mut::<DetailsIndex>().fields.insert(
        (component.key.clone(), entry.path.clone()),
        FieldWidget {
            entity,
            text,
            row,
            value: entry.value.clone(),
        },
    );
}

/// The names of the vector axes, and the color of the strip marking each one.
const AXES: [(&str, ThemeToken); 4] = [
    ("X", tokens::TEXT_INPUT_X_AXIS),
    ("Y", tokens::TEXT_INPUT_Y_AXIS),
    ("Z", tokens::TEXT_INPUT_Z_AXIS),
    ("W", tokens::TEXT_INPUT_W_AXIS),
];

/// The rows of the axes of `entry` if it is a mathematical vector: a struct of two to four editable numbers
/// named `x`, `y`, `z` and `w` in that order.
fn vector_axes<'a>(entry: &FieldEntry, children: &'a [FieldEntry]) -> Option<&'a [FieldEntry]> {
    let is_axis = |(child, (name, _)): (&FieldEntry, &(&str, ThemeToken))| {
        child.depth == entry.depth + 1
            && child.label.eq_ignore_ascii_case(name)
            && child.path == join(&entry.path, &name.to_ascii_lowercase())
            && matches!(child.value, FieldValue::Number(_))
    };
    (entry.value == FieldValue::Label(String::new())
        && (2..=AXES.len()).contains(&children.len())
        && children.iter().zip(AXES.iter()).all(is_axis))
    .then_some(children)
}

/// The padding of a field row at `indent`, whose right side leaves room between the widgets and
/// the write bar.
fn row_padding(indent: usize) -> UiRect {
    UiRect::left(px(indent as f32 * INDENT)).with_right(px(CARD_PADDING - WRITE_BAR_WIDTH))
}

/// Spawns the label of the vector `entry` above a number input for each of its `axes`.
fn spawn_vector_row(
    world: &mut World,
    container: Entity,
    component: &ComponentDetails,
    entry: &FieldEntry,
    axes: &[FieldEntry],
    indent: usize,
) {
    let row = world
        .spawn((
            InspectorUi,
            ThemedText,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: row_padding(indent),
                border: UiRect::right(px(WRITE_BAR_WIDTH)),
                min_width: px(0),
                ..Default::default()
            },
            ThemeBorderColor(UNWRITTEN_BAR),
            ChildOf(container),
        ))
        .id();
    spawn_row_label(world, row, entry.label.clone(), false);
    let inputs = world
        .spawn((
            InspectorUi,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                min_width: px(0),
                ..Default::default()
            },
            ChildOf(row),
        ))
        .id();
    track_widget(world, component, entry, row, None, None);

    for (axis_entry, (name, sigil)) in axes.iter().zip(AXES) {
        let Some(input) = spawn_number_input(world, inputs, axis_entry, Some(name), sigil) else {
            continue;
        };
        world.entity_mut(input).insert(InspectorField {
            component: component.key.clone(),
            path: axis_entry.path.clone(),
        });
        let widget = FieldWidget {
            entity: input,
            text: None,
            row: Some(row),
            value: axis_entry.value.clone(),
        };
        apply_value(world, &widget, &axis_entry.value);
        track_widget(world, component, axis_entry, input, None, Some(row));
    }
}

/// Spawns a block grouping the fields of a struct under a collapsible header, returning the block
/// and the body that holds the fields.
fn spawn_field_group(
    world: &mut World,
    container: Entity,
    label: &str,
    indent: usize,
) -> Option<(Entity, Entity)> {
    let block = spawn_child_scene(
        world,
        container,
        bsn! {
            InspectorUi
            @group()
            Node {
                padding: {UiRect::all(px(CARD_PADDING)).with_right(px(0))},
                row_gap: px(6),
                border_radius: px(CARD_RADIUS),
                margin: {UiRect::left(px(indent as f32 * INDENT)).with_right(px(CARD_PADDING))},
            }
            ThemedText
        },
    )?;
    let header = world
        .spawn((
            InspectorUi,
            ThemedText,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                min_width: px(0),
                ..Default::default()
            },
            ChildOf(block),
        ))
        .id();
    let toggle = spawn_child_scene(
        world,
        header,
        bsn! {
            InspectorUi
            @FeathersDisclosureToggle
            Checked
            on(inspector_field_group_toggled)
        },
    )?;
    spawn_row_label(world, header, label.to_string(), false);
    let body = world
        .spawn((
            InspectorUi,
            InspectorFieldGroupBody,
            ThemedText,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                min_width: px(0),
                ..Default::default()
            },
            ChildOf(block),
        ))
        .id();
    world
        .entity_mut(toggle)
        .insert(InspectorFieldGroupToggle(body));
    Some((block, body))
}

/// Observer that shows or hides the fields of a struct block when its toggle changes.
pub(crate) fn inspector_field_group_toggled(
    change: On<ValueChange<bool>>,
    toggles: Query<&InspectorFieldGroupToggle>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    let Ok(toggle) = toggles.get(change.source) else {
        return;
    };
    if change.value {
        commands.entity(change.source).insert(Checked);
    } else {
        commands.entity(change.source).remove::<Checked>();
    }
    if let Ok(mut node) = nodes.get_mut(toggle.0) {
        node.display = if change.value {
            Display::Flex
        } else {
            Display::None
        };
    }
}

/// Spawns the single-line label of a row into `parent`, inside a leading column that lines up
/// with the [`ColumnSplit`](crate::column_split::ColumnSplit) of the row's group if `leading` is set.
fn spawn_row_label(
    world: &mut World,
    parent: Entity,
    text: String,
    leading: bool,
) -> Option<Entity> {
    let node = Node {
        flex_shrink: 0.0,
        overflow: Overflow::clip_x(),
        ..Default::default()
    };
    let cell = match leading {
        true => world.spawn((ColumnSplitLeading, ThemedText, node, ChildOf(parent))),
        false => world.spawn((ThemedText, node, ChildOf(parent))),
    }
    .id();
    let label = spawn_child_scene(world, cell, caption(text))?;
    world.entity_mut(label).insert(TextLayout {
        linebreak: LineBreak::NoWrap,
        ..Default::default()
    });
    Some(label)
}

fn spawn_field_row(
    world: &mut World,
    container: Entity,
    component: &ComponentDetails,
    entry: &FieldEntry,
    indent: usize,
) -> Option<()> {
    let stacked = matches!(entry.value, FieldValue::Color(_));
    let row = world
        .spawn((
            InspectorUi,
            ThemedText,
            Node {
                display: Display::Flex,
                flex_direction: if stacked {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                align_items: if stacked {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                column_gap: px(6),
                row_gap: px(2),
                padding: row_padding(indent),
                border: UiRect::right(px(WRITE_BAR_WIDTH)),
                min_width: px(0),
                flex_shrink: 1.0,
                overflow: Overflow::clip_x(),
                ..Default::default()
            },
            ThemeBorderColor(UNWRITTEN_BAR),
            ChildOf(container),
        ))
        .id();

    spawn_row_label(world, row, entry.label.clone(), !stacked)?;

    let widget = spawn_widget(world, row, entry)?;
    if !matches!(entry.value, FieldValue::Variant { .. }) {
        apply_value(world, &widget, &entry.value);
    }

    let inputs = match entry.value {
        FieldValue::Bool(_)
        | FieldValue::Number(_)
        | FieldValue::Text(_)
        | FieldValue::Variant { .. } => [Some(widget.entity), None],
        FieldValue::Color(_) => [Some(widget.entity), widget.text],
        FieldValue::Label(_) => [None, None],
    };
    for input in inputs.into_iter().flatten() {
        let Ok(mut entity) = world.get_entity_mut(input) else {
            continue;
        };
        entity.insert(InspectorField {
            component: component.key.clone(),
            path: entry.path.clone(),
        });
    }

    track_widget(
        world,
        component,
        entry,
        widget.entity,
        widget.text,
        Some(row),
    );
    Some(())
}

/// Spawns a number input for `entry`, with an axis `label` and `sigil` strip if given.
fn spawn_number_input(
    world: &mut World,
    parent: Entity,
    entry: &FieldEntry,
    label: Option<&'static str>,
    sigil: ThemeToken,
) -> Option<Entity> {
    let unit = entry.annotations.unit;
    let entity = spawn_child_scene(
        world,
        parent,
        bsn! {
            InspectorUi
            @FeathersNumberInput {
                @interior_label_text: {unit},
                @label_text: {label},
                @sigil_color: {sigil},
            }
            on(inspector_field_f32_changed)
            on(inspector_field_f64_changed)
            on(inspector_field_i32_changed)
            on(inspector_field_i64_changed)
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
            }
        },
    )?;
    if let Some(limit) = &entry.limit {
        world.entity_mut(entity).insert(HardLimit(limit.clone()));
    }
    if let Some(step) = entry.annotations.step {
        world
            .entity_mut(entity)
            .insert((NumberInputStep(step), NumberInputSnapToStep));
    }
    Some(entity)
}

/// The number of variants an enum with only unit variants needs to show as a segmented control.
const SEGMENTS: core::ops::RangeInclusive<usize> = 2..=3;

/// Spawns a row of buttons, one for each of `labels`, with the `selected` one highlighted.
fn spawn_segmented(
    world: &mut World,
    parent: Entity,
    labels: Vec<String>,
    selected: usize,
) -> Option<Entity> {
    let entity = world
        .spawn((
            InspectorUi,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: px(1),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..Default::default()
            },
            ChildOf(parent),
        ))
        .id();
    let last = labels.len().saturating_sub(1);
    for (index, label) in labels.into_iter().enumerate() {
        let corners = match (index == 0, index == last) {
            (true, true) => RoundedCorners::All,
            (true, false) => RoundedCorners::Left,
            (false, true) => RoundedCorners::Right,
            (false, false) => RoundedCorners::None,
        };
        let variant = ButtonVariant::selected(index == selected);
        let button = spawn_child_scene(
            world,
            entity,
            bsn! {
                InspectorUi
                @FeathersButton {
                    @caption: bsn! {
                        @caption(label)
                        TextLayout {
                            linebreak: LineBreak::NoWrap,
                        }
                    },
                    @variant: {variant},
                    @corners: {corners},
                }
                InspectorSegment(index)
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    padding: UiRect::horizontal(px(4)),
                    overflow: Overflow::clip_x(),
                }
                on(inspector_field_segment_activated)
            },
        )?;
        world.entity_mut(button).insert(InheritableFont {
            font: FontSource::Families(fonts::SANS.into()),
            font_size: size::SMALL_FONT,
            weight: FontWeight::NORMAL,
        });
    }
    Some(entity)
}

/// Observer that turns a click on a segment of a segmented variant control into a [`FieldEdit`](super::FieldEdit).
pub(crate) fn inspector_field_segment_activated(
    activate: On<Activate>,
    segments: Query<(&InspectorSegment, &ChildOf)>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let Ok((segment, child_of)) = segments.get(activate.event_target()) else {
        return;
    };
    let selected = segment.0;
    emit_field_edit(
        child_of.parent(),
        |current| match current {
            FieldValue::Variant {
                variants,
                selected: current,
            } if *current != selected => Some(FieldValue::Variant {
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

fn spawn_widget(world: &mut World, row: Entity, entry: &FieldEntry) -> Option<FieldWidget> {
    let value = &entry.value;
    let entity = match value {
        FieldValue::Bool(_) => spawn_child_scene(
            world,
            row,
            bsn! {
                InspectorUi
                @FeathersCheckbox
                on(inspector_field_bool_changed)
            },
        )?,
        FieldValue::Number(_) => {
            spawn_number_input(world, row, entry, None, tokens::TEXT_INPUT_LABEL_BG)?
        }
        FieldValue::Text(_) => spawn_text_input(world, row, px(0), 1.0)?,
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
                        min_width: px(0),
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
                    @FeathersColorInput
                    on(inspector_field_color_changed)
                },
            )?;
            let text = spawn_text_input(world, cell, px(0), 1.0)?;
            world
                .entity_mut(text)
                .observe(inspector_field_hex_entered)
                .observe(inspector_field_hex_focus_lost);
            return Some(FieldWidget {
                entity,
                text: Some(text),
                row: Some(row),
                value: value.clone(),
            });
        }
        FieldValue::Variant { variants, selected } => {
            let labels: Vec<String> = variants
                .iter()
                .map(|name| entry.annotations.variant_label(name).to_string())
                .collect();
            if entry.unit_enum && SEGMENTS.contains(&labels.len()) {
                spawn_segmented(world, row, labels, *selected)?
            } else {
                let options = list_rows_from_strings(labels, Some(*selected));
                spawn_child_scene(
                    world,
                    row,
                    bsn! {
                        InspectorUi
                        @FeathersSelect {
                            @options: {options},
                        }
                        on(inspector_field_variant_changed)
                        Node {
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            min_width: px(0),
                        }
                    },
                )?
            }
        }
        FieldValue::Label(text) => spawn_value_caption(world, row, text.clone())?,
    };
    Some(FieldWidget {
        entity,
        text: None,
        row: Some(row),
        value: value.clone(),
    })
}

/// Spawns a text input sized by `width` and `flex_grow`, returning the entity holding its
/// [`EditableText`].
fn spawn_text_input(
    world: &mut World,
    parent: Entity,
    width: Val,
    flex_grow: f32,
) -> Option<Entity> {
    let container = spawn_child_scene(
        world,
        parent,
        bsn! {
            InspectorUi
            @FeathersTextInputContainer
            Node {
                width: {width},
                min_width: px(0),
                flex_grow: {flex_grow},
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
/// Variant selects are left untouched, since a changed variant respawns its group instead, while
/// segmented variant controls highlight the selected variant.
pub(super) fn apply_value(world: &mut World, widget: &FieldWidget, value: &FieldValue) {
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
                entity.insert(ColorInputValue(*color));
            }
            replace_text(world, widget.text, &color_to_hex(*color));
        }
        FieldValue::Variant { selected, .. } => {
            let segments: Vec<Entity> = world
                .get::<Children>(widget.entity)
                .map(|children| children.iter().copied().collect())
                .unwrap_or_default();
            for segment in segments {
                let Some(index) = world
                    .get::<InspectorSegment>(segment)
                    .map(|segment| segment.0)
                else {
                    continue;
                };
                if let Some(mut variant) = world.get_mut::<ButtonVariant>(segment) {
                    variant.set_if_neq(ButtonVariant::selected(index == *selected));
                }
            }
        }
        FieldValue::Label(text) => set_text(world, Some(widget.entity), text.clone()),
    }
}

/// Replaces the contents of the [`EditableText`] on `entity` with `text`, unless it already holds `text`.
pub(super) fn replace_text(world: &mut World, entity: Option<Entity>, text: &str) {
    if let Some(mut editable) = entity.and_then(|entity| world.get_mut::<EditableText>(entity))
        && editable.value() != text
    {
        editable.clear();
        editable.queue_edit(TextEdit::Insert(text.into()));
    }
}

/// Replaces the [`Text`] of `entity` with `value`, unless it already shows `value`.
pub(super) fn set_text(world: &mut World, entity: Option<Entity>, value: String) {
    let Some(mut text) = entity.and_then(|entity| world.get_mut::<Text>(entity)) else {
        return;
    };
    if text.0 == value {
        return;
    }
    text.0 = value;
}

/// The sRGB hexadecimal code of `color`, such as `#FF8000`.
pub(super) fn color_to_hex(color: Color) -> String {
    Srgba::from(color).to_hex()
}

/// Spawns `scene` as the last child of `parent`, logging a warning if it fails to spawn.
pub(super) fn spawn_child_scene(
    world: &mut World,
    parent: Entity,
    scene: impl Scene,
) -> Option<Entity> {
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
pub(super) fn descendant_with<C: Component>(world: &World, root: Entity) -> Option<Entity> {
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
