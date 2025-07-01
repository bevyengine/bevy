use super::{
    CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline, ComputePipelineDescriptor,
    PipelineCache, RenderPipeline, RenderPipelineDescriptor,
};
use bevy_ecs::{
    error::BevyError,
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_platform::{
    collections::{
        hash_map::{Entry, VacantEntry},
        HashMap,
    },
    hash::FixedHasher,
};
use core::{hash::Hash, marker::PhantomData};
use tracing::error;
use variadics_please::all_tuples;

pub use bevy_render_macros::{Specialize, SpecializerKey};

/// Defines a type that is able to be "specialized" and cached by creating and transforming
/// its descriptor type. This is implemented for [`RenderPipeline`] and [`ComputePipeline`], and
/// likely will not have much utility for other types.
pub trait Specializable {
    type Descriptor: PartialEq + Clone + Send + Sync;
    type CachedId: Clone + Send + Sync;
    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId;
    fn get_descriptor(pipeline_cache: &PipelineCache, id: Self::CachedId) -> &Self::Descriptor;
}

impl Specializable for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type CachedId = CachedRenderPipelineId;

    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId {
        pipeline_cache.queue_render_pipeline(descriptor)
    }

    fn get_descriptor(
        pipeline_cache: &PipelineCache,
        id: CachedRenderPipelineId,
    ) -> &Self::Descriptor {
        pipeline_cache.get_render_pipeline_descriptor(id)
    }
}

impl Specializable for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;

    type CachedId = CachedComputePipelineId;

    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId {
        pipeline_cache.queue_compute_pipeline(descriptor)
    }

    fn get_descriptor(
        pipeline_cache: &PipelineCache,
        id: CachedComputePipelineId,
    ) -> &Self::Descriptor {
        pipeline_cache.get_compute_pipeline_descriptor(id)
    }
}

/// Defines a type that is able to transform descriptors for a specializable
/// type T, based on a hashable key type.
///
/// This is mainly used when "specializing" render
/// pipelines, i.e. specifying shader defs and binding layout based on the key,
/// the result of which can then be cached and accessed quickly later.
///
/// This trait can be derived with `#[derive(Specializer)]` for structs whose
/// fields all implement [`Specializer`]. The key type will be tuple of the keys
/// of each field, and their specialization logic will be applied in field
/// order. Since derive macros can't have generic parameters, the derive macro
/// requires an additional `#[specialize(..targets)]` attribute to specify a
/// list of types to target for the implementation. `#[specialize(all)]` is
/// also allowed, and will generate a fully generic implementation at the cost
/// of slightly worse error messages.
///
/// Additionally, each field can optionally take a `#[key]` attribute to
/// specify a "key override". This will "hide" that field's key from being
/// exposed by the wrapper, and always use the value given by the attribute.
/// Values for this attribute may either be `default` which will use the key's
/// [`Default`] implementation, or a valid rust
/// expression of the key type.
///
/// Example:
/// ```rs
/// # use super::RenderPipeline;
/// # use super::RenderPipelineDescriptor;
/// # use bevy_ecs::error::BevyError;
///
/// struct A;
/// struct B;
/// #[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
/// struct BKey;
///
/// impl Specializer<RenderPipeline> for A {
///     type Key = ();
///
///     fn specializer(&self, key: (), descriptor: &mut RenderPipelineDescriptor) -> Result<(), BevyError>  {
/// #       let _ = (key, descriptor);
///         //...
///         Ok(())
///     }
/// }
///
/// impl Specializer<RenderPipeline> for B {
///     type Key = BKey;
///
///     fn specialize(&self, _key: Bkey, _descriptor: &mut RenderPipelineDescriptor) -> Result<BKey, BevyError> {
/// #       let _ = (key, descriptor);
///         //...
///         Ok(BKey)
///     }
/// }
///
/// #[derive(Specializer)]
/// #[specialize(RenderPipeline)]
/// struct C {
///     #[key(default)]
///     a: A,
///     b: B,
/// }
///
/// /*
/// The generated implementation:
/// impl Specializer<RenderPipeline> for C {
///     type Key = BKey;
///     fn specialize(
///         &self,
///         key: Self::Key,
///         descriptor: &mut RenderPipelineDescriptor
///     ) -> Result<Canonical<Self::Key>, BevyError> {
///         let _ = self.a.specialize((), descriptor);
///         let key = self.b.specialize(key, descriptor);
///         Ok(key)
///     }
/// }
/// */
/// ```
pub trait Specializer<T: Specializable>: Send + Sync + 'static {
    type Key: SpecializerKey;
    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut T::Descriptor,
    ) -> Result<Canonical<Self::Key>, BevyError>;
}

