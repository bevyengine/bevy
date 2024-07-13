use std::{marker::PhantomData, sync::OnceLock};

use bevy_app::{App, Plugin};
use bevy_ecs::system::{Res, Resource, SystemParam};

pub use bevy_asset_macros::AssetPack;

use crate::AssetServer;

/// This trait (and associated derive macro) provides syntax sugar for loading related assets
/// whose sources are known at compile time. When using the derive macro, all fields must have
/// either an `#[embedded("...")]` attribute or a `#[load(...)]` attribute.
///
/// `embedded` takes as argument a relative path to the embedded asset, while
/// `load` takes as argument an expression that implements `Into<AssetPath<'_>>`. This could be a string
/// literal, or something else depending on the use-case.
///
/// The derive macro also provides a top level attribute `src_path` to override the root
/// directory used by internal calls to `embedded_asset`. It's necessary for crates with-"src" root
/// directories, such as the cargo example.
///
/// For accessing an `AssetPack`, see `AssetPackPlugin`, `Pack` and `GetPack`
/// For a usage example, see the `asset_pack` example.
pub trait AssetPack: Send + Sync + 'static {
    fn init(app: &mut App);
    fn load(asset_server: &AssetServer) -> Self;
}

/// Provides setup for loading an asset pack of type `T`
pub struct AssetPackPlugin<T: AssetPack>(PhantomData<T>);

impl<T: AssetPack> Default for AssetPackPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: AssetPack> Plugin for AssetPackPlugin<T> {
    fn build(&self, app: &mut App) {
        T::init(app);
        app.init_resource::<Pack<T>>();
    }
}

/// A `Resource` that wraps access to an `AssetPack`
#[derive(Resource)]
struct Pack<T: AssetPack>(OnceLock<T>);

impl<T: AssetPack> Pack<T> {
    fn get(&self, asset_server: &AssetServer) -> &T {
        self.0.get_or_init(|| T::load(asset_server))
    }
}

impl<T: AssetPack> Default for Pack<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// A `SystemParam` that wraps `Pack<T>` and `AssetServer` for simple access
#[derive(SystemParam)]
pub struct GetPack<'w, T: AssetPack> {
    handles: Res<'w, Pack<T>>,
    asset_server: Res<'w, AssetServer>,
}

impl<'w, T: AssetPack> GetPack<'w, T> {
    pub fn get(&self) -> &T {
        self.handles.get(&self.asset_server)
    }
}
