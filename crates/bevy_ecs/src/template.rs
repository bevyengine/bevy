//! Functionality that relates to the [`Template`] trait.

use alloc::{boxed::Box, sync::Arc};
use bevy_ecs_macros::Component;
use bevy_platform::sync::Mutex;

pub use bevy_ecs_macros::FromTemplate;

use crate::{
    component::Mutable,
    entity::Entity,
    error::{BevyError, Result},
    lifecycle::HookContext,
    resource::Resource,
    system::{BoxedSystem, IntoSystem, SystemId},
    world::{DeferredWorld, EntityWorldMut, Mut, World},
};
use alloc::{vec, vec::Vec};
use variadics_please::all_tuples;

/// A [`Template`] is something that, given a spawn context (target [`Entity`], [`World`], etc), can produce a [`Template::Output`].
///
/// [`Template`] is the cornerstone of scene systems. It enables define types (and hierarchies) that require no [`World`] or [`Entity`] context to define,
/// but can _use_ that context to produce the final runtime state. A [`Template`] is notably:
/// * **Repeatable**: Building a [`Template`] does not consume it. This enables reusing "baked" scenes / avoids rebuilding scenes each time we want to spawn one.
/// * **Clone-able**: Templates can be duplicated via [`Template::clone_template`], enabling scenes to be duplicated, supporting copy-on-write behaviors, etc.
/// * **(Often) Serializable**: Templates are intended to be easily serialized and deserialized, as they are typically composed of raw data.
///
/// Asset handles and [`Entity`] are two commonly [`Template`]-ed types. Asset handles are often "loaded" from an "asset path". The "asset path" would be the [`Template`].
/// Likewise [`Entity`] on its own has no reasonable default. A type with an [`Entity`] reference could use an "entity path" template to point to a specific entity, relative
/// to the current spawn context.
///
/// See [`FromTemplate`], which defines the canonical [`Template`] for a type. This can be derived, which will generate a [`Template`] for the deriving type.
pub trait Template {
    /// The type of value produced by this [`Template`].
    type Output;

    /// Uses this template and the given `entity` context to produce a [`Template::Output`].
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output>;

    /// Clones this template. See [`Clone`].
    fn clone_template(&self) -> Self;
}

/// The context used to apply the current [`Template`]. This contains a reference to the entity that the template is being
/// applied to (via an [`EntityWorldMut`]).
pub struct TemplateContext<'a, 'w> {
    /// The current entity the template is being applied to
    pub entity: &'a mut EntityWorldMut<'w>,
    /// The scoped entities mapping for the current template context
    pub scoped_entities: &'a mut ScopedEntities,
    /// The entity scopes for the current template context. This matches
    /// the `scoped_entities`.
    pub entity_scopes: &'a EntityScopes,
}

impl<'a, 'w> TemplateContext<'a, 'w> {
    /// Creates a new [`TemplateContext`].
    pub fn new(
        entity: &'a mut EntityWorldMut<'w>,
        scoped_entities: &'a mut ScopedEntities,
        entity_scopes: &'a EntityScopes,
    ) -> Self {
        Self {
            entity,
            scoped_entities,
            entity_scopes,
        }
    }

    /// Retrieves the scoped entity if it has already been spawned, and spawns a new entity if it has not
    /// yet been spawned.
    pub fn get_scoped_entity(&mut self, scoped_entity_index: ScopedEntityIndex) -> Entity {
        self.scoped_entities.get(
            // SAFETY: this only uses the world to spawn an empty entity
            unsafe { self.entity.world_mut() },
            self.entity_scopes,
            scoped_entity_index,
        )
    }

    /// Retrieves a reference to the given resource `R`.
    #[inline]
    pub fn resource<R: Resource>(&self) -> &R {
        self.entity.resource()
    }

    /// Retrieves a mutable reference to the given resource `R`.
    #[inline]
    pub fn resource_mut<R: Resource<Mutability = Mutable>>(&mut self) -> Mut<'_, R> {
        self.entity.resource_mut()
    }
}

/// A mapping from from an entity reference's (scope, index) to a contiguous flat index that uniquely
/// identifies the entity within a scene.
#[derive(Default, Debug)]
pub struct EntityScopes {
    scopes: Vec<Vec<Option<usize>>>,
    next_index: usize,
}

