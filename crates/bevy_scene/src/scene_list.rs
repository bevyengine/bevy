use crate::{ResolveContext, ResolveSceneError, ResolvedScene, Scene, SceneDependencies};
use variadics_please::all_tuples;

/// This behaves like a list of [`Scene`], where each entry in the list is a new entity (see [`Scene`] for more details).
///
/// [`Scene`] is to [`Entity`] as [`SceneList`] is to [`Vec<Entity>`].
///
/// [`Entity`]: bevy_ecs::entity::Entity
pub trait SceneList: Send + Sync + 'static {
    /// This will apply the changes described in this [`SceneList`] to the given [`Vec<ResolvedScene>`]. This should not be called until all of
    /// the dependencies in [`Scene::register_dependencies`] have been loaded.
    fn resolve_list(
        &self,
        context: &mut ResolveContext,
        scenes: &mut Vec<ResolvedScene>,
    ) -> Result<(), ResolveSceneError>;

    /// [`SceneList`] can have [`Asset`] dependencies, which _must_ be loaded before calling [`SceneList::resolve_list`] or it might return a
    /// [`ResolveSceneError`]!
    ///
    /// [`Asset`]: bevy_asset::Asset
    fn register_dependencies(&self, dependencies: &mut SceneDependencies);
}

/// Corresponds to a single member of a [`SceneList`] (an [`Entity`] with an `S` [`Scene`]).
///
/// [`Entity`]: bevy_ecs::entity::Entity
pub struct EntityScene<S>(pub S);

impl<S: Scene> SceneList for EntityScene<S> {
    fn resolve_list(
        &self,
        context: &mut ResolveContext,
        scenes: &mut Vec<ResolvedScene>,
    ) -> Result<(), ResolveSceneError> {
        let mut resolved_scene = ResolvedScene::default();
        self.0.resolve(context, &mut resolved_scene)?;
        scenes.push(resolved_scene);
        Ok(())
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        self.0.register_dependencies(dependencies);
    }
}

macro_rules! scene_list_impl {
    ($($list: ident),*) => {
        impl<$($list: SceneList),*> SceneList for ($($list,)*) {
            fn resolve_list(&self, _context: &mut ResolveContext, _scenes: &mut Vec<ResolvedScene>) -> Result<(), ResolveSceneError> {
                #[expect(
                    clippy::allow_attributes,
                    reason = "This is inside a macro, and as such, may not trigger in all cases."
                )]
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($list,)*) = self;
                $($list.resolve_list(_context, _scenes)?;)*
                Ok(())
            }

            fn register_dependencies(&self, _dependencies: &mut SceneDependencies) {
                #[expect(
                    clippy::allow_attributes,
                    reason = "This is inside a macro, and as such, may not trigger in all cases."
                )]
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($list,)*) = self;
                $($list.register_dependencies(_dependencies);)*
            }
       }
    }
}

all_tuples!(scene_list_impl, 0, 12, P);

impl<S: Scene> SceneList for Vec<S> {
    fn resolve_list(
        &self,
        context: &mut ResolveContext,
        scenes: &mut Vec<ResolvedScene>,
    ) -> Result<(), ResolveSceneError> {
        for scene in self {
            let mut resolved_scene = ResolvedScene::default();
            scene.resolve(context, &mut resolved_scene)?;
            scenes.push(resolved_scene);
        }
        Ok(())
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        for scene in self {
            scene.register_dependencies(dependencies);
        }
    }
}

impl SceneList for Vec<Box<dyn Scene>> {
    fn resolve_list(
        &self,
        context: &mut ResolveContext,
        scenes: &mut Vec<ResolvedScene>,
    ) -> Result<(), ResolveSceneError> {
        for scene in self {
            let mut resolved_scene = ResolvedScene::default();
            scene.resolve(context, &mut resolved_scene)?;
            scenes.push(resolved_scene);
        }
        Ok(())
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        for scene in self {
            scene.register_dependencies(dependencies);
        }
    }
}
