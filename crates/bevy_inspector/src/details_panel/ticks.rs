//! The change ticks of the components shown in the details panel, and the highlight on the names
//! of components the app writes between refreshes.

use alloc::{format, string::String};

use bevy_ecs::{
    change_detection::ComponentTicks, component::ComponentId, entity::Entity, hierarchy::Children,
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    display::caption,
    font_styles::InheritableFont,
    theme::{ThemeTextColor, ThemeToken},
    tokens,
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_scene::{bsn, Scene};
use bevy_text::{FontSourceTemplate, FontWeight, LineBreak, TextLayout};
use bevy_ui::{px, widget::Text, Node, UiRect};

use super::{
    rows::{descendant_with, set_text, spawn_child_scene},
    ComponentDetails, ComponentKey, DetailsIndex,
};

const WRITTEN_NAME_TEXT: ThemeToken = tokens::TEXT_ACCENT;

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

pub(super) fn show_ticks(world: &mut World, component: &ComponentDetails, written: bool) {
    let Some((name, row)) = world
        .resource::<DetailsIndex>()
        .groups
        .get(&component.key)
        .map(|group| (group.name, group.ticks))
    else {
        return;
    };
    if let Some(ticks) = component.ticks {
        set_text(world, row, ticks_text(ticks));
    }
    let token = if written {
        WRITTEN_NAME_TEXT
    } else {
        tokens::LISTROW_TEXT
    };
    if let Some(name) = name
        && world
            .get::<ThemeTextColor>(name)
            .is_none_or(|color| color.0 != token)
    {
        world.entity_mut(name).insert(ThemeTextColor(token));
    }
}

#[cfg(test)]
pub(super) fn is_highlighted(world: &World, component: &str) -> bool {
    world
        .resource::<DetailsIndex>()
        .groups
        .get(component)
        .and_then(|group| group.name)
        .and_then(|name| world.get::<ThemeTextColor>(name))
        .is_some_and(|color| color.0 == WRITTEN_NAME_TEXT)
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