impl EntityScopes {
    /// The number of entities defined across all scopes.
    #[inline]
    pub fn entity_len(&self) -> usize {
        self.next_index
    }

    /// Allocate a new contiguous entity index for the given (scope, index) pair.
    pub fn alloc(&mut self, scoped_entity_index: ScopedEntityIndex) {
        *self.get_mut(scoped_entity_index) = Some(self.next_index);
        self.next_index += 1;
    }

    /// Assign an existing contiguous entity index for the given (scope, index) pair.
    /// This is generally used when there are multiple (scope, index) pairs that point
    /// to the same entity (ex: scene inheritance).
    pub fn assign(&mut self, scoped_entity_index: ScopedEntityIndex, value: usize) {
        let option = self.get_mut(scoped_entity_index);
        *option = Some(value);
    }

    #[expect(unsafe_code, reason = "Easily verifiable performance optimization")]
    fn get_mut(&mut self, scoped_entity_index: ScopedEntityIndex) -> &mut Option<usize> {
        // NOTE: this is ok because PatchContext::new_scope adds scopes as they are created.
        // this shouldn't panic unless internals are broken.
        let indices = &mut self.scopes[scoped_entity_index.scope];
        if scoped_entity_index.index >= indices.len() {
            indices.resize_with(scoped_entity_index.index + 1, || None);
        }
        // SAFETY: just allocated above
        unsafe { indices.get_unchecked_mut(scoped_entity_index.index) }
    }

    /// Gets the assigned contiguous entity index for the given (scope, index) pair
    pub fn get(&self, scoped_entity_index: ScopedEntityIndex) -> Option<usize> {
        *self
            .scopes
            .get(scoped_entity_index.scope)?
            .get(scoped_entity_index.index)?
    }

    /// Creates a new scope and returns it.
    pub fn add_scope(&mut self) -> usize {
        let scope_index = self.scopes.len();
        self.scopes.push(Vec::default());
        scope_index
    }
}

/// A contiguous list of entities identified by their index in the list.
#[derive(Debug)]
pub struct ScopedEntities(Vec<Option<Entity>>);

impl ScopedEntities {
    /// Creates a new [`ScopedEntities`] with the given `size`, initialized to [`None`] (no [`Entity`] assigned).
    pub fn new(size: usize) -> Self {
        Self(vec![None; size])
    }
}

impl ScopedEntities {
    /// Gets the [`Entity`] assigned to the given (scope, index) pair, if it exists, and spawns a new entity if
    /// it does not.
    pub fn get(
        &mut self,
        world: &mut World,
        entity_scopes: &EntityScopes,
        scoped_entity_index: ScopedEntityIndex,
    ) -> Entity {
        let index = entity_scopes.get(scoped_entity_index).unwrap();
        *self.0[index].get_or_insert_with(|| world.spawn_empty().id())
    }

    /// Assigns the given `entity` to the (scope, index) pair.
    pub fn set(
        &mut self,
        entity_scopes: &EntityScopes,
        scoped_entity_index: ScopedEntityIndex,
        entity: Entity,
    ) {
        let index = entity_scopes.get(scoped_entity_index).unwrap();
        self.0[index] = Some(entity);
    }
}

