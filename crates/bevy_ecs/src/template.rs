//! Functionality that relates to the [`Template`] trait.

pub use bevy_ecs_macros::GetTemplate;

use crate::{
    bundle::Bundle,
    entity::{Entity, EntityPath},
    error::{BevyError, Result},
    world::EntityWorldMut,
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
    fn build(&mut self, entity: &mut EntityWorldMut) -> Result<Self::Output>;

    /// This is used to register information about the template, such as dependencies that should be loaded before it is instantiated.
    #[inline]
    fn register_data(&self, _data: &mut TemplateData) {}
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
            fn build(&mut self, _entity: &mut EntityWorldMut) -> Result<Self::Output> {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($template,)*) = &mut self.0;
                Ok(($($template.build(_entity)?,)*))
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

impl Template for EntityPath<'static> {
    type Output = Entity;

    fn build(&mut self, entity: &mut EntityWorldMut) -> Result<Self::Output> {
        Ok(entity.resolve_path(self)?)
    }
}

impl GetTemplate for Entity {
    type Template = EntityPath<'static>;
}

impl<T: Clone + Default> Template for T {
    type Output = T;

    fn build(&mut self, _entity: &mut EntityWorldMut) -> Result<Self::Output> {
        Ok(self.clone())
    }
}

impl<T: Clone + Default> GetTemplate for T {
    type Template = T;
}

/// A type-erased, object-safe, downcastable version of [`Template`].
pub trait ErasedTemplate: Downcast + Send + Sync {
    /// Applies this template to the given `entity`.
    fn apply(&mut self, entity: &mut EntityWorldMut) -> Result<(), BevyError>;
}

impl_downcast!(ErasedTemplate);

impl<T: Template<Output: Bundle> + Send + Sync + 'static> ErasedTemplate for T {
    fn apply(&mut self, entity: &mut EntityWorldMut) -> Result<(), BevyError> {
        let bundle = self.build(entity)?;
        entity.insert(bundle);
        Ok(())
    }
}

// TODO: Consider cutting this
/// A [`Template`] implementation that holds _either_ a [`Template`] value _or_ the [`Template::Output`] value.
pub enum TemplateField<T: Template> {
    /// A [`Template`].
    Template(T),
    /// A [`Template::Output`].
    Value(T::Output),
}

impl<T: Template + Default> Default for TemplateField<T> {
    fn default() -> Self {
        Self::Template(<T as Default>::default())
    }
}

impl<T: Template<Output: Clone>> Template for TemplateField<T> {
    type Output = T::Output;

    fn build(&mut self, entity: &mut EntityWorldMut) -> Result<Self::Output> {
        Ok(match self {
            TemplateField::Template(value) => value.build(entity)?,
            TemplateField::Value(value) => value.clone(),
        })
    }
}

/// This is used by the [`GetTemplate`] derive to work around [this Rust limitation](https://github.com/rust-lang/rust/issues/86935).
/// A fix is implemented and on track for stabilization. If it is ever implemented, we can remove this.
pub type Wrapper<T> = T;

/// A [`Template`] driven by a function that returns an output. This is used to create "free floating" templates without
/// defining a new type. See [`template`] for usage.
pub struct FnTemplate<F: FnMut(&mut EntityWorldMut) -> Result<O>, O>(pub F);

impl<F: FnMut(&mut EntityWorldMut) -> Result<O>, O> Template for FnTemplate<F, O> {
    type Output = O;

    fn build(&mut self, entity: &mut EntityWorldMut) -> Result<Self::Output> {
        (self.0)(entity)
    }
}

/// Returns a "free floating" template for a given `func`. This prevents the need to define a custom type for one-off templates.
pub fn template<F: FnMut(&mut EntityWorldMut) -> Result<O>, O>(func: F) -> FnTemplate<F, O> {
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
        if let Some(value) = self.0.get(&TypeId::of::<T>()) {
            let value = value.downcast_ref::<Vec<T>>().unwrap();
            value.iter()
        } else {
            [].iter()
        }
    }
}
