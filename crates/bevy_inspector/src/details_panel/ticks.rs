//! The change ticks of the components shown in the details panel, and the marks on the
//! components and fields the app writes between refreshes.
//!
//! A component the app wrote since the previous refresh shows an accent bar on the right edge of
//! its card header, whether the card is expanded or collapsed. Change ticks are kept per
//! component, not per field, so within an expanded card the same bar marks each row whose shown
//! value differs from the one shown before the refresh. A write that leaves every value as it was
//! marks only the header. Edits made through the panel mark nothing, and every mark clears on the
//! next refresh without a write.

use alloc::{format, string::String, vec::Vec};

use bevy_ecs::{
    change_detection::ComponentTicks, component::ComponentId, entity::Entity, hierarchy::Children,
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    display::caption,
    font_styles::InheritableFont,
    theme::{ThemeBorderColor, ThemeTextColor, ThemeToken},
    tokens,
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_scene::{bsn, Scene};
use bevy_text::{FontSourceTemplate, FontWeight, LineBreak, TextLayout};
use bevy_ui::{px, widget::Text, Node, UiRect};

use super::{
    edit::same_value,
    rows::{descendant_with, set_text, spawn_child_scene},
    ComponentDetails, ComponentKey, DetailsIndex,
};

pub(super) const WRITE_BAR_WIDTH: f32 = 2.0;
pub(super) const UNWRITTEN_BAR: ThemeToken = tokens::TAB_STRIPE;
const WRITTEN_BAR: ThemeToken = tokens::TAB_STRIPE_SELECTED;

fn ticks_text(ticks: ComponentTicks) -> String {
    format!(
        "added {}, changed {}",
        ticks.added.get(),
        ticks.changed.get()
    )
}

pub(super) fn record_inspector_write(
    world: &mut World,
    entity: Entity,
    key: ComponentKey,
    id: ComponentId,
) {
    let Some(ticks) = world
        .get_entity(entity)
        .ok()
        .and_then(|entity| entity.get_change_ticks_by_id(id))
    else {
        return;
    };
    if let Some(mut index) = world.get_resource_mut::<DetailsIndex>() {
        index.inspector_writes.insert((entity, key), ticks.changed);
    }
}

pub(super) fn written_by_app(
    world: &mut World,
    entity: Entity,
    components: &[ComponentDetails],
) -> HashSet<ComponentKey> {
    let mut index = world.resource_mut::<DetailsIndex>();
    let mut changed = HashMap::default();
    let mut written = HashSet::default();
    for component in components {
        let Some(ticks) = component.ticks else {
            continue;
        };
        let moved = index
            .changed
            .get(&component.key)
            .is_some_and(|last| *last != ticks.changed);
        let by_inspector =
            index.inspector_writes.get(&(entity, component.key.clone())) == Some(&ticks.changed);
        if moved && !by_inspector {
            written.insert(component.key.clone());
        }
        changed.insert(component.key.clone(), ticks.changed);
    }
    index.changed = changed;
    index.inspector_writes.clear();
    written
}

pub(super) fn spawn_ticks_row(
    world: &mut World,
    group: Entity,
    component: &ComponentDetails,
) -> Option<Entity> {
    let ticks = component.ticks?;
    let row = spawn_child_scene(world, group, ticks_row(ticks_text(ticks)))?;
    world.entity_mut(group).insert_child(1, row);
    descendant_with::<Text>(world, row)
}

pub(super) fn changed_paths(world: &World, component: &ComponentDetails) -> Vec<String> {
    let index = world.resource::<DetailsIndex>();
    component
        .fields
        .iter()
        .filter(|entry| {
            index
                .fields
                .get(&(component.key.clone(), entry.path.clone()))
                .is_some_and(|widget| !same_value(&widget.value, &entry.value))
        })
        .map(|entry| entry.path.clone())
        .collect()
}

pub(super) fn rows_of(world: &World, component: &ComponentKey, paths: &[String]) -> Vec<Entity> {
    let index = world.resource::<DetailsIndex>();
    paths
        .iter()
        .filter_map(|path| index.fields.get(&(component.clone(), path.clone()))?.row)
        .collect()
}

pub(super) fn show_ticks(
    world: &mut World,
    component: &ComponentDetails,
    written: bool,
    changed: &[Entity],
) {
    let marked: HashSet<Entity> = match written {
        true => changed.iter().copied().collect(),
        false => HashSet::default(),
    };
    let mut index = world.resource_mut::<DetailsIndex>();
    let Some(group) = index.groups.get_mut(&component.key) else {
        return;
    };
    let (header, row) = (group.header, group.ticks);
    let previous = core::mem::replace(&mut group.marked, marked.clone());
    if let Some(ticks) = component.ticks {
        set_text(world, row, ticks_text(ticks));
    }
    if let Some(header) = header {
        show_bar(world, header, written);
    }
    for &row in previous.difference(&marked) {
        show_bar(world, row, false);
    }
    for &row in marked.difference(&previous) {
        show_bar(world, row, true);
    }
}

fn show_bar(world: &mut World, entity: Entity, written: bool) {
    let token = match written {
        true => WRITTEN_BAR,
        false => UNWRITTEN_BAR,
    };
    if let Ok(mut entity) = world.get_entity_mut(entity)
        && entity
            .get::<ThemeBorderColor>()
            .is_none_or(|color| color.0 != token)
    {
        entity.insert(ThemeBorderColor(token));
    }
}

#[cfg(test)]
fn bar_shown(world: &World, entity: Option<Entity>) -> bool {
    entity
        .and_then(|entity| world.get::<ThemeBorderColor>(entity))
        .is_some_and(|color| color.0 == WRITTEN_BAR)
}

#[cfg(test)]
pub(super) fn header_marked(world: &World, component: &str) -> bool {
    let group = world.resource::<DetailsIndex>().groups.get(component);
    bar_shown(world, group.and_then(|group| group.header))
}

#[cfg(test)]
pub(super) fn marked_paths(world: &World, component: &str) -> Vec<String> {
    let mut paths: Vec<String> = world
        .resource::<DetailsIndex>()
        .fields
        .iter()
        .filter(|((key, _), widget)| &*key.0 == component && bar_shown(world, widget.row))
        .map(|((_, path), _)| path.clone())
        .collect();
    paths.sort();
    paths
}

#[cfg(test)]
pub(super) fn name_color(world: &World, component: &str) -> Option<ThemeToken> {
    let group = world.resource::<DetailsIndex>().group(component)?;
    let name = descendant_with::<super::sync::InspectorDetailsName>(world, group)?;
    Some(world.get::<ThemeTextColor>(name)?.0.clone())
}

fn ticks_row(text: String) -> impl Scene {
    bsn! {
        Node {
            padding: UiRect::axes(px(8), px(3)),
        }
        InheritableFont {
            font: FontSourceTemplate::Families(fonts::SANS),
            font_size: size::SMALL_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [
            @caption(text)
            TextLayout {
                linebreak: LineBreak::NoWrap,
            }
            ThemeTextColor(tokens::TEXT_DIM)
        ]
    }
}
