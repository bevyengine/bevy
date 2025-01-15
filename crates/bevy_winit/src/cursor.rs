//! Components to customize winit cursor

use crate::{
    converters::convert_system_cursor_icon,
    state::{CursorSource, PendingCursor},
};
#[cfg(feature = "custom_cursor")]
use crate::{
    custom_cursor::{
        calculate_effective_rect, extract_and_transform_rgba_pixels, extract_rgba_pixels,
        CustomCursorPlugin,
    },
    state::{CustomCursorCache, CustomCursorCacheKey},
    WinitCustomCursor,
};
use bevy_app::{App, Last, Plugin};
#[cfg(feature = "custom_cursor")]
use bevy_asset::{Assets, Handle};
#[cfg(feature = "custom_cursor")]
use bevy_ecs::system::Res;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Local, Query},
    world::{OnRemove, Ref},
};
#[cfg(feature = "custom_cursor")]
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
#[cfg(feature = "custom_cursor")]
use bevy_math::URect;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::HashSet;
use bevy_window::{SystemCursorIcon, Window};
#[cfg(feature = "custom_cursor")]
use tracing::warn;

pub(crate) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "custom_cursor")]
        app.add_plugins(CustomCursorPlugin);

        app.register_type::<CursorIcon>()
            .add_systems(Last, update_cursors);

        app.add_observer(on_remove_cursor_icon);
    }
}

/// Insert into a window entity to set the cursor for that window.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub enum CursorIcon {
    #[cfg(feature = "custom_cursor")]
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

#[cfg(feature = "custom_cursor")]
impl From<CustomCursor> for CursorIcon {
    fn from(cursor: CustomCursor) -> Self {
        CursorIcon::Custom(cursor)
    }
}

#[cfg(feature = "custom_cursor")]
/// Custom cursor image data.
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash)]
pub enum CustomCursor {
    /// Image to use as a cursor.
    Image {
        /// The image must be in 8 bit int or 32 bit float rgba. PNG images
        /// work well for this.
        handle: Handle<Image>,
        /// The (optional) texture atlas used to render the image.
        texture_atlas: Option<TextureAtlas>,
        /// Whether the image should be flipped along its x-axis.
        flip_x: bool,
        /// Whether the image should be flipped along its y-axis.
        flip_y: bool,
        /// An optional rectangle representing the region of the image to
        /// render, instead of rendering the full image. This is an easy one-off
        /// alternative to using a [`TextureAtlas`].
        ///
        /// When used with a [`TextureAtlas`], the rect is offset by the atlas's
        /// minimal (top-left) corner position.
        rect: Option<URect>,
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

fn update_cursors(
    mut commands: Commands,
    windows: Query<(Entity, Ref<CursorIcon>), With<Window>>,
    #[cfg(feature = "custom_cursor")] cursor_cache: Res<CustomCursorCache>,
    #[cfg(feature = "custom_cursor")] images: Res<Assets<Image>>,
    #[cfg(feature = "custom_cursor")] texture_atlases: Res<Assets<TextureAtlasLayout>>,
    mut queue: Local<HashSet<Entity>>,
) {
    for (entity, cursor) in windows.iter() {
        if !(queue.remove(&entity) || cursor.is_changed()) {
            continue;
        }

        let cursor_source = match cursor.as_ref() {
            #[cfg(feature = "custom_cursor")]
            CursorIcon::Custom(CustomCursor::Image {
                handle,
                texture_atlas,
                flip_x,
                flip_y,
                rect,
                hotspot,
            }) => {
                let cache_key = CustomCursorCacheKey::Image {
                    id: handle.id(),
                    texture_atlas_layout_id: texture_atlas.as_ref().map(|a| a.layout.id()),
                    texture_atlas_index: texture_atlas.as_ref().map(|a| a.index),
                    flip_x: *flip_x,
                    flip_y: *flip_y,
                    rect: *rect,
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

                    let (rect, needs_sub_image) =
                        calculate_effective_rect(&texture_atlases, image, texture_atlas, rect);

                    let maybe_rgba = if *flip_x || *flip_y || needs_sub_image {
                        extract_and_transform_rgba_pixels(image, *flip_x, *flip_y, rect)
                    } else {
                        extract_rgba_pixels(image)
                    };

                    let Some(rgba) = maybe_rgba else {
                        warn!("Cursor image {handle:?} not accepted because it's not rgba8 or rgba32float format");
                        continue;
                    };

                    let source = match WinitCustomCursor::from_rgba(
                        rgba,
                        rect.width() as u16,
                        rect.height() as u16,
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
            #[cfg(all(
                feature = "custom_cursor",
                target_family = "wasm",
                target_os = "unknown"
            ))]
            CursorIcon::Custom(CustomCursor::Url { url, hotspot }) => {
                let cache_key = CustomCursorCacheKey::Url(url.clone());

                if cursor_cache.0.contains_key(&cache_key) {
                    CursorSource::CustomCached(cache_key)
                } else {
                    use crate::CustomCursorExtWebSys;
                    let source = WinitCustomCursor::from_url(url.clone(), hotspot.0, hotspot.1);
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
fn on_remove_cursor_icon(trigger: Trigger<OnRemove, CursorIcon>, mut commands: Commands) {
    // Use `try_insert` to avoid panic if the window is being destroyed.
    commands
        .entity(trigger.target())
        .try_insert(PendingCursor(Some(CursorSource::System(
            convert_system_cursor_icon(SystemCursorIcon::Default),
        ))));
}
