//! Functionality that relates to the [`Template`] trait.

pub use bevy_ecs_macros::GetTemplate;

use crate::{
    bundle::Bundle,
    entity::Entity,
    error::{BevyError, Result},
    resource::Resource,
    world::{EntityWorldMut, Mut, World},
};
use alloc::{boxed::Box, vec, vec::Vec};
use bevy_platform::collections::hash_map::Entry;
use bevy_utils::TypeIdMap;
use core::any::{Any, TypeId};
use downcast_rs::{impl_downcast, Downcast};
use variadics_please::all_tuples;

/// A [`Template`] is something that, given a spawn context (target [`Entity`], [`World`](crate::world::World), etc), can produce a [`Template::Output`].
pub trait Template {
    /// The type of value produced by this [`Template`].
    type Output;

    /// Uses this template and the given `entity` context to produce a [`Template::Output`].
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output>;

    /// Clones this template. See [`Clone`].
    fn clone_template(&self) -> Self;

    /// This is used to register information about the template, such as dependencies that should be loaded before it is instantiated.
    #[inline]
    fn register_data(&self, _data: &mut TemplateData) {}
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
    pub fn resource_mut<R: Resource>(&mut self) -> Mut<'_, R> {
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

/// [`GetTemplate`] is implemented for types that can be produced by a specific, canonical [`Template`]. This creates a way to correlate to the [`Template`] using the
/// desired template output type. This is used by Bevy's scene system.
pub trait GetTemplate: Sized {
    /// The [`Template`] for this type.
    type Template: Template;
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

            fn register_data(&self, _data: &mut TemplateData) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($template,)*) = &self.0;
                $($template.register_data(_data);)*
            }
        }
    }
}

/// A wrapper over a tuple of [`Template`] implementations, which also implements [`Template`]. This exists because [`Template`] cannot
/// be directly implemented for tuples of [`Template`] implementations.
pub struct TemplateTuple<T>(pub T);

all_tuples!(template_impl, 0, 12, T);

impl<T: Clone + Default> Template for T {
    type Output = T;

    fn build_template(&self, _context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(self.clone())
    }

    fn clone_template(&self) -> Self {
        self.clone()
    }
}

impl<T: Clone + Default> GetTemplate for T {
    type Template = T;
}

/// A [`Template`] reference to an [`Entity`].
pub enum EntityReference {
    /// A reference to an entity via a [`ScopedEntityIndex`]
    ScopedEntityIndex(ScopedEntityIndex),
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

impl Default for EntityReference {
    fn default() -> Self {
        Self::ScopedEntityIndex(ScopedEntityIndex { scope: 0, index: 0 })
    }
}

impl Template for EntityReference {
    type Output = Entity;

    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(match self {
            // unwrap is ok as this is "internals". when implemented correctly this will never panic
            EntityReference::ScopedEntityIndex(scoped_entity_index) => {
                context.get_scoped_entity(*scoped_entity_index)
            }
        })
    }

    fn clone_template(&self) -> Self {
        match self {
            Self::ScopedEntityIndex(scoped_entity_index) => {
                Self::ScopedEntityIndex(*scoped_entity_index)
            }
        }
    }
}

impl GetTemplate for Entity {
    type Template = EntityReference;
}

/// A type-erased, object-safe, downcastable version of [`Template`].
pub trait ErasedTemplate: Downcast + Send + Sync {
    /// Applies this template to the given `entity`.
    fn apply(&self, context: &mut TemplateContext) -> Result<(), BevyError>;

    /// Clones this template. See [`Clone`].
    fn clone_template(&self) -> Box<dyn ErasedTemplate>;
}

impl_downcast!(ErasedTemplate);

impl<T: Template<Output: Bundle> + Send + Sync + 'static> ErasedTemplate for T {
    fn apply(&self, context: &mut TemplateContext) -> Result<(), BevyError> {
        let bundle = self.build_template(context)?;
        context.entity.insert(bundle);
        Ok(())
    }

    fn clone_template(&self) -> Box<dyn ErasedTemplate> {
        Box::new(Template::clone_template(self))
    }
}

/// This is used by the [`GetTemplate`] derive to work around [this Rust limitation](https://github.com/rust-lang/rust/issues/86935).
/// A fix is implemented and on track for stabilization. If it is ever implemented, we can remove this.
pub type Wrapper<T> = T;

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

/// Arbitrary data storage which can be used by [`Template`] implementations to register metadata such as asset dependencies.
#[derive(Default)]
pub struct TemplateData(TypeIdMap<Box<dyn Any>>);

impl TemplateData {
    /// Adds the `value` to this storage. This will be added to the back of a list of other values of the same type.
    pub fn add<T: Any + Send + Sync>(&mut self, value: T) {
        match self.0.entry(TypeId::of::<T>()) {
            Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .downcast_mut::<Vec<T>>()
                    .unwrap()
                    .push(value);
            }
            Entry::Vacant(entry) => {
                entry.insert(Box::new(vec![value]));
            }
        }
    }

    /// Iterates over all stored values of the given type `T`.
    pub fn iter<T: Any>(&self) -> impl Iterator<Item = &T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<Vec<T>>())
            .map(|v| v.iter())
            .unwrap_or_default()
    }
}
