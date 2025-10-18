use crate::{PatchContext, ResolvedScene, Scene};
use bevy_asset::AssetPath;
use variadics_please::all_tuples;

pub trait SceneList: Send + Sync + 'static {
    fn patch_list(&self, context: &mut PatchContext, scenes: &mut Vec<ResolvedScene>);

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>);
}

pub struct EntityScene<S>(pub S);

impl<S: Scene> SceneList for EntityScene<S> {
    fn patch_list(&self, context: &mut PatchContext, scenes: &mut Vec<ResolvedScene>) {
        let mut resolved_scene = ResolvedScene::default();
        self.0.patch(context, &mut resolved_scene);
        scenes.push(resolved_scene);
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        self.0.register_dependencies(dependencies);
    }
}

macro_rules! scene_list_impl {
    ($($list: ident),*) => {
        impl<$($list: SceneList),*> SceneList for ($($list,)*) {
            fn patch_list(&self, _context: &mut PatchContext, _scenes: &mut Vec<ResolvedScene>) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($list,)*) = self;
                $($list.patch_list(_context, _scenes);)*
            }

            fn register_dependencies(&self, _dependencies: &mut Vec<AssetPath<'static>>) {
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
    fn patch_list(&self, context: &mut PatchContext, scenes: &mut Vec<ResolvedScene>) {
        for scene in self {
            let mut resolved_scene = ResolvedScene::default();
            scene.patch(context, &mut resolved_scene);
            scenes.push(resolved_scene);
        }
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        for scene in self {
            scene.register_dependencies(dependencies);
        }
    }
}

impl SceneList for Vec<Box<dyn Scene>> {
    fn patch_list(&self, context: &mut PatchContext, scenes: &mut Vec<ResolvedScene>) {
        for scene in self {
            let mut resolved_scene = ResolvedScene::default();
            scene.patch(context, &mut resolved_scene);
            scenes.push(resolved_scene);
        }
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        for scene in self {
            scene.register_dependencies(dependencies);
        }
    }
}