/// [`FromTemplate`] is implemented for types that can be produced by a specific, canonical [`Template`]. This creates a way to correlate to the [`Template`] using the
/// desired template output type. This is used by Bevy's scene system.
///
/// Both [`FromTemplate`] and [`Template`] are blanket implemented for types that implement [`Default`] and [`Clone`], meaning most types you would want to use
/// _already have templates_.
///
/// It is best to think of [`FromTemplate`] as an alternative to [`Default`] for types that require world/spawn context to instantiate. Note that because of the blanket
/// impl, you cannot implement [`FromTemplate`], [`Default`], and [`Clone`] together on the same type, as it would result in two conflicting [`FromTemplate`] impls.
/// This is also why [`Template`] has its own [`Template::clone_template`] method (to avoid using the [`Clone`] impl, which would pull in the auto-impl).
///
/// You can _and should_ prefer deriving [`Default`] and [`Clone`] instead of an explicit [`FromTemplate`] impl, unless your type uses something that requires (or uses)
/// a [`Template`]. Handles in an asset system or [`Entity`] are examples of "templated" types. If you want your type to support templates of them, you probably want
/// to derive [`FromTemplate`].
///
/// [`FromTemplate`] can be derived for types whose fields _also_ implement [`FromTemplate`]:
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Default, Clone)]
/// # struct Handle<T>(core::marker::PhantomData<T>);
/// # #[derive(Default, Clone)]
/// # struct Image;
/// #[derive(FromTemplate)]
/// struct Player {
///     image: Handle<Image>
/// }
/// ```
///
/// Deriving [`FromTemplate`] will generate a [`Template`] type for the deriving type. The example above would generate a `PlayerTemplate` like this:
/// ```
/// # use bevy_ecs::{prelude::*, template::TemplateContext};
/// # #[derive(FromTemplate)]
/// # struct Handle<T: core::marker::Unpin>(core::marker::PhantomData<T>);
/// # #[derive(Default, Clone)]
/// # struct Image;
/// struct Player {
///     image: Handle<Image>
/// }
///
/// impl FromTemplate for Player {
///     type Template = PlayerTemplate;
/// }
///
/// struct PlayerTemplate {
///     image: HandleTemplate<Image>,
/// }
///
/// impl Template for PlayerTemplate {
///     type Output = Player;
///     fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
///         Ok(Player {
///             image: self.image.build_template(context)?,
///         })
///     }
///
///     fn clone_template(&self) -> Self {
///         PlayerTemplate {
///             image: self.image.clone_template(),
///         }
///     }
/// }
/// ```
///
/// [`FromTemplate`] derives can specify custom templates to use instead of a canonical [`FromTemplate`]:
/// ```
/// # use bevy_ecs::{prelude::*, template::TemplateContext};
/// # struct Image;
/// #[derive(FromTemplate)]
/// struct Counter {
///     #[template(Always10)]
///     count: usize
/// }
///
/// #[derive(Default)]
/// struct Always10;
///
/// impl Template for Always10 {
///     type Output = usize;
///
///     fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
///         Ok(10)
///     }
///
///     fn clone_template(&self) -> Self {
///         Always10
///     }
/// }
/// ```
///
/// [`FromTemplate`] is automatically implemented for anything that is [`Default`] and [`Clone`]. "Built in" collection types like
/// [`Option`] and [`Vec`] pick up this "blanket" implementation, which is generally a good thing because it means these collection
/// types work with [`FromTemplate`] derives by default. However if the items in the collection have a custom [`FromTemplate`] impl
/// (ex: a manual implementation like `Handle<T>` for assets or an explicit [`FromTemplate`] derive), then relying on a [`Default`] /
/// [`Clone`] implementation doesn't work, as that won't run the template logic!
///
/// Therefore, cases like [`Option<Handle<T>>`] need something other than [`FromTemplate`] to determine the type. One option is to specify
/// the template manually:
///
/// ```
/// # use bevy_ecs::{prelude::*, template::{TemplateContext, OptionTemplate}};
/// # use core::marker::PhantomData;
/// # struct Handle<T>(PhantomData<T>);
/// # struct HandleTemplate<T>(PhantomData<T>);
/// # struct Image;
/// # impl<T> FromTemplate for Handle<T> {
/// #     type Template = HandleTemplate<T>;
/// # }
/// # impl<T> Template for HandleTemplate<T> {
/// #    type Output = Handle<T>;
/// #    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
/// #        unimplemented!()
/// #    }
/// #    fn clone_template(&self) -> Self {
/// #        unimplemented!()
/// #    }
/// # }
/// #[derive(FromTemplate)]
/// struct Widget {
///     #[template(OptionTemplate<HandleTemplate<Image>>)]
///     image: Option<Handle<Image>>
/// }
/// ```
///
/// However that is a bit of a mouthful! This is where [`BuiltInTemplate`] comes in. It fills the same role
/// as [`FromTemplate`], but has no blanket implementation for [`Default`] and [`Clone`], meaning we can have
/// custom implementations for types like [`Option`] and [`Vec`].
///
/// If you are deriving [`FromTemplate`] and you have a "built in" type like [`Option<Handle<T>>`] which has custom template logic,
/// annotate it with the `template(built_in)` attribute to use [`BuiltInTemplate`] instead of [`FromTemplate`]:
///
/// ```
/// # use bevy_ecs::{prelude::*, template::TemplateContext};
/// # use core::marker::PhantomData;
/// # struct Handle<T>(PhantomData<T>);
/// # struct HandleTemplate<T>(PhantomData<T>);
/// # struct Image;
/// # impl<T> FromTemplate for Handle<T> {
/// #     type Template = HandleTemplate<T>;
/// # }
/// # impl<T> Template for HandleTemplate<T> {
/// #    type Output = Handle<T>;
/// #    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
/// #        unimplemented!()
/// #    }
/// #    fn clone_template(&self) -> Self {
/// #        unimplemented!()
/// #    }
/// # }
/// #[derive(FromTemplate)]
/// struct Widget {
///     #[template(built_in)]
///     image: Option<Handle<Image>>
/// }
/// ```
pub trait FromTemplate: Sized {
    /// The [`Template`] for this type.
    type Template: Template<Output = Self>;
}

