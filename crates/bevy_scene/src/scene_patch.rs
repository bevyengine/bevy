use crate::{
    ApplySceneError, ResolveSceneError, ResolvedSceneListRoot, ResolvedSceneRoot, Scene,
    SceneDependencies, SceneList,
};
use alloc::sync::Arc;
use bevy_asset::{Asset, AssetServer, Assets, Handle, LoadFromPath, UntypedHandle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::BundleScratch,
    component::Component,
    entity::Entity,
    template::FromTemplate,
    world::{EntityWorldMut, World},
};
use bevy_reflect::TypePath;
use thiserror::Error;

/// An [`Asset`] that holds a [`Scene`], tracks its dependencies, and holds the [`ResolvedSceneRoot`] (after the [`Scene`] has been loaded and resolved).
#[derive(Asset, TypePath)]
pub struct ScenePatch {
    /// A [`Scene`].
    pub scene: Option<Box<dyn Scene>>,
    /// The dependencies of `scene` (populated using [`Scene::register_dependencies`]). These are "asset dependencies" and will affect the load state.
    #[dependency]
    pub dependencies: Vec<UntypedHandle>,
    /// The [`ResolvedSceneRoot`], if exists. This is populated after the [`Scene`] has been loaded and resolved
    // TODO: consider breaking this out to prevent mutating asset events when resolved. Assets as Entities will enable this!
    // TODO: This Arc exists to allow nested ResolvedSceneRoot::apply when borrowing inherited ScenePatch assets (see the ResolvedSceneRoot::apply implementation).
    pub resolved: Option<Arc<ResolvedSceneRoot>>,
}

impl ScenePatch {
    /// Kicks off a load of the `scene`. This enumerates the scene's dependencies using [`Scene::register_dependencies`], loads
    /// them using the given [`AssetServer`], and assigns the resulting asset handles to [`ScenePatch::dependencies`].
    pub fn load<P: Scene>(mut assets: &AssetServer, scene: P) -> Self {
        Self::load_with(&mut assets, scene)
    }

    /// Same as [`Self::load`], but allows passing in any [`LoadFromPath`] impl for more general
    /// loading cases.
    pub fn load_with<P: Scene>(load_from_path: &mut impl LoadFromPath, scene: P) -> Self {
        let mut dependencies = SceneDependencies::default();
        scene.register_dependencies(&mut dependencies);
        let dependencies = dependencies
            .iter()
            .map(|i| load_from_path.load_from_path_erased(i.type_id, i.path.clone()))
            .collect::<Vec<_>>();
        ScenePatch {
            scene: Some(Box::new(scene)),
            dependencies,
            resolved: None,
        }
    }

    /// Resolves the current `scene` (using [`Scene::resolve`]). This should only be called after every dependency has loaded from the `scene`'s
    /// [`Scene::register_dependencies`]. If successful, it will store the resolved result in [`ScenePatch::resolved`].
    pub fn resolve(
        &mut self,
        assets: &AssetServer,
        patches: &Assets<ScenePatch>,
    ) -> Result<(), ResolveSceneError> {
        let scene = self.scene.take().ok_or(ResolveSceneError::MissingScene)?;
        self.resolved = Some(Arc::new(ResolvedSceneRoot::resolve(
            scene, assets, patches,
        )?));
        Ok(())
    }

    /// Spawns the scene in `world` as a new entity. This should only be called after [`ScenePatch::resolve`].
    pub fn spawn<'w>(&self, world: &'w mut World) -> Result<EntityWorldMut<'w>, SpawnSceneError> {
        let resolved = self
            .resolved
            .as_deref()
            .ok_or(SpawnSceneError::UnresolvedSceneError)?;
        resolved
            .spawn(world)
            .map_err(SpawnSceneError::ApplySceneError)
    }

    /// Applies the scene to the given `entity`. This should only be called after [`ScenePatch::resolve`]
    pub fn apply<'w>(&self, entity: &'w mut EntityWorldMut) -> Result<(), SpawnSceneError> {
        let resolved = self
            .resolved
            .as_deref()
            .ok_or(SpawnSceneError::UnresolvedSceneError)?;
        resolved
            .apply(entity, &mut BundleScratch::default())
            .map_err(SpawnSceneError::ApplySceneError)
    }
}

