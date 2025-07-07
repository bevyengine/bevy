//! Provides a way to automatically set the mouse cursor based on hovered entity.
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_picking::{hover::HoverMap, pointer::PointerId, PickingSystems};
use bevy_window::Window;
use bevy_winit::cursor::CursorIcon;

/// A component that specifies the cursor icon to be used when the mouse is not hovering over
/// any other entity. This is used to set the default cursor icon for the window.
#[derive(Resource, Debug, Clone, Default)]
pub struct DefaultCursorIcon(pub CursorIcon);

/// System which updates the window cursor icon whenever the mouse hovers over an entity with
/// a [`CursorIcon`] component. If no entity is hovered, the cursor icon is set to
/// the cursor in the [`DefaultCursorIcon`] resource.
pub(crate) fn update_cursor(
    mut commands: Commands,
    hover_map: Option<Res<HoverMap>>,
    parent_query: Query<&ChildOf>,
    cursor_query: Query<&CursorIcon>,
    mut q_windows: Query<(Entity, &mut Window, Option<&CursorIcon>)>,
    r_default_cursor: Res<DefaultCursorIcon>,
) {
    let cursor = hover_map.and_then(|hover_map| match hover_map.get(&PointerId::Mouse) {
        Some(hover_set) => hover_set.keys().find_map(|entity| {
            cursor_query.get(*entity).ok().or_else(|| {
                parent_query
                    .iter_ancestors(*entity)
                    .find_map(|e| cursor_query.get(e).ok())
            })
        }),
        None => None,
    });

    let mut windows_to_change: Vec<Entity> = Vec::new();
    for (entity, _window, prev_cursor) in q_windows.iter_mut() {
        match (cursor, prev_cursor) {
            (Some(cursor), Some(prev_cursor)) if cursor == prev_cursor => continue,
            (None, None) => continue,
            _ => {
                windows_to_change.push(entity);
            }
        }
    }
    windows_to_change.iter().for_each(|entity| {
        if let Some(cursor) = cursor {
            commands.entity(*entity).insert(cursor.clone());
        } else {
            commands.entity(*entity).insert(r_default_cursor.0.clone());
        }
    });
}

/// Plugin that supports automatically changing the cursor based on the hovered entity.
pub struct CursorIconPlugin;

impl Plugin for CursorIconPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<DefaultCursorIcon>().is_none() {
            app.init_resource::<DefaultCursorIcon>();
        }
        app.add_systems(PreUpdate, update_cursor.in_set(PickingSystems::Last));
    }
}
