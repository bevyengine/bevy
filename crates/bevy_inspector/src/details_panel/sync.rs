//! Keeping the details panel in sync with the selected entity.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use bevy_app::Propagate;
use bevy_color::{Alpha, Srgba};
use bevy_dev_tools::inspection::{
    component_inspection::{ComponentDetailLevel, ComponentInspectionSettings},
    entity_inspection::EntityInspectionSettings,
    extension_methods::WorldInspectionExtensionTrait,
};
use bevy_ecs::{
    component::{Component, ComponentInfo},
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::With,
    reflect::AppTypeRegistry,
    resource::IsResource,
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    controls::FeathersDisclosureToggle,
    display::caption,
    font_styles::InheritableFont,
    theme::{
        InheritableThemeTextColor, SurfaceLevel, ThemeBackgroundColor, ThemeBorderColor,
        ThemeContext, ThemeTextColor, ThemedText,
    },
    tokens,
};
use bevy_input_focus::InputFocus;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::PartialReflect;
use bevy_scene::{bsn, on, Scene};
use bevy_text::{FontSourceTemplate, FontWeight, LineBreak, TextLayout};
use bevy_ui::{
    px, AlignItems, BoxShadow, Checked, Display, FlexDirection, JustifyContent, Node, Overflow,
    UiRect,
};

use super::{
    annotations::FieldAnnotations,
    edit::same_value,
    entity_section::prepend_entity_section,
    inspector_details_toggled,
    rows::{apply_value, descendant_with, spawn_child_scene, spawn_fields},
    ticks::{
        changed_paths, rows_of, show_ticks, spawn_ticks_row, written_by_app, UNWRITTEN_BAR,
        WRITE_BAR_WIDTH,
    },
    walk::field_entries,
    ComponentDetails, ComponentKey, DetailsCollapsed, DetailsColumnSplits, DetailsIndex,
    DetailsPanelSync, EmptyState, FieldEntry, FieldValue, GroupWidget, InspectorDetailsBody,
    InspectorDetailsComponent, InspectorDetailsFields, RowLayout, CARD_PADDING, CARD_RADIUS,
};
use crate::{
    column_split::ColumnSplit, entity_tree::InspectorUi, InspectableEntity, InspectorSelection,
    InspectorSource,
};

/// Rebuilds or refreshes the details panel so that it matches the selected entity.
///
/// Only the groups of components that were added, removed, collapsed or expanded are respawned.
/// Groups of components the new selection shares with the old one are kept.
pub fn sync_details_panel(world: &mut World) {
    let run = DetailsPanelSync::tick(world);

    let selection = world.resource::<InspectorSelection>().0.clone();
    let selection_changed = world.resource::<DetailsIndex>().selection.clone() != selection;
    if !run && !selection_changed {
        return;
    }

    let Some(body) = find_body(world) else {
        return;
    };

    let maybe_selected = selection.clone().map(|s| s.entity);

    let inspector_source = selection.clone().map(|s| s.source);
    let Some(inspector_source) = inspector_source else {
        let index = world.resource::<DetailsIndex>();
        if index.body != Some(body) || index.empty != Some(EmptyState::NoSelection) {
            reset_body(world, body, None, Some(EmptyState::NoSelection));
        }
        return;
    };
    let local = inspector_source == InspectorSource::Local;

    let inspected = crate::world_to_inspect(world, &inspector_source);
    let mut components = inspect_components(inspected, selection.clone(), local);
    let empty = empty_state(inspected, maybe_selected, &components);
    if local
        && matches!(empty, None | Some(EmptyState::NoComponents))
        && let Some(entity) = maybe_selected
    {
        prepend_entity_section(inspected, entity, &mut components);
    }

    let index = world.resource::<DetailsIndex>();
    if index.body != Some(body) || index.empty != empty {
        reset_body(world, body, selection.clone(), empty);
    } else if selection_changed {
        discard_focused_group(world);
        let mut index = world.resource_mut::<DetailsIndex>();
        index.selection = selection.clone();
        index.changed.clear();
    }
    let written = maybe_selected
        .map(|entity| written_by_app(world, entity, &components))
        .unwrap_or_default();
    if !components.is_empty() {
        sync_groups(world, body, &components, &written);
    }
}

/// Forces a details panel synchronization pass on the next tick, if the panel is set up.
pub(super) fn refresh_details_panel(world: &mut World) {
    if let Some(mut sync) = world.get_resource_mut::<DetailsPanelSync>() {
        sync.set_dirty();
    }
}

