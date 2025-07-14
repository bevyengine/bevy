use crate::{ResolvedRelatedScenes, ResolvedScene, SceneList, ScenePatch};
use bevy_asset::{AssetPath, AssetServer, Assets};
use bevy_ecs::{
    bundle::Bundle,
    error::Result,
    relationship::Relationship,
    template::{FnTemplate, GetTemplate, Template},
    world::EntityWorldMut,
};
use std::{any::TypeId, marker::PhantomData};
use variadics_please::all_tuples;

pub trait Scene: Send + Sync + 'static {
    fn patch(&self, assets: &AssetServer, patches: &Assets<ScenePatch>, scene: &mut ResolvedScene);
    fn register_dependencies(&self, _dependencies: &mut Vec<AssetPath<'static>>) {}
}

macro_rules! scene_impl {
    ($($patch: ident),*) => {
        impl<$($patch: Scene),*> Scene for ($($patch,)*) {
            fn patch(&self, _assets: &AssetServer, _patches: &Assets<ScenePatch>, _scene: &mut ResolvedScene) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($patch,)*) = self;
                $($patch.patch(_assets, _patches, _scene);)*
            }

            fn register_dependencies(&self, _dependencies: &mut Vec<AssetPath<'static>>) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($patch,)*) = self;
                $($patch.register_dependencies(_dependencies);)*
            }
       }
    }
}

all_tuples!(scene_impl, 0, 12, P);

pub struct TemplatePatch<F: Fn(&mut T), T>(pub F, pub PhantomData<T>);

pub fn template_value<T: Template + Default + Clone>(
    value: T,
) -> TemplatePatch<impl Fn(&mut T), T> {
    TemplatePatch(
        move |input: &mut T| {
            *input = value.clone();
        },
        PhantomData,
    )
}

pub trait PatchGetTemplate {
    type Template;
    fn patch<F: Fn(&mut Self::Template)>(func: F) -> TemplatePatch<F, Self::Template>;
}

impl<G: GetTemplate> PatchGetTemplate for G {
    type Template = G::Template;
    fn patch<F: Fn(&mut Self::Template)>(func: F) -> TemplatePatch<F, Self::Template> {
        TemplatePatch(func, PhantomData)
    }
}

pub trait PatchTemplate: Sized {
    fn patch_template<F: Fn(&mut Self)>(func: F) -> TemplatePatch<F, Self>;
}

impl<T: Template> PatchTemplate for T {
    fn patch_template<F: Fn(&mut Self)>(func: F) -> TemplatePatch<F, Self> {
        TemplatePatch(func, PhantomData)
    }
}

impl<
        F: Fn(&mut T) + Send + Sync + 'static,
        T: Template<Output: Bundle> + Send + Sync + Default + 'static,
    > Scene for TemplatePatch<F, T>
{
    fn patch(
        &self,
        _assets: &AssetServer,
        _patches: &Assets<ScenePatch>,
        scene: &mut ResolvedScene,
    ) {
        let template = scene.get_or_insert_template::<T>();
        (self.0)(template);
    }
}

pub struct RelatedScenes<R: Relationship, L: SceneList> {
    pub related_template_list: L,
    pub marker: PhantomData<R>,
}

impl<R: Relationship, L: SceneList> RelatedScenes<R, L> {
    pub fn new(list: L) -> Self {
        Self {
            related_template_list: list,
            marker: PhantomData,
        }
    }
}

impl<R: Relationship, L: SceneList> Scene for RelatedScenes<R, L> {
    fn patch(&self, assets: &AssetServer, patches: &Assets<ScenePatch>, scene: &mut ResolvedScene) {
        let related = scene
            .related
            .entry(TypeId::of::<R>())
            .or_insert_with(ResolvedRelatedScenes::new::<R>);
        self.related_template_list
            .patch_list(assets, patches, &mut related.scenes);
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        self.related_template_list
            .register_dependencies(dependencies);
    }
}

pub struct InheritScene<S: Scene>(pub S);

impl<S: Scene> Scene for InheritScene<S> {
    fn patch(&self, assets: &AssetServer, patches: &Assets<ScenePatch>, scene: &mut ResolvedScene) {
        self.0.patch(assets, patches, scene);
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        self.0.register_dependencies(dependencies);
    }
}

#[derive(Clone)]
pub struct InheritSceneAsset(pub AssetPath<'static>);

impl<I: Into<AssetPath<'static>>> From<I> for InheritSceneAsset {
    fn from(value: I) -> Self {
        InheritSceneAsset(value.into())
    }
}

impl Scene for InheritSceneAsset {
    fn patch(&self, assets: &AssetServer, patches: &Assets<ScenePatch>, scene: &mut ResolvedScene) {
        let id = assets.get_path_id(&self.0).unwrap();
        let scene_patch = patches.get(id.typed()).unwrap();
        scene_patch.patch.patch(assets, patches, scene);
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        dependencies.push(self.0.clone())
    }
}

impl<F: (FnMut(&mut EntityWorldMut) -> Result<O>) + Clone + Send + Sync + 'static, O: Bundle> Scene
    for FnTemplate<F, O>
{
    fn patch(
        &self,
        _assets: &AssetServer,
        _patches: &Assets<ScenePatch>,
        scene: &mut ResolvedScene,
    ) {
        scene.push_template(FnTemplate(self.0.clone()));
    }
}
