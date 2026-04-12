use crate::{InheritSceneError, ResolvedScene, SceneList, ScenePatch};
use bevy_asset::{Asset, AssetPath, AssetServer, Assets};
use bevy_ecs::{
    bundle::Bundle,
    error::Result,
    event::EntityEvent,
    name::Name,
    relationship::Relationship,
    system::IntoObserverSystem,
    template::{
        EntityScopes, FnTemplate, FromTemplate, ScopedEntityIndex, Template, TemplateContext,
    },
};
use core::{any::TypeId, marker::PhantomData};
use thiserror::Error;
use variadics_please::all_tuples;

/// Conceptually, a [`Scene`] describes what a spawned [`Entity`] should look like. This often describes what [`Component`]s the entity should have.
///
/// [`Scene`] is _always_ a single top level [`Entity`] / root entity.  For "lists of scenes" / multiple "root" entities, see [`SceneList`]. These are
/// separate traits for logical reasons: [`World::spawn`] is a "single entity" action. Additionally, "scene inheritance" only makes sense when both scenes
/// are "single root entities". A good way to think of this is [`Entity`] vs [`Vec<Entity>`]: these are different types with different APIs and semantics.
///
/// ## Resolving Scenes
///
/// Functionally, a [`Scene`] is something that can contribute to a [`ResolvedScene`] by calling [`Scene::resolve`]. [`Scene`] is inherently composable.
/// A collection of [`Scene`]s is essentially a description of what a final [`ResolvedScene`] should look like. This is typically done with
/// tuples of [`Scene`]s (which also implement [`Scene`]).
///
/// A [`Scene`] generally does one or more of the following to a [`ResolvedScene`]:
/// - Adding a new [`Template`]
/// - Editing an existing [`Template`] (ex: "patching" [`Template`] fields)
/// - Adding one or more "related" [`ResolvedScene`]s, which will be spawned alongside the root [`ResolvedScene`] and "related" back to it with a [`Relationship`].
/// - Editing an existing "related" [`ResolvedScene`].
/// - Setting a [`ScenePatch`] to inherit from.
///
/// See [`ResolvedScene`] for more information on how it can be composed.
///
/// A [`Scene`] can have dependencies (defined with [`Scene::register_dependencies`]), which _must_ be loaded before calling [`Scene::resolve`], or it
/// might return a [`ResolveSceneError`].
///
/// You generally don't need to resolve [`Scene`]s yourself. Instead use APIs like [`World::spawn_scene`] or [`World::queue_spawn_scene`]
///
/// [`World::spawn`]: crate::World::spawn
/// [`World::spawn_scene`]: crate::WorldSceneExt::spawn_scene
/// [`World::queue_spawn_scene`]: crate::WorldSceneExt::queue_spawn_scene
/// [`Entity`]: bevy_ecs::entity::Entity
/// [`Component`]: bevy_ecs::component::Component
pub trait Scene: Send + Sync + 'static {
    /// This will apply the changes described in this [`Scene`] to the given [`ResolvedScene`]. This should not be called until all of the dependencies
    /// in [`Scene::register_dependencies`] have been loaded. The scene system will generally call this method on behalf of developers.
    ///
    /// [`Scene`]s are free to modify [`ResolvedScene`] in arbitrary ways. In the context of related entities, in general they should just be pushing new
    /// entities to the back of the list.
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError>;

    /// [`Scene`] can have [`Asset`] dependencies, which _must_ be loaded before calling [`Scene::resolve`] or it might return a [`ResolveSceneError`]!
    ///
    /// In most cases, the scene system will ensure [`Scene::resolve`] is called _after_ these dependencies have been loaded.
    ///
    /// [`Asset`]: bevy_asset::Asset
    fn register_dependencies(&self, _dependencies: &mut SceneDependencies) {}
}

/// A collection of asset dependencies required by a [`Scene`].
#[derive(Default)]
pub struct SceneDependencies(Vec<SceneDependency>);

impl SceneDependencies {
    /// Registers a new asset dependency with the given `type_id` and `path`. The `type_id` should match
    /// the type of the asset being loaded.
    pub fn register_erased(&mut self, type_id: TypeId, path: AssetPath<'static>) {
        self.0.push(SceneDependency { path, type_id });
    }

    /// Registers a new asset dependency with the given `A` type and `path`. `A` should match
    /// the type of the asset being loaded.
    pub fn register<A: Asset>(&mut self, path: AssetPath<'static>) {
        self.register_erased(TypeId::of::<A>(), path);
    }

