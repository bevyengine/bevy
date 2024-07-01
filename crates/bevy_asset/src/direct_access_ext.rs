//! Add methods on `World` to simplify loading assets when all
//! you have is a `World`.

use bevy_ecs::world::World;

use crate::{meta::Settings, Asset, AssetPath, AssetServer, Assets, Handle};

pub trait DirectAssetAccessExt {
    /// Insert an asset similarly to [`Assets::add`].
    fn add_asset<A: Asset>(&mut self, asset: impl Into<A>) -> Handle<A>;

    /// Load an asset similarly to [`AssetServer::load`].
    fn load_asset<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A>;

    /// Load an asset with settings, similarly to [`AssetServer::load_with_settings`].
    fn load_asset_with_settings<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A>;
}
impl DirectAssetAccessExt for World {
    /// Insert an asset similarly to [`Assets::add`].
    ///
    /// # Panics
    /// If `self` doesn't have an [`AssetServer`] resource initialized yet.
    fn add_asset<'a, A: Asset>(&mut self, asset: impl Into<A>) -> Handle<A> {
        self.resource_mut::<Assets<A>>().add(asset)
    }

    /// Load an asset similarly to [`AssetServer::load`].
    ///
    /// # Panics
    /// If `self` doesn't have an [`AssetServer`] resource initialized yet.
    fn load_asset<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        self.resource::<AssetServer>().load(path)
    }
    /// Load an asset with settings, similarly to [`AssetServer::load_with_settings`].
    ///
    /// # Panics
    /// If `self` doesn't have an [`AssetServer`] resource initialized yet.
    fn load_asset_with_settings<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A> {
        self.resource::<AssetServer>()
            .load_with_settings(path, settings)
    }
}