macro_rules! template_impl {
    ($($template: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        impl<$($template: Template),*> Template for TemplateTuple<($($template,)*)> {
            type Output = ($($template::Output,)*);
            fn build_template(&self, _context: &mut TemplateContext) -> Result<Self::Output> {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($template,)*) = &self.0;
                Ok(($($template.build_template(_context)?,)*))
            }

            fn clone_template(&self) -> Self {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($template,)*) = &self.0;
                TemplateTuple(($($template.clone_template(),)*))
            }
        }
    }
}

/// A wrapper over a tuple of [`Template`] implementations, which also implements [`Template`]. This exists because [`Template`] cannot
/// be directly implemented for tuples of [`Template`] implementations.
pub struct TemplateTuple<T>(pub T);

all_tuples!(template_impl, 0, 12, T);

// This includes `Unpin` to enable specialization for Templates that also implement Default, by using the
// ["auto trait specialization" trick](https://github.com/coolcatcoder/rust_techniques/issues/1)
impl<T: Clone + Default + Unpin> Template for T {
    type Output = T;

    fn build_template(&self, _context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(self.clone())
    }

    fn clone_template(&self) -> Self {
        self.clone()
    }
}

// This includes `Unpin` to enable specialization for Templates that also implement Default, by using the
// ["auto trait specialization" trick](https://github.com/coolcatcoder/rust_techniques/issues/1)
impl<T: Clone + Default + Unpin> FromTemplate for T {
    type Template = T;
}

/// This is used to help improve error messages related to [`FromTemplate`] specialization. Developers should generally just ignore
/// this trait and read the error message when they encounter it.
#[diagnostic::on_unimplemented(
    message = "This type does not manually implement FromTemplate, and it must. If you are deriving FromTemplate and you see this, it is likely because \
               a field does not have a FromTemplate impl. This can usually be fixed by using a custom template for that field. \
               Ex: for an Option<Handle<Image>> field, annotate the field with `#[template(OptionTemplate<HandleTemplate<Image>>)]`",
    note = "FromTemplate currently uses pseudo-specialization to enable FromTemplate to override Default. This error message is a consequence of t."
)]
pub trait SpecializeFromTemplate: Sized {}

/// A [`Template`] reference to an [`Entity`].
#[derive(Default)]
pub enum EntityTemplate {
    /// A reference to a specific [`Entity`]
    Entity(Entity),
    /// A reference to an entity via a [`ScopedEntityIndex`]
    ScopedEntityIndex(ScopedEntityIndex),
    /// An entity has not been specified. Building a template with this variant will result in an error.
    #[default]
    None,
}

/// An entity index within the current [`TemplateContext`], which is defined by a scope
/// and an index. This references a specific (and sometimes yet-to-be-spawned) entity defined
/// within a given scope.
///
/// In most cases this is initialized by the scene system and should not be initialized manually.
/// Scopes must be defined ahead of time on the [`TemplateContext`].
#[derive(Copy, Clone, Debug)]
pub struct ScopedEntityIndex {
    /// The scope of the entity index. This must be defined ahead of time.
    pub scope: usize,
    /// The index that uniquely identifies the entity within the current scope.
    pub index: usize,
}

