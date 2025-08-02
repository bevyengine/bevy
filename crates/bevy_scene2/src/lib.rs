#![allow(missing_docs)]

pub mod prelude {
    pub use crate::{
        bsn, bsn_list, on, CommandsSpawnScene, LoadScene, PatchGetTemplate, PatchTemplate, Scene,
        SceneList, ScenePatchInstance, SpawnScene,
    };
}

mod resolved_scene;
mod scene;
mod scene_list;
mod scene_patch;
mod spawn;

pub use bevy_scene2_macros::*;

pub use resolved_scene::*;
pub use scene::*;
pub use scene_list::*;
pub use scene_patch::*;
pub use spawn::*;

use bevy_app::{App, Plugin, Update};
use bevy_asset::{AssetApp, AssetPath, AssetServer, Handle};
use bevy_ecs::{prelude::*, system::IntoObserverSystem, template::Template};
use std::marker::PhantomData;

#[derive(Default)]
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QueuedScenes>()
            .init_asset::<ScenePatch>()
            .add_systems(Update, (resolve_scene_patches, spawn_queued).chain());
    }
}

/// This is used by the [`bsn!`] macro to generate compile-time only references to symbols. Currently this is used
/// to add IDE support for nested type names, as it allows us to pass the input Ident from the input to the output code.
pub const fn touch_type<T>() {}

pub trait LoadScene {
    fn load_scene<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
        scene: impl Scene,
    ) -> Handle<ScenePatch>;
}

impl LoadScene for AssetServer {
    fn load_scene<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
        scene: impl Scene,
    ) -> Handle<ScenePatch> {
        let scene = ScenePatch::load(self, scene);
        self.load_with_path(path, scene)
    }
}

pub struct OnTemplate<I, E, B, M>(pub I, pub PhantomData<fn() -> (E, B, M)>);

impl<I: IntoObserverSystem<E, B, M> + Clone, E: EntityEvent, B: Bundle, M: 'static> Template
    for OnTemplate<I, E, B, M>
{
    type Output = ();

    fn build(&mut self, entity: &mut EntityWorldMut) -> Result<Self::Output> {
        entity.observe(self.0.clone());
        Ok(())
    }
}

impl<
        I: IntoObserverSystem<E, B, M> + Clone + Send + Sync,
        E: EntityEvent,
        B: Bundle,
        M: 'static,
    > Scene for OnTemplate<I, E, B, M>
{
    fn patch(
        &self,
        _assets: &AssetServer,
        _patches: &bevy_asset::Assets<ScenePatch>,
        scene: &mut ResolvedScene,
    ) {
        scene.push_template(OnTemplate(self.0.clone(), PhantomData));
    }
}

pub fn on<I: IntoObserverSystem<E, B, M>, E: EntityEvent, B: Bundle, M: 'static>(
    observer: I,
) -> OnTemplate<I, E, B, M> {
    OnTemplate(observer, PhantomData)
}
