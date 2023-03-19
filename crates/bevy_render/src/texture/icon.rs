use std::path::PathBuf;

use crate::texture::{Image, MaybeImage};
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{AssetServer, Assets, Handle, LoadState};
use bevy_ecs::{
    prelude::{Component, Entity, NonSendMut, Query, Res},
    query::With,
    schedule::IntoSystemConfigs,
    system::Commands,
};
use bevy_log::{error, info};
use bevy_window::Window;
use bevy_winit::WinitWindows;

/// An icon that can be placed at the top left of the window.
#[derive(Component, Debug)]
pub struct WindowIcon(Option<Handle<Image>>);

impl WindowIcon {
    pub fn new(maybe_handle: Option<Handle<Image>>) -> Self {
        Self(maybe_handle)
    }
}

#[derive(Debug)]
pub struct WindowIconPlugin(PathBuf);

/// Set the window icon. Lower-level systems are also available if something fancier is desired.
///
/// # Example
/// TBD
///
/// This functionality [is only known to work on Windows and X11](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_window_icon).
impl Plugin for WindowIconPlugin {
    fn build(&self, app: &mut crate::App) {
        app.add_systems(Startup, insert_component(self.0.clone()))
            .add_systems(Update, set_window_icon.run_if(image_asset_loaded()));
    }
}

impl WindowIconPlugin {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

pub fn insert_component(
    path: PathBuf,
) -> impl FnMut(Commands, Query<Entity, With<Window>>, Res<AssetServer>) {
    move |mut commands: Commands, query: Query<_, _>, asset_server: Res<_>| {
        let icon_handle = asset_server.load(path.clone());

        for id in query.iter() {
            commands
                .entity(id)
                .insert(WindowIcon::new(Some(icon_handle.clone())));
        }
    }
}

pub fn set_window_icon(
    images: Res<Assets<Image>>,
    query: Query<(Entity, &WindowIcon)>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    let WinitWindows {
        windows,
        entity_to_winit,
        ..
    } = &mut *winit_windows;

    for (id, WindowIcon(maybe_handle)) in query.iter() {
        let Some(window_id) = entity_to_winit.get(&id) else {
            error!("entity with window icon ({:?}) not associated with a winit window", id);
            continue;
        };
        let window = windows.get(window_id).unwrap();

        if let Some(handle) = maybe_handle {
            info!("attempting to set window icon to an image");
            window.set_window_icon(MaybeImage::from(images.get(handle)).into());
        } else {
            info!("window icon set to None");
            window.set_window_icon(None);
        }
    }
}

pub fn image_asset_loaded() -> impl FnMut(Res<AssetServer>, Query<&WindowIcon>) -> bool {
    let mut has_run = false;

    move |asset_server: Res<_>, query: Query<_>| {
        if has_run {
            return false;
        }

        let loaded = query.iter().all(|WindowIcon(maybe_handle)| {
            maybe_handle
                .as_ref()
                .map(|handle| asset_server.get_load_state(handle.clone_weak()) == LoadState::Loaded)
                .unwrap_or(true)
        });

        if loaded {
            has_run = true;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::WindowIcon;
    use crate::texture::Image;
    use crate::texture::WindowIconPlugin;
    use bevy_app::prelude::App;
    use bevy_asset::AddAsset;
    use bevy_asset::Assets;

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugin(bevy_core::TaskPoolPlugin::default())
            .add_plugin(bevy_core::TypeRegistrationPlugin::default())
            .add_plugin(bevy_asset::AssetPlugin::default())
            .init_non_send_resource::<bevy_winit::WinitWindows>()
            .init_resource::<bevy_winit::WinitSettings>()
            .set_runner(bevy_winit::winit_runner)
            .add_asset::<Image>();
        app
    }

    #[test]
    fn window_icon() {
        let mut app = make_app();

        app.add_plugin(WindowIconPlugin::new("some/image.png".into()));

        let image_handle = app
            .world
            .resource_mut::<Assets<Image>>()
            .add(Image::default());

        assert!(app
            .world
            .resource::<Assets<Image>>()
            .get(&image_handle)
            .is_some());

        let entity = app.world.spawn(WindowIcon::new(Some(image_handle))).id();

        app.update();

        assert!(app.world.get::<WindowIcon>(entity).is_some());
    }
}