/// An [`Error`] that occurs during scene spawning.
#[derive(Error, Debug)]
pub enum SpawnSceneError {
    /// Failed to apply a [`ResolvedScene`].
    ///
    /// [`ResolvedScene`]: crate::ResolvedScene
    #[error(transparent)]
    ApplySceneError(#[from] ApplySceneError),
    #[error(transparent)]
    /// Calling [`Scene::resolve`] failed.
    ResolveSceneError(#[from] ResolveSceneError),
    /// Attempted to spawn a scene that has not been resolved yet.
    #[error("This scene has not been resolved yet and cannot be spawned. It is likely waiting for dependencies to load")]
    UnresolvedSceneError,
}

/// A component that, when added, will queue applying the given [`ScenePatch`] after the scene and its dependencies have been loaded and resolved.
#[derive(Component, FromTemplate, Deref, DerefMut)]
pub struct ScenePatchInstance(pub Handle<ScenePatch>);

/// An [`Asset`] that holds a [`SceneList`], tracks its dependencies, and holds a [`ResolvedSceneListRoot`] (after the [`SceneList`] has been loaded and resolved)
#[derive(Asset, TypePath)]
pub struct SceneListPatch {
    /// A [`SceneList`].
    pub scene_list: Option<Box<dyn SceneList>>,

    /// The dependencies of `scene_list` (populated using [`SceneList::register_dependencies`]). These are "asset dependencies" and will affect the load state.
    #[dependency]
    pub dependencies: Vec<UntypedHandle>,

    /// The [`ResolvedSceneListRoot`], if exists. This is populated after the scene list and its dependencies have been loaded and resolved.
    // TODO: consider breaking this out to prevent mutating asset events when resolved
    pub resolved: Option<ResolvedSceneListRoot>,
}

impl SceneListPatch {
    /// Kicks off a load of the `scene_list`. This enumerates the scene list's dependencies using [`SceneList::register_dependencies`], loads
    /// them using the given [`AssetServer`], and assigns the resulting asset handles to [`SceneListPatch::dependencies`].
    pub fn load<L: SceneList>(assets: &AssetServer, scene_list: L) -> Self {
        let mut dependencies = SceneDependencies::default();
        scene_list.register_dependencies(&mut dependencies);
        let dependencies = dependencies
            .iter()
            .map(|dep| assets.load_builder().load_erased(dep.type_id, &dep.path))
            .collect::<Vec<_>>();
        SceneListPatch {
            scene_list: Some(Box::new(scene_list)),
            dependencies,
            resolved: None,
        }
    }

    /// Resolves the current `scene` (using [`SceneList::resolve_list`]). This should only be called after every dependency has loaded from the `scene_list`'s
    /// [`SceneList::register_dependencies`].
    pub fn resolve(
        &mut self,
        assets: &AssetServer,
        patches: &Assets<ScenePatch>,
    ) -> Result<(), ResolveSceneError> {
        let scene_list = self
            .scene_list
            .take()
            .ok_or(ResolveSceneError::MissingScene)?;
        self.resolved = Some(ResolvedSceneListRoot::resolve(scene_list, assets, patches)?);
        Ok(())
    }

    /// Spawns the scene list in `world` as new entities. This should only be called after [`SceneListPatch::resolve`].
    pub fn spawn<'w>(&self, world: &'w mut World) -> Result<Vec<Entity>, SpawnSceneError> {
        self.spawn_with(world, |_| {})
    }

    /// Spawns the scene list in `world` as new entities. This should only be called after [`SceneListPatch::resolve`].
    pub(crate) fn spawn_with<'w>(
        &self,
        world: &'w mut World,
        func: impl Fn(&mut EntityWorldMut),
    ) -> Result<Vec<Entity>, SpawnSceneError> {
        let resolved = self
            .resolved
            .as_ref()
            .ok_or(SpawnSceneError::UnresolvedSceneError)?;
        resolved
            .spawn_with(world, func)
            .map_err(SpawnSceneError::ApplySceneError)
    }
}
