mod assets;
mod components;
pub mod rendering;

use core::{hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;

pub use {assets::Material2d, components::MeshMaterial2d};

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`Material2d`]
/// asset type (which includes [`Material2d`] types).
pub struct Material2dPlugin<M: Material2d>(PhantomData<M>);

impl<M: Material2d> Default for Material2dPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material2d> Plugin for Material2dPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<M>().register_type::<MeshMaterial2d<M>>();

        app.add_plugins(rendering::Material2dRenderingPlugin::<M>::default());
    }
}