/// Defines a type that is able to be used as a key for types that `impl Specialize`
///
/// **Most types should implement this trait with `IS_CANONICAL = true` and `Canonical = Self`**.
/// This is the implementation generated by `#[derive(SpecializerKey)]`
///
/// In this case, "canonical" means that each unique value of this type will produce
/// a unique specialized result, which isn't true in general. `MeshVertexBufferLayout`
/// is a good example of a type that's `Eq + Hash`, but that isn't canonical: vertex
/// attributes could be specified in any order, or there could be more attributes
/// provided than the specialized pipeline requires. Its `Canonical` key type would
/// be `VertexBufferLayout`, the final layout required by the pipeline.
///
/// Processing keys into canonical keys this way allows the `SpecializedCache` to reuse
/// resources more eagerly where possible.
pub trait SpecializerKey: Clone + Hash + Eq {
    /// Denotes whether this key is canonical or not. This should only be `true`
    /// if and only if `Canonical = Self`.
    const IS_CANONICAL: bool;

    /// The canonical key type to convert this into during specialization.
    type Canonical: Hash + Eq;
}

pub type Canonical<T> = <T as SpecializerKey>::Canonical;

impl<T: Specializable> Specializer<T> for () {
    type Key = ();

    fn specialize(
        &self,
        _key: Self::Key,
        _descriptor: &mut T::Descriptor,
    ) -> Result<(), BevyError> {
        Ok(())
    }
}

impl<T: Specializable, V: Send + Sync + 'static> Specializer<T> for PhantomData<V> {
    type Key = ();

    fn specialize(
        &self,
        _key: Self::Key,
        _descriptor: &mut T::Descriptor,
    ) -> Result<(), BevyError> {
        Ok(())
    }
}

macro_rules! impl_specialization_key_tuple {
    ($($T:ident),*) => {
        impl <$($T: SpecializerKey),*> SpecializerKey for ($($T,)*) {
            const IS_CANONICAL: bool = true $(&& <$T as SpecializerKey>::IS_CANONICAL)*;
            type Canonical = ($(Canonical<$T>,)*);
        }
    };
}

all_tuples!(impl_specialization_key_tuple, 0, 12, T);

/// Defines a specializer that can also provide a "base descriptor".
///
/// In order to be composable, [`Specializer`] implementers don't create full
/// descriptors, only transform them. However, [`SpecializedCache`]s need a
/// "base descriptor" at creation time in order to have something for the
/// [`Specializer`] implementation to work off of. This trait allows
/// [`SpecializedCache`] to impl [`FromWorld`] for [`Specializer`]
/// implementations that also satisfy [`FromWorld`] and [`GetBaseDescriptor`].
///
/// This trait can be also derived with `#[derive(Specializer)]`, by marking
/// a field with `#[base_descriptor]` to use its [`GetBaseDescriptor`] implementation.
///
/// Example:
/// ```rs
/// struct A;
/// struct B;
///
/// impl Specializer<RenderPipeline> for A {
///     type Key = ();
///
///     fn specialize(&self, _key: (), _descriptor: &mut RenderPipelineDescriptor) {
///         //...
///     }
/// }
///
/// impl Specializer<RenderPipeline> for B {
///     type Key = u32;
///
///     fn specialize(&self, _key: u32, _descriptor: &mut RenderPipelineDescriptor) {
///         //...
///     }
/// }
///
/// impl GetBaseDescriptor<RenderPipeline> for B {
///     fn get_base_descriptor(&self) -> RenderPipelineDescriptor {
/// #       todo!()
///         //...
///     }
/// }
///
///
/// #[derive(Specializer)]
/// #[specialize(RenderPipeline)]
/// struct C {
///     #[key(default)]
///     a: A,
///     #[base_descriptor]
///     b: B,
/// }
///
/// /*
/// The generated implementation:
/// impl GetBaseDescriptor for C {
///     fn get_base_descriptor(&self) -> RenderPipelineDescriptor {
///         self.b.base_descriptor()
///     }
/// }
/// */
/// ```
pub trait GetBaseDescriptor<T: Specializable>: Specializer<T> {
    fn get_base_descriptor(&self) -> T::Descriptor;
}

