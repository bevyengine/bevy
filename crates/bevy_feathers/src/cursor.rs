//! Provides a way to automatically set the mouse cursor based on hovered entity.
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    query::{With, Without},
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_picking::{hover::HoverMap, pointer::PointerId, PickingSystems};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(feature = "custom_cursor")]
use bevy_window::CustomCursor;
use bevy_window::{CursorIcon, SystemCursorIcon, Window};

/// A resource that specifies the cursor icon to be used when the mouse is not hovering over
/// any other entity. This is used to set the default cursor icon for the window.
#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DefaultCursor(pub EntityCursor);

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
    pub fn to_cursor_icon(&self) -> CursorIcon {
        match self {
            #[cfg(feature = "custom_cursor")]
            EntityCursor::Custom(custom_cursor) => CursorIcon::Custom(custom_cursor.clone()),
            EntityCursor::System(icon) => CursorIcon::from(*icon),
        }
    }

    /// Compare the [`EntityCursor`] to a [`CursorIcon`] so that we can see whether or not
    /// the window cursor needs to be changed.
    pub fn eq_cursor_icon(&self, cursor_icon: &CursorIcon) -> bool {
        match (self, cursor_icon) {
            #[cfg(feature = "custom_cursor")]
            (EntityCursor::Custom(custom), CursorIcon::Custom(other)) => custom == other,
            (EntityCursor::System(system), CursorIcon::System(cursor_icon)) => {
                *system == *cursor_icon
            }
            _ => false,
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
/// the cursor in the [`DefaultCursor`] resource.
pub(crate) fn update_cursor(
    mut commands: Commands,
    hover_map: Option<Res<HoverMap>>,
    parent_query: Query<&ChildOf>,
    cursor_query: Query<&EntityCursor, Without<Window>>,
    q_windows: Query<(Entity, Option<&CursorIcon>), With<Window>>,
    r_default_cursor: Res<DefaultCursor>,
) {
    let cursor = hover_map
        .and_then(|hover_map| match hover_map.get(&PointerId::Mouse) {
            Some(hover_set) => hover_set.keys().find_map(|entity| {
                cursor_query.get(*entity).ok().or_else(|| {
                    parent_query
                        .iter_ancestors(*entity)
                        .find_map(|e| cursor_query.get(e).ok())
                })
            }),
            None => None,
        })
        .unwrap_or(&r_default_cursor.0);

    for (entity, prev_cursor) in q_windows.iter() {
        if let Some(prev_cursor) = prev_cursor
            && cursor.eq_cursor_icon(prev_cursor)
        {
            continue;
        }
        commands.entity(entity).insert(cursor.to_cursor_icon());
    }
}

/// Plugin that supports automatically changing the cursor based on the hovered entity.
pub struct CursorIconPlugin;

impl Plugin for CursorIconPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<DefaultCursor>().is_none() {
            app.init_resource::<DefaultCursor>();
        }
        app.add_systems(PreUpdate, update_cursor.in_set(PickingSystems::Last));
    }
}