    /// Iterates the current dependencies.
    pub fn iter(&self) -> impl Iterator<Item = &SceneDependency> {
        self.0.iter()
    }
}

/// An asset dependency of a [`Scene`].
pub struct SceneDependency {
    /// The path of the asset.
    pub path: AssetPath<'static>,
    /// The type of the asset.
    pub type_id: TypeId,
}

/// An [`Error`] that occurs during [`Scene::resolve`].
#[derive(Error, Debug)]
pub enum ResolveSceneError {
    /// Caused when a dependency listed in [`Scene::register_dependencies`] is not available when calling [`Scene::resolve`]
    #[error("Cannot resolve scene because the asset dependency {0} is not present. This could be because it isn't loaded yet, or because the asset does not exist. Consider using `queue_spawn_scene()` if you would like to wait for scene dependencies before spawning.")]
    MissingSceneDependency(AssetPath<'static>),
    /// Caused when inheriting a scene during [`Scene::resolve`] fails.
    #[error(transparent)]
    InheritSceneError(#[from] InheritSceneError),
}

/// Context used by [`Scene`] implementations during [`Scene::resolve`].
pub struct ResolveContext<'a> {
    /// The current asset server
    pub assets: &'a AssetServer,
    /// The current [`ScenePatch`] asset collection
    pub patches: &'a Assets<ScenePatch>,
    /// The currently inherited [`ScenePatch`], if there is one.
    pub inherited: Option<&'a ScenePatch>,
    pub(crate) entity_scopes: &'a mut EntityScopes,
    pub(crate) current_scope: usize,
}

impl<'a> ResolveContext<'a> {
    /// The current entity scope.
    #[inline]
    pub fn current_entity_scope(&self) -> usize {
        self.current_scope
    }

    /// Creates a new entity scope, which is active for the duration of `func`. When this function returns,
    /// the original scope will be returned to.
    pub fn new_entity_scope<T>(&mut self, func: impl FnOnce(&mut ResolveContext) -> T) -> T {
        let current_scope = self.entity_scopes.add_scope();
        let mut context = ResolveContext {
            assets: self.assets,
            patches: self.patches,
            inherited: None,
            entity_scopes: self.entity_scopes,
            current_scope,
        };
        (func)(&mut context)
    }
}

macro_rules! scene_impl {
    ($($patch: ident),*) => {
        impl<$($patch: Scene),*> Scene for ($($patch,)*) {
            fn resolve(&self, _context: &mut ResolveContext, _scene: &mut ResolvedScene) -> Result<(), ResolveSceneError> {
                #[expect(
                    clippy::allow_attributes,
                    reason = "This is inside a macro, and as such, may not trigger in all cases."
                )]
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($patch,)*) = self;
                $($patch.resolve(_context, _scene)?;)*
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
                let ($($patch,)*) = self;
                $($patch.register_dependencies(_dependencies);)*
            }
       }
    }
}

all_tuples!(scene_impl, 0, 12, P);

impl Scene for Box<dyn Scene> {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        (**self).resolve(context, scene)
    }
    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        (**self).register_dependencies(dependencies);
    }
}

impl SceneList for Box<dyn SceneList> {
    fn resolve_list(
        &self,
        context: &mut ResolveContext,
        scenes: &mut Vec<ResolvedScene>,
    ) -> Result<(), ResolveSceneError> {
        (**self).resolve_list(context, scenes)
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        (**self).register_dependencies(dependencies);
    }
}

/// A [`Scene`] that patches a [`Template`] of type `T` with a given function `F`.
///
/// Functionally, a [`TemplatePatch`] scene will initialize a [`Default`] value of the patched
/// template if it does not already exist in the [`ResolvedScene`], then it will apply the patch on top
/// of the current [`Template`] in the [`ResolvedScene`].
///
/// This is usually created by the [`PatchTemplate`] or [`PatchFromTemplate`] traits.
///
/// This enables defining things like "field" patches, which set specific fields without overriding
/// any other fields:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_scene::PatchFromTemplate;
/// #[derive(FromTemplate)]
/// struct Position {
///     x: usize,
///     y: usize,
/// }
///
/// let patch = Position::patch(|position_template, context| {
///     position_template.x = 10;
/// });
///
/// let position = Position { x: 0, y: 0};
/// // applying patch to position would result in { x: 10, y: 0 }
/// ```
pub struct TemplatePatch<F: Fn(&mut T, &mut ResolveContext), T>(pub F, pub PhantomData<T>);

