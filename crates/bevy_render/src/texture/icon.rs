use crate::texture::Image;
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{AssetServer, Assets, Handle, LoadState};
use bevy_ecs::{
    prelude::{Component, Entity, NonSendMut, Query, Res},
    query::With,
    schedule::IntoSystemConfigs,
    system::Commands,
};
use bevy_log::{error, info, warn};
use bevy_window::Window;
use bevy_winit::{Icon, WinitWindows};
use std::{fmt::Display, path::PathBuf};
use thiserror::Error;

/// Set the window icon. Lower-level systems are also available if something fancier is desired.
///
/// # Example
/// TBD
///
/// This functionality [is only known to work on Windows and X11](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_window_icon).
#[derive(Component, Debug)]
pub struct WindowIcon(Option<Handle<Image>>);

impl WindowIcon {
    pub fn new(maybe_handle: Option<Handle<Image>>) -> Self {
        Self(maybe_handle)
    }
}

#[derive(Debug)]
pub struct WindowIconPlugin(PathBuf);

impl Plugin for WindowIconPlugin {
    fn build(&self, app: &mut crate::App) {
        app.add_systems(Startup, insert_component(Some(self.0.clone())))
            .add_systems(Update, set_window_icon.run_if(image_asset_loaded()));
    }
}

impl WindowIconPlugin {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

pub fn insert_component(
    path: Option<PathBuf>,
) -> impl FnMut(Commands, Query<Entity, With<Window>>, Res<AssetServer>) {
    move |mut commands: Commands, query: Query<_, _>, asset_server: Res<_>| {
        let maybe_handle = path.as_ref().map(|path| asset_server.load(path.clone()));

        for id in query.iter() {
            commands
                .entity(id)
                .insert(WindowIcon::new(maybe_handle.clone()));
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

#[derive(Error, Debug)]
pub struct IconCoversionError(String);

impl Display for IconCoversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Convert an [`Image`] to `bevy_winit::Icon`.
impl TryInto<Icon> for Image {
    type Error = IconCoversionError;

    fn try_into(self) -> Result<Icon, Self::Error> {
        let icon = match self.try_into_dynamic() {
            Ok(icon) => icon,

            Err(err) => {
                return Err(IconCoversionError(format!(
                    "icon conversion error: {}",
                    err
                )));
            }
        };

        let width = icon.width();
        let height = icon.height();
        let data = icon.into_rgba8().into_raw();

        Icon::from_rgba(data, width, height)
            .map_err(|err| IconCoversionError(format!("icon conversion error: {}", err)))
    }
}

/// Work around the orphan rule for converting `Option<&Image>` to `Option<Icon>`.
#[derive(Debug)]
pub(crate) struct MaybeImage<'a>(Option<&'a Image>);

impl<'a> From<Option<&'a Image>> for MaybeImage<'a> {
    fn from(value: Option<&'a Image>) -> Self {
        Self(value)
    }
}

impl<'a> From<MaybeImage<'a>> for Option<Icon> {
    fn from(value: MaybeImage<'a>) -> Self {
        match value {
            MaybeImage(Some(image)) => {
                let result: Result<Icon, _> = image.clone().try_into();
                match result {
                    Ok(icon) => Some(icon),

                    Err(err) => {
                        error!("failed to convert image to icon: {}", err);
                        None
                    }
                }
            }

            MaybeImage(None) => {
                warn!("window icon image asset not loaded");
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::WindowIcon;
    use crate::texture::Image;
    use crate::texture::WindowIconPlugin;
    use bevy_app::prelude::App;
    use bevy_asset::AddAsset;
    use bevy_asset::Assets;
    use bevy_winit::Icon;

    #[test]
    fn into_icon() {
        let image = Image::default();
        let _: Icon = image.try_into().unwrap();
    }

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
