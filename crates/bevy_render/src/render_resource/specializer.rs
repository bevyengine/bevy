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

pub use bevy_render_macros::{Specializer, SpecializerKey};

/// Defines a type that is able to be "specialized" and cached by creating and transforming
/// its descriptor type. This is implemented for [`RenderPipeline`] and [`ComputePipeline`], and
/// likely will not have much utility for other types.
///
/// See docs on [`Specializer`] for more info.
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

/// Defines a type capable of "specializing" values of a type T.
///
/// Specialization is the process of generating variants of a type T
/// from small hashable keys, and specializers themselves can be
/// thought of as [pure functions] from the key type to `T`, that
/// [memoize] their results based on the key.
///
/// <div class="warning">
/// Because specialization is designed for use with render and compute
/// pipelines, specializers act on <i>descriptors</i> of <code>T</code> rather
/// than produce <code>T</code> itself, but the above comparison is still valid.
/// </div>
///
/// Since compiling render and compute pipelines can be so slow,
/// specialization allows a Bevy app to detect when it would compile
/// a duplicate pipeline and reuse what's already in the cache. While
/// pipelines could all be memoized hashing each whole descriptor, this
/// would be much slower and could still create duplicates. In contrast,
/// memoizing groups of *related* pipelines based on a small hashable
/// key is much faster. See the docs on [`SpecializerKey`] for more info.
///
/// ## Composing Specializers
///
/// This trait can be derived with `#[derive(Specializer)]` for structs whose
/// fields all implement [`Specializer`]. This allows for composing multiple
/// specializers together, and makes encapsulation and separating concerns
/// between specializers much nicer. One could make individual specializers
/// for common operations and place them in entirely separate modules, then
/// compose them together with a single `#[derive]`
///
/// ```rust
/// # use bevy_ecs::error::BevyError;
/// # use bevy_render::render_resource::Specializer;
/// # use bevy_render::render_resource::SpecializerKey;
/// # use bevy_render::render_resource::RenderPipeline;
/// # use bevy_render::render_resource::RenderPipelineDescriptor;
/// struct A;
/// struct B;
/// #[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
/// struct BKey { contrived_number: u32 };
///
/// impl Specializer<RenderPipeline> for A {
///     type Key = ();
///
///     fn specialize(
///         &self,
///         key: (),
///         descriptor: &mut RenderPipelineDescriptor
///     ) -> Result<(), BevyError>  {
/// #       let _ = descriptor;
///         // mutate the descriptor here
///         Ok(key)
///     }
/// }
///
/// impl Specializer<RenderPipeline> for B {
///     type Key = BKey;
///
///     fn specialize(
///         &self,
///         key: BKey,
///         descriptor: &mut RenderPipelineDescriptor
///     ) -> Result<BKey, BevyError> {
/// #       let _ = descriptor;
///         // mutate the descriptor here
///         Ok(key)
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
///
/// The key type for a composed specializer will be a tuple of the keys
/// of each field, and their specialization logic will be applied in field
/// order. Since derive macros can't have generic parameters, the derive macro
/// requires an additional `#[specialize(..targets)]` attribute to specify a
/// list of types to target for the implementation. `#[specialize(all)]` is
/// also allowed, and will generate a fully generic implementation at the cost
/// of slightly worse error messages.
///
/// Additionally, each field can optionally take a `#[key]` attribute to
/// specify a "key override". This will hide that field's key from being
/// exposed by the wrapper, and always use the value given by the attribute.
/// Values for this attribute may either be `default` which will use the key's
/// [`Default`] implementation, or a valid rust expression of the key type.
///
/// [pure functions]: https://en.wikipedia.org/wiki/Pure_function
/// [memoize]: https://en.wikipedia.org/wiki/Memoization
pub trait Specializer<T: Specializable>: Send + Sync + 'static {
    type Key: SpecializerKey;
    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut T::Descriptor,
    ) -> Result<Canonical<Self::Key>, BevyError>;
}

// TODO: update docs for `SpecializerKey` with a more concrete example
// once we've migrated mesh layout specialization

/// Defines a type that is able to be used as a key for [`Specializer`]s
///
/// <div class = "warning">
/// <strong>Most types should implement this trait with the included derive macro.</strong> <br/>
/// This generates a "canonical" key type, with <code>IS_CANONICAL = true</code>, and <code>Canonical = Self</code>
/// </div>
///
/// ## What's a "canonical" key?
///
/// The specialization API memoizes pipelines based on the hash of each key, but this
/// can still produce duplicates. For example, if one used a list of vertex attributes
/// as a key, even if all the same attributes were present they could be in any order.
/// In each case, though the keys would be "different" they would produce the same
/// pipeline.
///
/// To address this, during specialization keys are processed into a [canonical]
/// (or "standard") form that represents the actual descriptor that was produced.
/// In the previous example, that would be the final `VertexBufferLayout` contained
/// by the pipeline descriptor. This new key is used by [`SpecializedCache`] to
/// perform additional checks for duplicates, but only if required. If a key is
/// canonical from the start, then there's no need.
///
/// For implementors: the main property of a canonical key is that if two keys hash
/// differently, they should nearly always produce different descriptors.
///
/// [canonical]: https://en.wikipedia.org/wiki/Canonicalization
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

// TODO: How to we fake_variadics this?
all_tuples!(impl_specialization_key_tuple, 0, 12, T);

/// Defines a specializer that can also provide a "base descriptor".
///
/// In order to be composable, [`Specializer`] implementers don't create full
/// descriptors, only transform them. However, [`SpecializedCache`]s need a
/// "base descriptor" at creation time in order to have something for the
/// [`Specializer`] to work off of. This trait allows [`SpecializedCache`]
/// to impl [`FromWorld`] for [`Specializer`]s that also satisfy [`FromWorld`]
/// and [`GetBaseDescriptor`].
///
/// This trait can be also derived with `#[derive(Specializer)]`, by marking
/// a field with `#[base_descriptor]` to use its [`GetBaseDescriptor`] implementation.
///
/// Example:
/// ```rust
/// # use bevy_ecs::error::BevyError;
/// # use bevy_render::render_resource::Specializer;
/// # use bevy_render::render_resource::GetBaseDescriptor;
/// # use bevy_render::render_resource::SpecializerKey;
/// # use bevy_render::render_resource::RenderPipeline;
/// # use bevy_render::render_resource::RenderPipelineDescriptor;
/// struct A;
/// struct B;
///
/// impl Specializer<RenderPipeline> for A {
/// #   type Key = ();
/// #
/// #   fn specialize(
/// #       &self,
/// #       key: (),
/// #       _descriptor: &mut RenderPipelineDescriptor
/// #   ) -> Result<(), BevyError> {
/// #       Ok(key)
/// #   }
///     // ...
/// }
///
/// impl Specializer<RenderPipeline> for B {
/// #   type Key = ();
/// #
/// #   fn specialize(
/// #       &self,
/// #       key: (),
/// #       _descriptor: &mut RenderPipelineDescriptor
/// #   ) -> Result<(), BevyError> {
/// #       Ok(key)
/// #   }
///     // ...
/// }
///
/// impl GetBaseDescriptor<RenderPipeline> for B {
///     fn get_base_descriptor(&self) -> RenderPipelineDescriptor {
/// #       todo!()
///         // ...
///     }
/// }
///
///
/// #[derive(Specializer)]
/// #[specialize(RenderPipeline)]
/// struct C {
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