impl From<Entity> for EntityTemplate {
    fn from(entity: Entity) -> Self {
        Self::Entity(entity)
    }
}

impl Template for EntityTemplate {
    type Output = Entity;

    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(match self {
            Self::Entity(entity) => *entity,
            Self::ScopedEntityIndex(scoped_entity_index) => {
                context.get_scoped_entity(*scoped_entity_index)
            }
            Self::None => {
                return Err(BevyError::error(
                    "Failed to specify an entity for this EntityTemplate",
                ))
            }
        })
    }

    fn clone_template(&self) -> Self {
        match self {
            Self::Entity(entity) => Self::Entity(*entity),
            Self::ScopedEntityIndex(scoped_entity_index) => {
                Self::ScopedEntityIndex(*scoped_entity_index)
            }
            Self::None => Self::None,
        }
    }
}

impl FromTemplate for Entity {
    type Template = EntityTemplate;
}

/// A [`Template`] that produces a [`SystemId`] for a given system. This is used to create one-shot systems from templates.
///
/// ```ignore
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::SystemId;
/// use bevy_scene::prelude::*;
///
/// #[derive(Component, FromTemplate)]
/// struct Callback(SystemId);
///
/// fn scene() -> impl Scene{
///     bsn! {
///         Callback(system_value(|| {})),    
///     }
/// }
/// ```
///
/// Note that each system will be registered only once.
/// They share the same [`SystemId`] across all templates referencing it.
/// And the system will be automatically unregistered when no more referenced.
///
/// Therefore the system has same restriction as [`World::register_system_cached`].
/// You should generally use the [`system_value`] helper function to create this, which enforces that the system is a zero-sized type.
pub struct SystemIdTemplate(Arc<Mutex<Option<SystemOrId>>>);

enum SystemOrId {
    BoxedSystem(BoxedSystem<(), ()>),
    SystemId(SystemId<(), ()>),
}

impl Default for SystemIdTemplate {
    fn default() -> Self {
        system_value(|| {})
    }
}

// These are used to track one-shot systems registered from templates, so that they can be cleaned up when the template is no longer in use and to prevent duplicate registrations.
//
// A template instance will create a linked entity for each holding one-shot system. Each system will create a entity with `TypeIdRecorder` saved in `SceneSystemRegistry`, which counting references to it via `RefToSystem`.
//
// `template_A`: `(CompA(system_1), CompB{..., callback: system_2}, ..., LinkedWith(vec![CompA1_system1_entity, CompB1_system2_entity, ...])`
//
// `template_A_clone`: `(CompA(system_1), CompB{..., callback: system_2}, ..., LinkedWith(vec![CompA2_system1_entity, CompB2_system2_entity, ...])`
//
// `system_1`: `TypeIdRecorder{...}, SystemRefs(vec![CompA1_system1_entity, CompA2_system1_entity, ...])`
//
// `system_2`: `TypeIdRecorder{...}, SystemRefs(vec![CompB1_system2_entity, CompB2_system2_entity, ...])`
//
// When `template_A` and `template_A_clone` are dropped, the linked entities will be despawned, which triggers the cleanup system to check the system entities. Since both `system_1` and `system_2` have no more references, they will be despawned and the system is unregistered.

/// A registry that tracks one-shot systems by their [`core::any::TypeId`], since we don't want it registered multiple times for same system.
#[derive(Resource, Default)]
struct SceneSystemRegistry {
    type_map: bevy_platform::collections::HashMap<core::any::TypeId, Entity>,
}

#[derive(Component, Clone)]
struct TypeIdRecorder {
    type_id: core::any::TypeId,
}

/// These simulate a many-to-many between templates and systems in companion with [`LinkLifetimeWith`].
/// A template refers to its systems via [`LinkedWith`] and a system refers to its templates via [`SystemRefs`].
#[derive(Component)]
#[relationship(relationship_target = SystemRefs)]
#[component(on_remove = on_ref_to_system_remove)]
struct RefToSystem(Entity);

