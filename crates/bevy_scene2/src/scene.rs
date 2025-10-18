use crate::{ResolvedRelatedScenes, ResolvedScene, SceneList, ScenePatch};
use bevy_asset::{AssetPath, AssetServer, Assets};
use bevy_ecs::{
    bundle::Bundle,
    error::Result,
    name::Name,
    relationship::Relationship,
    template::{EntityScopes, FnTemplate, GetTemplate, Template, TemplateContext},
};
use std::{any::TypeId, marker::PhantomData};
use variadics_please::all_tuples;

pub trait Scene: Send + Sync + 'static {
    fn patch(&self, context: &mut PatchContext, scene: &mut ResolvedScene);
    fn register_dependencies(&self, _dependencies: &mut Vec<AssetPath<'static>>) {}
}

pub struct PatchContext<'a> {
    pub assets: &'a AssetServer,
    pub patches: &'a Assets<ScenePatch>,
    pub(crate) entity_scopes: &'a mut EntityScopes,
    pub(crate) current_scope: usize,
}

impl<'a> PatchContext<'a> {
    #[inline]
    pub fn current_scope(&self) -> usize {
        self.current_scope
    }

    pub fn new_scope(&mut self, func: impl FnOnce(&mut PatchContext)) {
        let current_scope = self.entity_scopes.add_scope();
        let mut context = PatchContext {
            assets: self.assets,
            patches: self.patches,
            entity_scopes: self.entity_scopes,
            current_scope,
        };
        (func)(&mut context);
    }
}

macro_rules! scene_impl {
    ($($patch: ident),*) => {
        impl<$($patch: Scene),*> Scene for ($($patch,)*) {
            fn patch(&self, _context: &mut PatchContext, _scene: &mut ResolvedScene) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($patch,)*) = self;
                $($patch.patch(_context, _scene);)*
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

pub struct TemplatePatch<F: Fn(&mut T, &mut PatchContext), T>(pub F, pub PhantomData<T>);

pub fn template_value<T: Template + Default + Clone>(
    value: T,
) -> TemplatePatch<impl Fn(&mut T, &mut PatchContext), T> {
    TemplatePatch(
        move |input: &mut T, _context: &mut PatchContext| {
            *input = value.clone();
        },
        PhantomData,
    )
}

pub trait PatchGetTemplate {
    type Template;
    fn patch<F: Fn(&mut Self::Template, &mut PatchContext)>(
        func: F,
    ) -> TemplatePatch<F, Self::Template>;
}

impl<G: GetTemplate> PatchGetTemplate for G {
    type Template = G::Template;
    fn patch<F: Fn(&mut Self::Template, &mut PatchContext)>(
        func: F,
    ) -> TemplatePatch<F, Self::Template> {
        TemplatePatch(func, PhantomData)
    }
}

pub trait PatchTemplate: Sized {
    fn patch_template<F: Fn(&mut Self, &mut PatchContext)>(func: F) -> TemplatePatch<F, Self>;
}

impl<T: Template> PatchTemplate for T {
    fn patch_template<F: Fn(&mut Self, &mut PatchContext)>(func: F) -> TemplatePatch<F, Self> {
        TemplatePatch(func, PhantomData)
    }
}

impl<
        F: Fn(&mut T, &mut PatchContext) + Send + Sync + 'static,
        T: Template<Output: Bundle> + Send + Sync + Default + 'static,
    > Scene for TemplatePatch<F, T>
{
    fn patch(&self, context: &mut PatchContext, scene: &mut ResolvedScene) {
        let template = scene.get_or_insert_template::<T>();
        (self.0)(template, context);
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
    fn patch(&self, context: &mut PatchContext, scene: &mut ResolvedScene) {
        let related = scene
            .related
            .entry(TypeId::of::<R>())
            .or_insert_with(ResolvedRelatedScenes::new::<R>);
        self.related_template_list
            .patch_list(context, &mut related.scenes);
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        self.related_template_list
            .register_dependencies(dependencies);
    }
}

pub struct InheritScene<S: Scene>(pub S);

impl<S: Scene> Scene for InheritScene<S> {
    fn patch(&self, context: &mut PatchContext, scene: &mut ResolvedScene) {
        context.new_scope(|context| {
            self.0.patch(context, scene);
        });
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
    fn patch(&self, context: &mut PatchContext, scene: &mut ResolvedScene) {
        let id = context.assets.get_path_id(&self.0).unwrap();
        let scene_patch = context.patches.get(id.typed()).unwrap();
        context.new_scope(|context| {
            scene_patch.patch.patch(context, scene);
        });
    }

    fn register_dependencies(&self, dependencies: &mut Vec<AssetPath<'static>>) {
        dependencies.push(self.0.clone())
    }
}

impl<F: (FnMut(&mut TemplateContext) -> Result<O>) + Clone + Send + Sync + 'static, O: Bundle> Scene
    for FnTemplate<F, O>
{
    fn patch(&self, _context: &mut PatchContext, scene: &mut ResolvedScene) {
        scene.push_template(FnTemplate(self.0.clone()));
    }
}

pub struct NameEntityReference {
    pub name: Name,
    pub index: usize,
}

impl Scene for NameEntityReference {
    fn patch(&self, context: &mut PatchContext, scene: &mut ResolvedScene) {
        if let Some((scope, index)) = scene.entity_references.first().copied() {
            let entity_index = context.entity_scopes.get(scope, index).unwrap();
            context
                .entity_scopes
                .assign(context.current_scope, self.index, entity_index);
        } else {
            context
                .entity_scopes
                .alloc(context.current_scope, self.index);
        }
        scene
            .entity_references
            .push((context.current_scope, self.index));
        let name = scene.get_or_insert_template::<Name>();
        *name = self.name.clone();
    }
}