pub type SpecializerFn<T, S> =
    fn(<S as Specializer<T>>::Key, &mut <T as Specializable>::Descriptor) -> Result<(), BevyError>;

/// A cache for specializable resources. For a given key type the resulting
/// resource will only be created if it is missing, retrieving it from the
/// cache otherwise.
#[derive(Resource)]
pub struct SpecializedCache<T: Specializable, S: Specializer<T>> {
    specializer: S,
    user_specializer: Option<SpecializerFn<T, S>>,
    base_descriptor: T::Descriptor,
    primary_cache: HashMap<S::Key, T::CachedId>,
    secondary_cache: HashMap<Canonical<S::Key>, T::CachedId>,
}

impl<T: Specializable, S: Specializer<T>> SpecializedCache<T, S> {
    /// Creates a new [`SpecializedCache`] from a [`Specializer`],
    /// an optional "user specializer", and a base descriptor. The
    /// user specializer is applied after the [`Specializer`], with
    /// the same key.
    #[inline]
    pub fn new(
        specializer: S,
        user_specializer: Option<SpecializerFn<T, S>>,
        base_descriptor: T::Descriptor,
    ) -> Self {
        Self {
            specializer,
            user_specializer,
            base_descriptor,
            primary_cache: Default::default(),
            secondary_cache: Default::default(),
        }
    }

    /// Specializes a resource given the [`Specializer`]'s key type.
    #[inline]
    pub fn specialize(
        &mut self,
        pipeline_cache: &PipelineCache,
        key: S::Key,
    ) -> Result<T::CachedId, BevyError> {
        let entry = self.primary_cache.entry(key.clone());
        match entry {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => Self::specialize_slow(
                &self.specializer,
                self.user_specializer,
                self.base_descriptor.clone(),
                pipeline_cache,
                key,
                entry,
                &mut self.secondary_cache,
            ),
        }
    }

    #[cold]
    fn specialize_slow(
        specializer: &S,
        user_specializer: Option<SpecializerFn<T, S>>,
        base_descriptor: T::Descriptor,
        pipeline_cache: &PipelineCache,
        key: S::Key,
        primary_entry: VacantEntry<S::Key, T::CachedId, FixedHasher>,
        secondary_cache: &mut HashMap<Canonical<S::Key>, T::CachedId>,
    ) -> Result<T::CachedId, BevyError> {
        let mut descriptor = base_descriptor.clone();
        let canonical_key = specializer.specialize(key.clone(), &mut descriptor)?;

        if let Some(user_specializer) = user_specializer {
            (user_specializer)(key, &mut descriptor)?;
        }

        // if the whole key is canonical, the secondary cache isn't needed.
        if <S::Key as SpecializerKey>::IS_CANONICAL {
            return Ok(primary_entry
                .insert(<T as Specializable>::queue(pipeline_cache, descriptor))
                .clone());
        }

        let id = match secondary_cache.entry(canonical_key) {
            Entry::Occupied(entry) => {
                if cfg!(debug_assertions) {
                    let stored_descriptor =
                        <T as Specializable>::get_descriptor(pipeline_cache, entry.get().clone());
                    if &descriptor != stored_descriptor {
                        error!(
                            "Invalid Specializer<{}> impl for {}: the cached descriptor \
                            is not equal to the generated descriptor for the given key. \
                            This means the Specializer implementation uses unused information \
                            from the key to specialize the pipeline. This is not allowed \
                            because it would invalidate the cache.",
                            core::any::type_name::<T>(),
                            core::any::type_name::<S>()
                        );
                    }
                }
                entry.into_mut().clone()
            }
            Entry::Vacant(entry) => entry
                .insert(<T as Specializable>::queue(pipeline_cache, descriptor))
                .clone(),
        };

        primary_entry.insert(id.clone());
        Ok(id)
    }
}

/// [`SpecializedCache`] implements [`FromWorld`] for [`Specializer`]s
/// that also satisfy [`FromWorld`] and [`GetBaseDescriptor`]. This will
/// create a [`SpecializedCache`] with no user specializer, and the base
/// descriptor take from the specializer's [`GetBaseDescriptor`] implementation.
impl<T, S> FromWorld for SpecializedCache<T, S>
where
    T: Specializable,
    S: FromWorld + Specializer<T> + GetBaseDescriptor<T>,
{
    fn from_world(world: &mut World) -> Self {
        let specializer = S::from_world(world);
        let base_descriptor = specializer.get_base_descriptor();
        Self::new(specializer, None, base_descriptor)
    }
}