fn on_ref_to_system_remove(mut world: DeferredWorld, ctx: HookContext) {
    let system_refs_entity = world.get::<RefToSystem>(ctx.entity).unwrap().0;
    let system_refs = &world.get::<SystemRefs>(system_refs_entity).unwrap().0;

    // This hook is called in `on_remove` while relationship is maintained in `on_discard`,
    // which is ensured to be run before `on_remove` when removing a `Component`.
    // The collection hereby should be empty if a system is no more referenced.
    // And we can clean it up.
    if system_refs.is_empty() {
        let system_recorder = world
            .get::<TypeIdRecorder>(system_refs_entity)
            .unwrap()
            .clone();

        let mut registry = world.resource_mut::<SceneSystemRegistry>();
        registry.type_map.remove(&system_recorder.type_id);

        let mut commands = world.commands();

        // reused the entity spawned by `register_boxed_system`, so we don't need to despawn it again.
        commands.unregister_system::<(), ()>(SystemId::from_entity(system_refs_entity));
    }
}
#[derive(Component)]
#[relationship_target(relationship = RefToSystem)]
struct SystemRefs(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = LinkedWith)]
struct LinkLifetimeWith(Entity);

/// When a template entity is despawned, all its relationship with systems will be removed.
/// Hence the system entities will be notified via `on_ref_to_system_remove` hook to check if they are still referenced by any templates, and clean up if not.
#[derive(Component)]
#[relationship_target(relationship = LinkLifetimeWith, linked_spawn)]
struct LinkedWith(Vec<Entity>);

impl Template for SystemIdTemplate {
    type Output = SystemId<(), ()>;

    fn build_template(&self, _context: &mut TemplateContext) -> Result<Self::Output> {
        // ensure the registry exists.
        _context.entity.world_scope(|world| {
            world.init_resource::<SceneSystemRegistry>();
        });

        let mut template_state = self.0.lock().unwrap();
        let template_value = template_state.take().unwrap();

        let system_id = match template_value {
            SystemOrId::BoxedSystem(system) => _context.entity.world_scope(|world| {
                let registry = world.resource::<SceneSystemRegistry>();

                let type_id = system.system_type();

                if let Some(&entity) = registry.type_map.get(&type_id) {
                    // this system has already been registered, so just get a `SystemId` for it.
                    SystemId::from_entity(entity)
                } else {
                    // otherwise register the system and do associated preparation.
                    let system_id = world.register_boxed_system(system);
                    let system_entity = system_id.entity;

                    // link `TypeId` to `SystemId` for future reference.
                    world
                        .entity_mut(system_entity)
                        .insert(TypeIdRecorder { type_id });

                    let mut registry = world.resource_mut::<SceneSystemRegistry>();

                    registry.type_map.insert(type_id, system_entity);

                    system_id
                }
            }),
            SystemOrId::SystemId(system_id) => system_id,
        };

        // replace the system with its `SystemId`, so that next time we can skip registration and just clone the `SystemId`.
        *template_state = Some(SystemOrId::SystemId(system_id));

        // create the relationship between template and system entities.
        let template_item_entity = _context.entity.id();
        let system_entity = system_id.entity;

        // a middle entity is needed since we don't have many-to-many relationship yet.
        _context.entity.world_scope(|world| {
            world.spawn((
                RefToSystem(system_entity),
                LinkLifetimeWith(template_item_entity),
            ));
        });

        Ok(system_id)
    }

    fn clone_template(&self) -> Self {
        SystemIdTemplate(self.0.clone())
    }
}

impl FromTemplate for SystemId<(), ()> {
    type Template = SystemIdTemplate;
}

/// A helper function to create a [`SystemIdTemplate`] from a given system. This has the same restriction with [`World::register_system_cached`],
/// as the system of same type will be registered only once.
pub fn system_value<M, S: IntoSystem<(), (), M>>(system: S) -> SystemIdTemplate {
    const {
        assert!(
            size_of::<S>() == 0,
            "Non-ZST systems (e.g. capturing closures, function pointers) cannot be cached.",
        );
    }

    SystemIdTemplate(Arc::new(Mutex::new(Some(SystemOrId::BoxedSystem(
        Box::new(IntoSystem::into_system(system)),
    )))))
}