/// Returns a [`Scene`] that completely overwrites the current value of a [`Template`] `T` with the given `value`.
/// The `value` is cloned each time the [`Template`] is built.
pub fn template_value<T: Template + Clone>(
    value: T,
) -> TemplatePatch<impl Fn(&mut T, &mut ResolveContext), T> {
    TemplatePatch(
        move |input: &mut T, _context: &mut ResolveContext| {
            *input = value.clone();
        },
        PhantomData,
    )
}

/// A helper function that returns a [`TemplatePatch`] [`Scene`] for something that implements [`FromTemplate`].
/// It will use [`FromTemplate::Template`] as the "patched template".
pub trait PatchFromTemplate {
    /// The [`Template`] that will be patched.
    type Template;

    /// Takes a "patch function" `func`, and turns it into a [`TemplatePatch`].
    fn patch<F: Fn(&mut Self::Template, &mut ResolveContext)>(
        func: F,
    ) -> TemplatePatch<F, Self::Template>;
}

impl<G: FromTemplate> PatchFromTemplate for G {
    type Template = G::Template;
    fn patch<F: Fn(&mut Self::Template, &mut ResolveContext)>(
        func: F,
    ) -> TemplatePatch<F, Self::Template> {
        TemplatePatch(func, PhantomData)
    }
}

/// A helper function that returns a [`TemplatePatch`] [`Scene`] for something that implements [`Template`].
pub trait PatchTemplate: Sized {
    /// Takes a "patch function" `func` that patches this [`Template`], and turns it into a [`TemplatePatch`].
    fn patch_template<F: Fn(&mut Self, &mut ResolveContext)>(func: F) -> TemplatePatch<F, Self>;
}

impl<T: Template> PatchTemplate for T {
    fn patch_template<F: Fn(&mut Self, &mut ResolveContext)>(func: F) -> TemplatePatch<F, Self> {
        TemplatePatch(func, PhantomData)
    }
}

impl<
        F: Fn(&mut T, &mut ResolveContext) + Send + Sync + 'static,
        T: Template<Output: Bundle> + Send + Sync + Default + 'static,
    > Scene for TemplatePatch<F, T>
{
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        let template = scene.get_or_insert_template::<T>(context);
        (self.0)(template, context);
        Ok(())
    }
}

/// A [`Scene`] that adds an `L` [`SceneList`] as "related scenes", using the `R` [`Relationship`]
pub struct RelatedScenes<R: Relationship, L: SceneList> {
    /// The related [`SceneList`]. Each entity described in the list will be spawned with the given [`Relationship`] to the
    /// entity described in the current [`Scene`].
    pub related_template_list: L,

    /// Marker holding the `R` type.
    pub marker: PhantomData<R>,
}

impl<R: Relationship, L: SceneList> RelatedScenes<R, L> {
    /// Creates a new [`RelatedScenes`] with the given `list`.
    pub fn new(list: L) -> Self {
        Self {
            related_template_list: list,
            marker: PhantomData,
        }
    }
}

impl<R: Relationship, L: SceneList> Scene for RelatedScenes<R, L> {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        let related = scene.get_or_insert_related_resolved_scenes::<R>();
        self.related_template_list
            .resolve_list(context, &mut related.scenes)
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        self.related_template_list
            .register_dependencies(dependencies);
    }
}

/// A [`Scene`] that will inherit from the [`ScenePatch`] stored at the given [`AssetPath`].
/// This will _not_ resolve the inherited scene directly on top of this [`ResolvedScene`]. Instead
/// it will set [`ResolvedScene::inherit`], which (when spawning the [`ResolvedScene`]) will apply the inherited [`ResolvedScene`]
/// first. _Then_ the top-level [`ResolvedScene`] will be applied.
///
/// This also enables copy-on-write semantics for all future [`Template`] accesses. See [`ResolvedScene`] for more info on "inheritance".
#[derive(Clone)]
pub struct InheritSceneAsset(
    /// The [`AssetPath`] of the [`ScenePatch`] to inherit from.
    pub AssetPath<'static>,
);

impl<I: Into<AssetPath<'static>>> From<I> for InheritSceneAsset {
    fn from(value: I) -> Self {
        InheritSceneAsset(value.into())
    }
}

