use bevy_app::{App, Last, Plugin, Update};
use bevy_asset::{AssetId, Assets, Handle};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    event::EventReader,
    observer::Trigger,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Local, Query, Res},
    world::{OnRemove, Ref},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::{tracing::warn, HashSet};
use bevy_window::{SystemCursorIcon, Window, WindowCreated};
use bevy_winit::{
    convert_system_cursor_icon, CursorSource, CustomCursorCache, CustomCursorCacheKey,
    PendingCursor,
};
use wgpu::TextureFormat;

use crate::prelude::Image;

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CursorIcon>()
            .init_resource::<CustomCursorCache>()
            .add_systems(Last, update_cursors)
            .add_systems(Update, add_cursor_to_windows);

        app.observe(on_remove_cursor_icon);
    }
}

/// Insert into a window entity to set the cursor for that window.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub enum CursorIcon {
    /// Makes the cursor hidden
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`**, **`X11`**, and **`Wayland`**: The cursor is hidden only when inside the window.
    ///     To stop the cursor from leaving the window, change the [`bevy_window::Window`]
    ///     entity's
    ///     [`bevy_window::CursorOptions`] [`bevy_window::CursorGrabMode`] to
    ///     [`bevy_window::CursorGrabMode::Locked`] or
    ///     [`bevy_window::CursorGrabMode::Confined`]
    /// - **`macOS`**: The cursor is hidden only when the window is focused.
    /// - **`iOS`** and **`Android`** do not have cursors
    ///
    /// # Note for developers
    ///
    /// This was originally part of the [`bevy_window::CursorOptions`] component
    /// as the `visible` field but it was moved out after adding Custom cursors. Before,
    /// since we only had system cursors provided by the OS, this was heavily tied into
    /// the implementation provided by [`bevy_winit`]. Now, you can think of this as part
    /// of the `bevy_render` level, hence the drift from the original [`bevy_winit`]
    /// coupled API.
    Hidden,
    /// Custom cursor image.
    Custom(CustomCursor),
    /// System provided cursor icon.
    System(SystemCursorIcon),
}

impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::System(Default::default())
    }
}

impl From<SystemCursorIcon> for CursorIcon {
    fn from(icon: SystemCursorIcon) -> Self {
        CursorIcon::System(icon)
    }
}

impl From<CustomCursor> for CursorIcon {
    fn from(cursor: CustomCursor) -> Self {
        CursorIcon::Custom(cursor)
    }
}

/// Custom cursor image data.
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash)]
pub enum CustomCursor {
    /// Image to use as a cursor.
    Image {
        /// The image must be in 8 bit int or 32 bit float rgba. PNG images
        /// work well for this.
        handle: Handle<Image>,
        /// X and Y coordinates of the hotspot in pixels. The hotspot must be
        /// within the image bounds.
        hotspot: (u16, u16),
    },
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    /// A URL to an image to use as the cursor.
    Url {
        /// Web URL to an image to use as the cursor. PNGs preferred. Cursor
        /// creation can fail if the image is invalid or not reachable.
        url: String,
        /// X and Y coordinates of the hotspot in pixels. The hotspot must be
        /// within the image bounds.
        hotspot: (u16, u16),
    },
}

pub fn update_cursors(
    mut commands: Commands,
    windows: Query<(Entity, Ref<CursorIcon>), With<Window>>,
    cursor_cache: Res<CustomCursorCache>,
    images: Res<Assets<Image>>,
    mut queue: Local<HashSet<Entity>>,
) {
    for (entity, cursor) in windows.iter() {
        if !(queue.remove(&entity) || cursor.is_changed()) {
            continue;
        }

        let cursor_source = match cursor.as_ref() {
            CursorIcon::Hidden => CursorSource::Hidden,
            CursorIcon::Custom(CustomCursor::Image { handle, hotspot }) => {
                let cache_key = match handle.id() {
                    AssetId::Index { index, .. } => {
                        CustomCursorCacheKey::AssetIndex(index.to_bits())
                    }
                    AssetId::Uuid { uuid } => CustomCursorCacheKey::AssetUuid(uuid.as_u128()),
                };

                if cursor_cache.0.contains_key(&cache_key) {
                    CursorSource::CustomCached(cache_key)
                } else {
                    let Some(image) = images.get(handle) else {
                        warn!(
                            "Cursor image {handle:?} is not loaded yet and couldn't be used. Trying again next frame."
                        );
                        queue.insert(entity);
                        continue;
                    };
                    let Some(rgba) = image_to_rgba_pixels(image) else {
                        warn!("Cursor image {handle:?} not accepted because it's not rgba8 or rgba32float format");
                        continue;
                    };

                    let width = image.texture_descriptor.size.width;
                    let height = image.texture_descriptor.size.height;
                    let source = match bevy_winit::WinitCustomCursor::from_rgba(
                        rgba,
                        width as u16,
                        height as u16,
                        hotspot.0,
                        hotspot.1,
                    ) {
                        Ok(source) => source,
                        Err(err) => {
                            warn!("Cursor image {handle:?} is invalid: {err}");
                            continue;
                        }
                    };

                    CursorSource::Custom((cache_key, source))
                }
            }
            #[cfg(all(target_family = "wasm", target_os = "unknown"))]
            CursorIcon::Custom(CustomCursor::Url { url, hotspot }) => {
                let cache_key = CustomCursorCacheKey::Url(url.clone());

                if cursor_cache.0.contains_key(&cache_key) {
                    CursorSource::CustomCached(cache_key)
                } else {
                    use bevy_winit::CustomCursorExtWebSys;
                    let source =
                        bevy_winit::WinitCustomCursor::from_url(url.clone(), hotspot.0, hotspot.1);
                    CursorSource::Custom((cache_key, source))
                }
            }
            CursorIcon::System(system_cursor_icon) => {
                CursorSource::System(convert_system_cursor_icon(*system_cursor_icon))
            }
        };

        commands
            .entity(entity)
            .insert(PendingCursor(Some(cursor_source)));
    }
}

/// Resets the cursor to the default icon when `CursorIcon` is removed.
pub fn on_remove_cursor_icon(trigger: Trigger<OnRemove, CursorIcon>, mut commands: Commands) {
    commands
        .entity(trigger.entity())
        .insert(PendingCursor(Some(CursorSource::System(
            convert_system_cursor_icon(SystemCursorIcon::Default),
        ))));
}

/// Returns the image data as a `Vec<u8>`.
/// Only supports rgba8 and rgba32float formats.
fn image_to_rgba_pixels(image: &Image) -> Option<Vec<u8>> {
    match image.texture_descriptor.format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint => Some(image.data.clone()),
        TextureFormat::Rgba32Float => Some(
            image
                .data
                .chunks(4)
                .map(|chunk| {
                    let chunk = chunk.try_into().unwrap();
                    let num = bytemuck::cast_ref::<[u8; 4], f32>(chunk);
                    (num * 255.0) as u8
                })
                .collect(),
        ),
        _ => None,
    }
}

/// We need to add this here because we can't do it inside of [`bevy_winit`] due to issues
/// with circular dependencies. Before we had custom cursors, these cursor related options
/// would just be created automatically with the [`Window`] component, now however, this
/// has to be made with the creation of the window so that users can set the visibility
/// of it after the window is created.
fn add_cursor_to_windows(
    mut commands: Commands,
    mut ev_window_created: EventReader<WindowCreated>,
) {
    for WindowCreated { window: entity } in ev_window_created.read() {
        commands.entity(*entity).insert(CursorIcon::default());
    }
}
