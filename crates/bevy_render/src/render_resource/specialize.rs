use super::{
    CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline, ComputePipelineDescriptor,
    PipelineCache, RenderPipeline, RenderPipelineDescriptor,
};
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_utils::hashbrown::HashMap;
use core::hash::Hash;

pub use bevy_render_macros::{SpecializeComputePipeline, SpecializeRenderPipeline};

pub trait SpecializeTarget {
    type Descriptor: Clone + Send + Sync;
    type CachedId: Clone + Send + Sync;
    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId;
}

impl SpecializeTarget for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type CachedId = CachedRenderPipelineId;

    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId {
        pipeline_cache.queue_render_pipeline(descriptor)
    }
}

impl SpecializeTarget for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;

    type CachedId = CachedComputePipelineId;

    fn queue(pipeline_cache: &PipelineCache, descriptor: Self::Descriptor) -> Self::CachedId {
        pipeline_cache.queue_compute_pipeline(descriptor)
    }
}

pub trait Specialize<T: SpecializeTarget>: Send + Sync + 'static {
    type Key: Clone + Hash + Eq;
    fn specialize(&self, key: Self::Key, descriptor: &mut T::Descriptor);
}

pub trait HasBaseDescriptor<T: SpecializeTarget>: Specialize<T> {
    fn base_descriptor(&self) -> T::Descriptor;
}

#[derive(Resource)]
pub struct Specializer<T: SpecializeTarget, S: Specialize<T>> {
    specializer: S,
    user_specializer: Option<fn(S::Key, &mut T::Descriptor)>,
    base_descriptor: T::Descriptor,
    pipelines: HashMap<S::Key, T::CachedId>,
}

impl<T: SpecializeTarget, S: Specialize<T>> Specializer<T, S> {
    pub fn new(
        specializer: S,
        user_specializer: Option<fn(S::Key, &mut T::Descriptor)>,
        base_descriptor: T::Descriptor,
    ) -> Self {
        Self {
            specializer,
            user_specializer,
            base_descriptor,
            pipelines: HashMap::new(),
        }
    }

    pub fn specialize(&mut self, pipeline_cache: &PipelineCache, key: S::Key) -> T::CachedId {
        self.pipelines
            .entry(key.clone())
            .or_insert_with(|| {
                let mut descriptor = self.base_descriptor.clone();
                self.specializer.specialize(key.clone(), &mut descriptor);
                if let Some(user_specializer) = self.user_specializer {
                    (user_specializer)(key, &mut descriptor);
                }
                <T as SpecializeTarget>::queue(pipeline_cache, descriptor)
            })
            .clone()
    }
}

impl<T: SpecializeTarget, S: FromWorld + HasBaseDescriptor<T>> FromWorld for Specializer<T, S> {
    fn from_world(world: &mut World) -> Self {
        let specializer = S::from_world(world);
        let base_descriptor = specializer.base_descriptor();
        Self::new(specializer, None, base_descriptor)
    }
}
