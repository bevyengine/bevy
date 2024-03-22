mod function_feature;
pub use function_feature::*;

use bevy_ecs::component::{Component, ComponentDescriptor, ComponentId, StorageType};
use bevy_ecs::world::{EntityRef, World};
use bevy_render::renderer::RenderContext;

use std::marker::PhantomData;
use std::sync::Mutex;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{SystemParam, SystemParamItem};
use bevy_render::render_graph::{Node, NodeRunError, RenderGraphContext, RenderSubGraph};
use bevy_render::render_resource::{WgpuFeatures, WgpuLimits};
use bevy_utils::{all_tuples, CowArc};

pub trait Feature<G: RenderSubGraph>: Sized + Send + Sync + 'static {
    type Sig: FeatureSignature<true>;
    type CompatibilityKey;

    fn check_compatibility(
        &self,
        features: WgpuFeatures,
        limits: WgpuLimits,
    ) -> Self::CompatibilityKey;

    fn dependencies<'s, 'b: 's>(
        &'s self,
        compatibility: Self::CompatibilityKey,
        builder: &'b mut FeatureDependencyBuilder<G, Self>,
    ) -> RenderHandles<'b, true, FeatureInput<G, Self>>;

    fn build(&self, _compatibility: Self::CompatibilityKey, _app: &mut App) {} // for adding systems not associated with the main View

    fn build_feature<'s, 'b: 's>(
        &'s self,
        compatibility: Self::CompatibilityKey,
        builder: &'b mut FeatureBuilder<'b, G, Self>,
        inputs: RenderHandles<'b, true, FeatureInput<G, Self>>,
    ) -> RenderHandles<'b, true, FeatureOutput<G, Self>>;
}

#[derive(PartialEq, Eq, Ord, PartialOrd)]
pub enum FeatureStage {
    Extract = 0,
    SpecializePipelines = 1,
    PrepareResources = 2,
    PrepareBindGroups = 3,
    Dispatch = 4,
}

pub enum Compatibility {
    Full,
    None,
}

#[derive(Clone)]
pub struct RenderHandle<'a, A: FeatureIO<false>> {
    label: Option<CowArc<'static, str>>,
    source: RawRenderHandle<A>,
    data: PhantomData<&'a A>,
}

impl<'a, A: FeatureIO<false>> RenderHandle<'a, A> {
    pub fn hole<L: Into<CowArc<'static, str>>>(label: Option<L>) -> Self {
        RenderHandle {
            label: label.map(|l| l.into()),
            source: RawRenderHandle::hole(),
            data: PhantomData,
        }
    }
}

#[derive(Copy, Clone)]
pub struct RawRenderHandle<A: FeatureIO<false>> {
    source: Option<ComponentId>,
    data: PhantomData<fn() -> A>,
}

impl<A: FeatureIO<false>> RawRenderHandle<A> {
    //SAFETY: the layout of id must match that of A
    unsafe fn from_id(id: ComponentId) -> Self {
        Self {
            source: Some(id),
            data: PhantomData,
        }
    }

    fn hole() -> Self {
        Self {
            source: None,
            data: PhantomData,
        }
    }

    unsafe fn new(id: Option<ComponentId>) -> Self {
        Self {
            source: id,
            data: PhantomData,
        }
    }

    fn get<'w>(&self, entity: EntityRef<'w>) -> Option<&'w A> {
        self.source
            .and_then(|id| entity.get_by_id(id))
            //SAFETY: by construction we can assume that the layout of the internal id is the same as A
            .map(|ptr| unsafe { ptr.deref::<A>() })
    }
}

pub struct FeatureBuilder<'w, G: RenderSubGraph, F: Feature<G>> {
    app: &'w mut App,
    data: PhantomData<fn(G, F)>,
}

impl<'w, G: RenderSubGraph, F: Feature<G>> FeatureBuilder<'w, G, F> {
    pub fn add_sub_feature<'a, Marker: 'static, S: IntoSubFeature<Marker>>(
        &'a mut self,
        stage: FeatureStage,
        input: RenderHandles<'a, true, SubFeatureInput<S::SubFeature>>,
        sub_feature: S,
    ) -> RenderHandles<'a, false, SubFeatureOutput<S::SubFeature>> {
        todo!()
    }

    pub fn map<'a, A: FeatureIO<false>, B: FeatureIO<false>>(
        &'a mut self,
        handles: RenderHandles<'a, false, A>,
        f: impl for<'_w> Fn(A::Item<'_w>) -> B,
    ) -> RenderHandle<'a, B> {
        todo!()
    }

    pub fn map_many<'a, A: FeatureIO<true>, B: FeatureIO<false>>(
        &'a mut self,
        handles: RenderHandles<'a, true, A>,
        f: impl for<'_w> Fn(A::Item<'_w>) -> B,
    ) -> RenderHandle<'a, B> {
        self.app
            .world
            .init_component_with_descriptor(ComponentDescriptor::new::<FeatureComponent<B>>());
        //register system to map from input to output;
        todo!()
    }
}

type RenderHandles<'a, const MULT: bool, A> = <A as FeatureIO<MULT>>::Handles<'a>;

pub trait FeatureIO<const MULT: bool>: Sized + Send + Sync + 'static {
    type RawHandles;
    type Handles<'a>;
    type Item<'w>;

    fn get(
        entity: EntityRef<'_>,
        handles: Self::RawHandles,
    ) -> Option<<Self as FeatureIO<MULT>>::Item<'_>>;
}