/// A [`Template`] driven by a function that returns an output. This is used to create "free floating" templates without
/// defining a new type. See [`template`] for usage.
pub struct FnTemplate<F: Fn(&mut TemplateContext) -> Result<O>, O>(pub F);

impl<F: Fn(&mut TemplateContext) -> Result<O> + Clone, O> Template for FnTemplate<F, O> {
    type Output = O;

    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        (self.0)(context)
    }

    fn clone_template(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Returns a "free floating" template for a given `func`. This prevents the need to define a custom type for one-off templates.
pub fn template<F: Fn(&mut TemplateContext) -> Result<O>, O>(func: F) -> FnTemplate<F, O> {
    FnTemplate(func)
}

/// Roughly equivalent to [`FromTemplate`], but does not have a blanket implementation for [`Default`] + [`Clone`] types.
/// This is generally used for common generic collection types like [`Option`] and [`Vec`], which have [`Default`] + [`Clone`] impls and
/// therefore also pick up the [`FromTemplate`] behavior. This is fine when the `T` in [`Option<T>`] is not "templated"
/// (ex: does not have an explicit [`FromTemplate`] derive). But if `T` is "templated", such as [`Option<Handle<T>>`], then it would require
/// a manual `#[template(OptionTemplate<HandleTemplate<T>>)]` field annotation. This isn't fun to type out.
///
/// [`BuiltInTemplate`] enables equivalent "template type inference", by annotating a field with a type that implements [`BuiltInTemplate`] with
/// `#[template(built_in)]`.
pub trait BuiltInTemplate: Sized {
    /// The template to consider the "built in" template for this type.
    type Template: Template;
}

impl<T: FromTemplate> BuiltInTemplate for Option<T> {
    type Template = OptionTemplate<T::Template>;
}

impl<T: FromTemplate> BuiltInTemplate for Vec<T> {
    type Template = VecTemplate<T::Template>;
}

/// A [`Template`] for [`Option`].
#[derive(Default)]
pub enum OptionTemplate<T> {
    /// Template of [`Option::Some`].
    Some(T),
    /// Template of [`Option::None`].
    #[default]
    None,
}

impl<T> From<Option<T>> for OptionTemplate<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => OptionTemplate::Some(value),
            None => OptionTemplate::None,
        }
    }
}

impl<T> From<T> for OptionTemplate<T> {
    fn from(value: T) -> Self {
        OptionTemplate::Some(value)
    }
}

impl<T: Template> Template for OptionTemplate<T> {
    type Output = Option<T::Output>;

    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(match &self {
            OptionTemplate::Some(template) => Some(template.build_template(context)?),
            OptionTemplate::None => None,
        })
    }

    fn clone_template(&self) -> Self {
        match self {
            OptionTemplate::Some(value) => OptionTemplate::Some(value.clone_template()),
            OptionTemplate::None => OptionTemplate::None,
        }
    }
}

/// A [`Template`] for [`Vec`].
pub struct VecTemplate<T>(pub Vec<T>);

impl<T> Default for VecTemplate<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: Template> Template for VecTemplate<T> {
    type Output = Vec<T::Output>;

    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        let mut output = Vec::with_capacity(self.0.len());
        for value in &self.0 {
            output.push(value.build_template(context)?);
        }
        Ok(output)
    }

    fn clone_template(&self) -> Self {
        VecTemplate(self.0.iter().map(Template::clone_template).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use alloc::string::{String, ToString};

    #[test]
    fn option_template() {
        #[derive(FromTemplate)]
        struct Handle(String);

        #[derive(FromTemplate)]
        struct Foo {
            #[template(built_in)]
            handle: Option<Handle>,
        }

        let mut world = World::new();
        let foo_template = FooTemplate {
            handle: Some(HandleTemplate("handle_path".to_string())).into(),
        };
        let foo = world.spawn_empty().build_template(&foo_template).unwrap();
        assert_eq!(foo.handle.unwrap().0, "handle_path".to_string());
    }
}