/// Despawns the group holding the focused widget and clears the focus, discarding any value typed
/// into it before the selection changed.
fn discard_focused_group(world: &mut World) {
    let Some(mut entity) = world.get_resource::<InputFocus>().and_then(InputFocus::get) else {
        return;
    };
    loop {
        let group = world
            .resource::<DetailsIndex>()
            .groups
            .iter()
            .find(|(_, group)| group.entity == entity)
            .map(|(key, _)| key.clone());
        if let Some(key) = group {
            despawn_group(world, &key);
            world.resource_mut::<InputFocus>().clear();
            return;
        }
        let Some(&ChildOf(parent)) = world.get::<ChildOf>(entity) else {
            return;
        };
        entity = parent;
    }
}

/// The body of the details panel, found through its [`InspectorDetailsBody`] marker.
pub(super) fn find_body(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<InspectorDetailsBody>>()
        .iter(world)
        .next()
}

/// The component groups of `selection` in the inspected `world`, sorted in display order.
pub(crate) fn inspect_components(
    world: &World,
    selection: Option<InspectableEntity>,
    local: bool,
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
                fields = fields.into_iter().map(FieldEntry::into_read_only).collect();
            }
            let ticks = world
                .get_entity(selection.entity)
                .ok()
                .filter(|_| local)
                .and_then(|entity| entity.get_change_ticks_by_id(component.component_id));
            let display_name = FieldAnnotations::of_type(
                component
                    .reflected_value
                    .as_deref()
                    .and_then(PartialReflect::get_represented_type_info),
            )
            .display_name;
            ComponentDetails {
                key: ComponentKey::of(world, component.component_id),
                name: display_name
                    .map(ToString::to_string)
                    .unwrap_or_else(|| crate::component_short_name(world, component.component_id)),
                memory: component.memory_size.to_string(),
                fields,
                ticks,
            }
        })
        .collect();
    #[cfg(feature = "remote")]
    if let Some(record) = world.component_id::<crate::remote::RemoteComponents>() {
        let record = ComponentKey::of(world, record);
        components.retain(|component| component.key != record);
    }
    if let Some(marker) = world.component_id::<IsResource>() {
        let marker = ComponentKey::of(world, marker);
        components.retain(|component| component.key != marker);
    }
    components.sort_by(|left, right| (&left.name, &left.key).cmp(&(&right.name, &right.key)));
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
    selection: Option<InspectableEntity>,
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
fn sync_groups(
    world: &mut World,
    body: Entity,
    components: &[ComponentDetails],
    written: &HashSet<ComponentKey>,
) {
    let collapsed = world.resource::<DetailsCollapsed>().clone();
    let present: HashSet<&ComponentKey> =
        components.iter().map(|component| &component.key).collect();
    let stale: Vec<ComponentKey> = world
        .resource::<DetailsIndex>()
        .groups
        .iter()
        .filter(|(key, group)| {
            !present.contains(key) || group.expanded == collapsed.is_collapsed(key)
        })
        .map(|(key, _)| key.clone())
        .collect();

    let mut reordered = !stale.is_empty();
    for key in stale {
        despawn_group(world, &key);
    }

    for component in components {
        let written = written.contains(&component.key);
        let shown = world
            .resource::<DetailsIndex>()
            .groups
            .contains_key(&component.key);
        let kept = shown.then(|| update_in_place(world, component)).flatten();
        let changed = match kept {
            Some(changed) => changed,
            None => {
                let paths = match written {
                    true => changed_paths(world, component),
                    false => Vec::new(),
                };
                despawn_group(world, &component.key);
                let expanded = !collapsed.is_collapsed(&component.key);
                spawn_group(world, body, component, expanded);
                reordered = true;
                rows_of(world, &component.key, &paths)
            }
        };
        show_ticks(world, component, written, &changed);
    }

    if reordered {
        order_groups(world, body, components);
    }
}

/// Writes the current field values of `component` into its existing widgets, returning the rows
/// whose shown value changed.
///
/// Returns `None`, leaving every widget untouched, if rows were added, removed, relabeled or
/// changed kind, and the group must be respawned instead.
fn update_in_place(world: &mut World, component: &ComponentDetails) -> Option<Vec<Entity>> {
    let mut updates = Vec::new();
    {
        let index = world.resource::<DetailsIndex>();
        let group = index.groups.get(&component.key)?;
        if group.ticks.is_some() != (group.expanded && component.ticks.is_some()) {
            return None;
        }
        if !group.expanded {
            return Some(Vec::new());
        }
        if group.layout.len() != component.fields.len()
            || !group
                .layout
                .iter()
                .zip(&component.fields)
                .all(|(row, entry)| row.matches(entry))
        {
            return None;
        }
        for entry in &component.fields {
            let key = (component.key.clone(), entry.path.clone());
            let widget = index.fields.get(&key)?;
            if same_value(&widget.value, &entry.value) {
                continue;
            }
            if matches!(entry.value, FieldValue::Variant { .. }) {
                return None;
            }
            updates.push((key, widget.clone(), entry.value.clone()));
        }
    }

    let mut changed = Vec::new();
    for (key, widget, value) in updates {
        apply_value(world, &widget, &value);
        changed.extend(widget.row);
        if let Some(stored) = world.resource_mut::<DetailsIndex>().fields.get_mut(&key) {
            stored.value = value;
        }
    }
    Some(changed)
}

