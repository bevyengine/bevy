//! Provides a way to automatically set the mouse cursor based on hovered entity.
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_picking::{hover::HoverMap, pointer::PointerId, PickingSystems};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_window::{SystemCursorIcon, Window};
use bevy_winit::cursor::CursorIcon;

/// A component that specifies the cursor icon to be used when the mouse is not hovering over
/// any other entity. This is used to set the default cursor icon for the window.
#[derive(Resource, Debug, Clone, Default)]
pub struct DefaultEntityCursor(pub EntityCursor);

/// A component that specifies the cursor shape to be used when the pointer hovers over an entity.
/// This is copied to the windows's [`CursorIcon`] component.
///
/// This is effectively the same type as [`CustomCursor`] but with different methods, and used
/// in different places.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub enum EntityCursor {
    #[cfg(feature = "custom_cursor")]
    /// Custom cursor image.
    Custom(CustomCursor),
    /// System provided cursor icon.
    System(SystemCursorIcon),
}

impl EntityCursor {
    /// Convert the [`EntityCursor`] to a [`CursorIcon`] so that it can be inserted into a
    /// window.
    pub fn as_cursor_icon(&self) -> CursorIcon {
        match self {
            #[cfg(feature = "custom_cursor")]
            EntityCursor::Custom(custom_cursor) => CursorIcon::Custom(custom_cursor),
            EntityCursor::System(icon) => CursorIcon::from(*icon),
        }
    }

    /// Compare the [`EntityCursor`] to a [`CursorIcon`] so that we can see whether or not
    /// the window cursor needs to be changed.
    pub fn eq_cursor_icon(&self, cursor_icon: &CursorIcon) -> bool {
        match (self, cursor_icon) {
            #[cfg(feature = "custom_cursor")]
            (EntityCursor::Custom(custom), CursorIcon::Custom(other)) => custom == other,
            (EntityCursor::System(system), cursor_icon) => {
                CursorIcon::from(*system) == *cursor_icon
            }
        }
    }
}

impl Default for EntityCursor {
    fn default() -> Self {
        EntityCursor::System(Default::default())
    }
}

/// System which updates the window cursor icon whenever the mouse hovers over an entity with
/// a [`CursorIcon`] component. If no entity is hovered, the cursor icon is set to
/// the cursor in the [`DefaultCursorIcon`] resource.
pub(crate) fn update_cursor(
    mut commands: Commands,
    hover_map: Option<Res<HoverMap>>,
    parent_query: Query<&ChildOf>,
    cursor_query: Query<&EntityCursor>,
    mut q_windows: Query<(Entity, &mut Window, Option<&CursorIcon>)>,
    r_default_cursor: Res<DefaultEntityCursor>,
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
            (Some(cursor), Some(prev_cursor)) if cursor.eq_cursor_icon(prev_cursor) => continue,
            (None, None) => continue,
            _ => {
                windows_to_change.push(entity);
            }
        }
    }
    windows_to_change.iter().for_each(|entity| {
        if let Some(cursor) = cursor {
            commands.entity(*entity).insert(cursor.as_cursor_icon());
        } else {
            commands
                .entity(*entity)
                .insert(r_default_cursor.0.as_cursor_icon());
        }
    });
}

/// Plugin that supports automatically changing the cursor based on the hovered entity.
pub struct CursorIconPlugin;

impl Plugin for CursorIconPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<DefaultEntityCursor>().is_none() {
            app.init_resource::<DefaultEntityCursor>();
        }
        app.add_systems(PreUpdate, update_cursor.in_set(PickingSystems::Last));
    }
}
