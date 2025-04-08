use super::{
    CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline, ComputePipelineDescriptor,
    PipelineCache, RenderPipeline, RenderPipelineDescriptor,
};
use bevy_ecs::{
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_platform_support::collections::HashMap;
use core::{hash::Hash, marker::PhantomData};

pub use bevy_render_macros::{HasBaseDescriptor, Specialize};

/// Defines a type that is able to be "specialized" and cached by creating and transforming
/// its descriptor type. This is implemented for [`RenderPipeline`] and [`ComputePipeline`], and
/// likely will not have much utility for other types.
pub trait Specializable {
    type Descriptor: Clone + Send + Sync;
    type CachedId: Clone + Send + Sync;
    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId;
}

impl Specializable for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type CachedId = CachedRenderPipelineId;

    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId {
        pipeline_cache.queue_render_pipeline(descriptor)
    }
}

impl Specializable for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;

    type CachedId = CachedComputePipelineId;

    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId {
        pipeline_cache.queue_compute_pipeline(descriptor)
    }
}

/// Defines a type that is able to transform descriptors for a specializable
/// type T, based on a known hashable key type.
///
/// This is mainly used when "specializing" render
/// pipelines, i.e. specifying shader defs and binding layout based on the key,
/// the result of which can then be cached and accessed quickly later.
///
/// This trait can be derived with `#[derive(Specialize)]` for structs whose
/// fields all implement [`Specialize`]. The key type will be tuple of the keys
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
/// struct A;
/// struct B;
///
/// impl Specialize<RenderPipeline> for A {
///     type Key = ();
///
///     fn specialize(&self, _key: (), _descriptor: &mut RenderPipelineDescriptor) {
///         //...
///     }
/// }
///
/// impl Specialize<RenderPipeline> for B {
///     type Key = u32;
///
///     fn specialize(&self, _key: u32, _descriptor: &mut RenderPipelineDescriptor) {
///         //...
///     }
/// }
///
/// #[derive(Specialize)]
/// #[specialize(RenderPipeline)]
/// struct C {
///     #[key(default)]
///     a: A,
///     b: B,
/// }
///
/// /*
/// The generated implementation:
/// impl Specialize<RenderPipeline> for C {
///     type Key = u32;
///     fn specialize(&self, key: u32, descriptor: &mut RenderPipelineDescriptor) {
///         self.a.specialize((), descriptor);
///         self.b.specialize(key, descriptor);
///     }
/// }
/// */
/// ```
pub trait Specialize<T: Specializable>: Send + Sync + 'static {
    type Key: Clone + Hash + Eq;
    fn specialize(&self, key: Self::Key, descriptor: &mut T::Descriptor);
}

impl<T: Specializable> Specialize<T> for () {
    type Key = ();

    fn specialize(&self, _key: Self::Key, _descriptor: &mut T::Descriptor) {}
}

impl<T: Specializable, V: Send + Sync + 'static> Specialize<T> for PhantomData<V> {
    type Key = ();

    fn specialize(&self, _key: Self::Key, _descriptor: &mut T::Descriptor) {}
}

/// Defines a specializer that can also provide a "base descriptor".
///
/// In order to be composable, [`Specialize`] implementers don't create full
/// descriptors, only transform them. However, [`Specializer`]s need a "base
/// descriptor" at creation time in order to have something for the
/// [`Specialize`] implementation to work off of. This trait allows
/// [`Specializer`] to impl [`FromWorld`] for [`Specialize`] implementationss
/// that also satisfy [`FromWorld`] and [`HasBaseDescriptor`].
///
/// This trait can be derived with `#[derive(HasBaseDescriptor)]` for structs.
/// Like `#[derive(Specialize)]`, it requires an additional.
/// `#[specialize(..targets)]` attribute. See the [`Specialize`] docs for more
/// info.
///
/// If the struct has a single field, it will defer to that field's
/// [`HasBaseDescriptor`] implementation. Otherwise a single
/// `#[base_descriptor]` attribute is required to mark which field to defer to.
///
/// Example:
/// ```rs
/// struct A;
/// struct B;
///
/// impl Specialize<RenderPipeline> for A {
///     type Key = ();
///
///     fn specialize(&self, _key: (), _descriptor: &mut RenderPipelineDescriptor) {
///         //...
///     }
/// }
///
/// impl Specialize<RenderPipeline> for B {
///     type Key = u32;
///
///     fn specialize(&self, _key: u32, _descriptor: &mut RenderPipelineDescriptor) {
///         //...
///     }
/// }
///
/// impl HasBaseDescriptor<RenderPipeline> for B {
///     fn base_descriptor(&self) -> RenderPipelineDescriptor {
/// #       todo!()
///         //...
///     }
/// }
///
///
/// #[derive(Specialize, HasBaseDescriptor)]
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
/// impl HasBaseDescriptor for C {
///     fn base_descriptor(&self) -> RenderPipelineDescriptor {
///         self.b.base_descriptor()
///     }
/// }
/// */
/// ```
pub trait HasBaseDescriptor<T: Specializable>: Specialize<T> {
    fn base_descriptor(&self) -> T::Descriptor;
}

pub type SpecializeFn<T, S> = fn(<S as Specialize<T>>::Key, &mut <T as Specializable>::Descriptor);

/// A cache for specializable resources. For a given key type the resulting
/// resource will only be created if it is missing, retrieving it from the
/// cache otherwise.
#[derive(Resource)]
pub struct Specializer<T: Specializable, S: Specialize<T>> {
    specializer: S,
    user_specializer: Option<SpecializeFn<T, S>>,
    base_descriptor: T::Descriptor,
    specialized: HashMap<S::Key, T::CachedId>,
}

impl<T: Specializable, S: Specialize<T>> Specializer<T, S> {
    /// Creates a new [`Specializer`] from a [`Specialize`] implementation,
    /// an optional "user specializer", and a base descriptor. The user
    /// specializer is applied after the [`Specialize`] implementation, with
    /// the same key.
    pub fn new(
        specializer: S,
        user_specializer: Option<SpecializeFn<T, S>>,
        base_descriptor: T::Descriptor,
    ) -> Self {
        Self {
            specializer,
            user_specializer,
            base_descriptor,
            specialized: Default::default(),
        }
    }

    /// Specializes a resource given the [`Specialize`] implementation's key type.
    pub fn specialize(&mut self, pipeline_cache: &PipelineCache, key: S::Key) -> T::CachedId {
        self.specialized
            .entry(key.clone())
            .or_insert_with(|| {
                let mut descriptor = self.base_descriptor.clone();
                self.specializer.specialize(key.clone(), &mut descriptor);
                if let Some(user_specializer) = self.user_specializer {
                    (user_specializer)(key, &mut descriptor);
                }
                <T as Specializable>::queue(pipeline_cache, descriptor)
            })
            .clone()
    }
}

/// [`Specializer`] implements [`FromWorld`] for [`Specialize`] implementations
/// that also satisfy [`FromWorld`] and [`HasBaseDescriptor`]. This will create
/// a [`Specializer`] with no user specializer, and the base descriptor taken
/// from the [`Specialize`] implementation.
impl<T, S> FromWorld for Specializer<T, S>
where
    T: Specializable,
    S: FromWorld + Specialize<T> + HasBaseDescriptor<T>,
{
    fn from_world(world: &mut World) -> Self {
        let specializer = S::from_world(world);
        let base_descriptor = specializer.base_descriptor();
        Self::new(specializer, None, base_descriptor)
    }
}
