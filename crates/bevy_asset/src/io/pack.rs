use std::{marker::PhantomData, sync::OnceLock};

use bevy_app::{App, Plugin};
use bevy_ecs::system::{Res, Resource, SystemParam};

pub use bevy_asset_macros::AssetPack;

use crate::AssetServer;

pub trait AssetPack: Send + Sync + 'static {
    fn init(app: &mut App);
    fn load(asset_server: &AssetServer) -> Self;
}

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