impl<A: Send + Sync + 'static> FeatureIO<false> for A {
    type RawHandles = RawRenderHandle<A>;
    type Handles<'a> = RenderHandle<'a, A>;
    type Item<'w> = &'w A;

    fn get(
        entity: EntityRef<'_>,
        handles: Self::RawHandles,
    ) -> Option<<Self as FeatureIO<false>>::Item<'_>> {
        handles.get(entity)
    }
}

macro_rules! impl_feature_io {
    ($(($T: ident, $r: ident, $h: ident)),*) => {
        impl <$($T: FeatureIO<false>),*> FeatureIO<true> for ($($T,)*) {
            type RawHandles = ($(RawRenderHandle<$T>,)*);
            type Handles<'a> = ($(RenderHandle<'a, $T>,)*);
            type Item<'w> = ($(&'w $T,)*);

            #[allow(unused_variables, unreachable_patterns)]
            fn get(
                entity: EntityRef<'_>,
                handles: Self::RawHandles,
            ) -> Option<<Self as FeatureIO<true>>::Item<'_>> {
                let ($($h,)*) = handles;
                match ($($h.get(entity),)*) {
                    ($(Some($r),)*) => Some(($($r,)*)),
                    _ => None,
                }
            }
        }
    };
}

all_tuples!(impl_feature_io, 0, 16, T, r, h);

pub trait FeatureSignature<const MULTI_OUTPUT: bool>: 'static {
    type In: FeatureIO<true>;
    type Out: FeatureIO<MULTI_OUTPUT>;
}

pub struct FeatureSigData<I, O>(PhantomData<fn(I) -> O>);

impl<const MULTI_OUTPUT: bool, I: FeatureIO<true>, O: FeatureIO<MULTI_OUTPUT>>
    FeatureSignature<MULTI_OUTPUT> for FeatureSigData<I, O>
{
    type In = I;
    type Out = O;
}

#[macro_export]
macro_rules! FeatureSig_Macro {
    [$i: ty => $o: ty] => {
        $crate::render_feature::FeatureSigData<$i, $o>
    };
}

pub use FeatureSig_Macro as Sig;

type FeatureInput<G, F> = <<F as Feature<G>>::Sig as FeatureSignature<true>>::In;
type FeatureOutput<G, F> = <<F as Feature<G>>::Sig as FeatureSignature<true>>::Out;
type SubFeatureInput<F> = <<F as SubFeature>::Sig as FeatureSignature<false>>::In;
type SubFeatureOutput<F> = <<F as SubFeature>::Sig as FeatureSignature<false>>::Out;

pub trait SubFeature: Send + Sync + 'static {
    type Sig: FeatureSignature<false>;
    type Param: SystemParam;

    fn run(
        &self,
        view_entity: Entity,
        input: SubFeatureInput<Self>,
        param: &SystemParamItem<Self::Param>,
    ) -> SubFeatureOutput<Self>;
}

pub trait IntoSubFeature<Marker>: 'static {
    type SubFeature: SubFeature;

    fn into_sub_feature(self) -> Self::SubFeature;
}

impl<T: SubFeature> IntoSubFeature<()> for T {
    type SubFeature = Self;
    #[inline]
    fn into_sub_feature(self) -> Self {
        self
    }
}

//implementation time!!!!!

struct SubFeatureNode<G: RenderSubGraph, F: Feature<G>, S: SubFeature> {
    sub_feature: Mutex<S>, //todo: better storage?
    data: PhantomData<(G, F)>,
}

impl<G: RenderSubGraph, F: Feature<G>, S: SubFeature> Node for SubFeatureNode<G, F, S> {
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

pub struct FeatureDependencyBuilder<'w, G: RenderSubGraph, F: Feature<G>> {
    app: &'w mut App,
    data: PhantomData<fn(G, F)>,
}

impl<'w, G: RenderSubGraph, F: Feature<G>> FeatureDependencyBuilder<'w, G, F> {
    pub fn with_dep<'a: 'w, O: Feature<G>>(
        &'a mut self,
    ) -> RenderHandles<'a, true, FeatureOutput<G, O>> {
        todo!()
    }

    pub fn map<'a, A: FeatureIO<false>, B: FeatureIO<false>>(
        &'a mut self,
        handles: RenderHandles<'a, false, A>,
        f: impl for<'_w> Fn(A::Item<'_w>) -> B,
    ) -> RenderHandle<'a, B> {
        todo!()
    }

    pub fn map_many<'a, A: FeatureIO<true>, B: FeatureIO<false>>(
        &'a mut self,
        handles: RenderHandles<'a, true, A>,
        f: impl for<'_w> Fn(A::Item<'_w>) -> B,
    ) -> RenderHandle<'a, B> {
        self.app
            .world
            .init_component_with_descriptor(ComponentDescriptor::new::<FeatureComponent<B>>());
        //register system to map from input to output;
        todo!()
    }
}

#[macro_export]
macro_rules! IOHandles_Impl {
    ($($h: tt),*) => {
        ($($crate::render_feature::SingleHandle!($h)),*)
    }
}

pub use IOHandles_Impl as Handles;

#[macro_export]
macro_rules! SingleIOHandle_Impl {
    (_) => {
        $crate::render_feature::RenderHandle::hole()
    };
    ($h: expr) => {
        $h
    };
}

pub use SingleIOHandle_Impl as SingleHandle;

//SAFETY: this must stay repr(transparent) to make sure it has the same layout as A
#[repr(transparent)]
struct FeatureComponent<A: FeatureIO<false>>(A);

impl<A: FeatureIO<false>> Component for FeatureComponent<A> {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}
