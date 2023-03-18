use crate::texture::Image;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    prelude::{Component, Entity, NonSendMut, Query, Res},
    system::Commands,
};
use bevy_log::{error, info};
use bevy_winit::WinitWindows;
use winit::window::Icon;

/// An icon that can be placed at the top left of the window.
#[derive(Component, Debug)]
pub struct WindowIcon(pub Option<Handle<Image>>);

/// Set or unset the window icon, depending on whether `Some(image_handle)` or `None` is provided.
///
/// # Example
/// ```rust,no_run
/// use bevy_app::{App, Startup, Update};
/// use bevy_asset::AssetServer;
/// use bevy_ecs::prelude::*;
/// use bevy_render::texture::{set_window_icon, WindowIcon};
///
/// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
///     let icon_handle = asset_server.load("branding/icon.png");
///     commands.spawn(WindowIcon(Some(icon_handle)));
/// }
///
/// fn main() {
///   App::new()
///     .add_systems(Startup, setup)
///     .add_systems(Update, set_window_icon)
///     .run();
/// }
/// ```
///
/// This functionality [is only known to work on Windows and X11](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_window_icon).
pub fn set_window_icon(
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut WindowIcon)>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    for (entity, window_icon) in query.iter_mut() {
        let icon = {
            if let Some(image) = &window_icon.0 {
                let Some(icon) = images.get(image) else { continue };
                let result: Result<Icon, _> = icon.clone().try_into();

                match result {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        error!("failed to set window icon: {}", err);
                        commands.entity(entity).remove::<WindowIcon>();
                        continue;
                    }
                }
            } else {
                None
            }
        };

        if let Some(icon) = &icon {
            for (_id, window) in &mut winit_windows.windows {
                window.set_window_icon(Some(icon.clone()));
            }
        } else {
            for (_id, window) in &mut winit_windows.windows {
                window.set_window_icon(None);
            }
        }

        info!("window icon set");
        commands.entity(entity).remove::<WindowIcon>();
    }
}