impl Scene for InheritSceneAsset {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        if let Some(handle) = context.assets.get_handle::<ScenePatch>(&self.0)
            && let Some(scene_patch) = context.patches.get(&handle)
        {
            scene.inherit(handle)?;
            context.inherited = Some(scene_patch);
            Ok(())
        } else {
            Err(ResolveSceneError::MissingSceneDependency(self.0.clone()))
        }
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        dependencies.register::<ScenePatch>(self.0.clone());
    }
}

impl<F: (Fn(&mut TemplateContext) -> Result<O>) + Clone + Send + Sync + 'static, O: Bundle> Scene
    for FnTemplate<F, O>
{
    fn resolve(
        &self,
        _context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        scene.push_template(FnTemplate(self.0.clone()));
        Ok(())
    }
}

/// Sets up a given name as an "entity reference" for the current entity. This pairs the [`Self::name`] field
/// to a given [`Self::index`] field.
///
/// The `index` should be a dense, unique identifier (within the current "entity scope") that can be used to reference this entity.
/// Usually this is not set manually by a user. Instead this is generally done by a macro (such as the [`bsn!`] macro) or an asset loader
/// (such as the BSN asset loader).
///
/// [`bsn!`]: crate::bsn
pub struct NameEntityReference {
    /// The name to give this entity.
    pub name: Name,
    /// The index (within the current "entity scope") of this entity reference.
    pub index: usize,
}

impl Scene for NameEntityReference {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        let this_index = ScopedEntityIndex {
            scope: context.current_entity_scope(),
            index: self.index,
        };
        if let Some(first_index) = scene.entity_indices.first().copied() {
            let consolidated_index = context.entity_scopes.get(first_index).unwrap();
            context.entity_scopes.assign(this_index, consolidated_index);
        } else {
            context.entity_scopes.alloc(this_index);
        }
        scene.entity_indices.push(this_index);
        let name = scene.get_or_insert_template::<Name>(context);
        *name = self.name.clone();
        Ok(())
    }
}

/// A [`Scene`] that will create a new "entity scope" and fully resolve the given scene `S` on top of the current [`ResolvedScene`] (using that scope).
/// It is not "inherited" or cached.
pub struct SceneScope<S: Scene>(pub S);

impl<S: Scene> Scene for SceneScope<S> {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        context.new_entity_scope(|context| self.0.resolve(context, scene))
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        self.0.register_dependencies(dependencies);
    }
}

/// A [`SceneList`] that will create a new "entity scope" and fully resolve the given scene list `L` on top of the current [`Vec<ResolvedScene>`]
/// (using that scope). It is not "inherited" or cached.
pub struct SceneListScope<L: SceneList>(pub L);

impl<L: SceneList> SceneList for SceneListScope<L> {
    fn resolve_list(
        &self,
        context: &mut ResolveContext,
        scenes: &mut Vec<ResolvedScene>,
    ) -> Result<(), ResolveSceneError> {
        context.new_entity_scope(|context| self.0.resolve_list(context, scenes))
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        self.0.register_dependencies(dependencies);
    }
}

/// A [`Template`] / [`Scene`] that will create an [`Observer`] of a given [`EntityEvent`] on the current [`Scene`] entity.
/// This is typically initialized using the [`on()`] function, which returns an [`OnTemplate`].
///
/// [`Observer`]: bevy_ecs::observer::Observer
pub struct OnTemplate<I, E, B, M>(pub I, pub PhantomData<fn() -> (E, B, M)>);

impl<I: IntoObserverSystem<E, B, M> + Clone, E: EntityEvent, B: Bundle, M: 'static> Template
    for OnTemplate<I, E, B, M>
{
    type Output = ();

    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        context.entity.observe(self.0.clone());
        Ok(())
    }

    fn clone_template(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<
        I: IntoObserverSystem<E, B, M> + Clone + Send + Sync,
        E: EntityEvent,
        B: Bundle,
        M: 'static,
    > Scene for OnTemplate<I, E, B, M>
{
    fn resolve(
        &self,
        _context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        scene.push_template(OnTemplate(self.0.clone(), PhantomData));
        Ok(())
    }
}

/// Returns an [`OnTemplate`] that will create an [`Observer`] of a given [`EntityEvent`] on the current [`Scene`] entity.
///
/// [`Observer`]: bevy_ecs::observer::Observer
pub fn on<I: IntoObserverSystem<E, B, M>, E: EntityEvent, B: Bundle, M: 'static>(
    observer: I,
) -> OnTemplate<I, E, B, M> {
    OnTemplate(observer, PhantomData)
}