/// Despawns the group of `component` and forgets its field widgets.
fn despawn_group(world: &mut World, component: &ComponentKey) {
    let mut index = world.resource_mut::<DetailsIndex>();
    let group = index.groups.remove(component);
    index.fields.retain(|(key, _), _| key != component);
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
        .filter_map(|(position, component)| Some((index.group(&component.key.0)?, position)))
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
    let header = descendant_with::<InspectorDetailsHeader>(world, group);
    let ticks = expanded
        .then(|| spawn_ticks_row(world, group, component))
        .flatten();
    world.resource_mut::<DetailsIndex>().groups.insert(
        component.key.clone(),
        GroupWidget {
            entity: group,
            expanded,
            layout: component.fields.iter().map(RowLayout::new).collect(),
            header,
            ticks,
            marked: HashSet::default(),
        },
    );

    if let Some(toggle) = descendant_with::<FeathersDisclosureToggle>(world, group) {
        let mut toggle = world.entity_mut(toggle);
        toggle.insert(InspectorDetailsComponent(component.key.clone()));
        if expanded {
            toggle.insert(Checked);
        }
    }

    let Some(container) = descendant_with::<InspectorDetailsFields>(world, group) else {
        return;
    };
    if !expanded {
        if let Some(mut node) = world.get_mut::<Node>(container) {
            node.display = Display::None;
        }
        return;
    }

    if !component.fields.is_empty() {
        let mut split = ColumnSplit::default();
        if let Some(&fraction) = world
            .resource::<DetailsColumnSplits>()
            .0
            .get(&component.key)
        {
            split.fraction = fraction;
        }
        world
            .entity_mut(container)
            .insert((InspectorDetailsComponent(component.key.clone()), split));
    }

    spawn_fields(world, container, component, &component.fields, 0);
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub(super) struct InspectorDetailsName;

#[derive(Component, Debug, Default, Clone, Copy)]
struct InspectorDetailsHeader;

fn component_group(name: String, memory: String) -> impl Scene {
    bsn! {
        InspectorUi
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border: px(1),
            border_radius: px(CARD_RADIUS),
            overflow: Overflow::clip(),
        }
        ThemeBackgroundColor(tokens::SUBPANE_BODY_BG)
        ThemeBorderColor(tokens::SUBPANE_BODY_BORDER)
        BoxShadow::new(
            Srgba::BLACK.with_alpha(0.88).into(),
            px(0),
            px(0),
            px(0),
            px(1),
        )
        Children [
            InspectorDetailsHeader
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                height: px(26),
                padding: UiRect::horizontal(px(8)),
                border: UiRect::right(px(WRITE_BAR_WIDTH)),
            }
            ThemeBackgroundColor(tokens::SUBPANE_HEADER_BG)
            ThemeBorderColor(UNWRITTEN_BAR)
            InheritableThemeTextColor(tokens::LISTROW_TEXT)
            InheritableFont {
                font: FontSourceTemplate::Families(fonts::SANS),
                font_size: size::SMALL_FONT,
                weight: FontWeight::MEDIUM,
            }
            Children [
                ThemedText
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    align_items: AlignItems::Center,
                }
                Children [
                    @FeathersDisclosureToggle
                    on(inspector_details_toggled)
                ]
                --
                ThemedText
                Node {
                    min_width: px(0),
                    overflow: Overflow::clip(),
                }
                Children [
                    @caption(name)
                    InspectorDetailsName
                    ThemeTextColor(tokens::LISTROW_TEXT)
                    Node {
                        flex_shrink: 0.0,
                    }
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                    }
                ]
                --
                ThemedText
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: px(6),
                }
                Children [
                    @caption(memory)
                    Node {
                        flex_shrink: 0.0,
                    }
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                    }
                    ThemeTextColor(tokens::TEXT_DIM)
                    --
                    Node {
                        width: px(15),
                        height: px(15),
                        flex_shrink: 0.0,
                    }
                ]
            ]
            --
            InspectorDetailsFields
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: {UiRect::all(px(CARD_PADDING)).with_right(px(0))},
            }
            Propagate::<ThemeContext>(ThemeContext(SurfaceLevel::Base))
            InheritableThemeTextColor(tokens::TEXT_MAIN)
            InheritableFont {
                font: FontSourceTemplate::Families(fonts::SANS),
                font_size: size::SMALL_FONT,
                weight: FontWeight::NORMAL,
            }
        ]
    }
}
