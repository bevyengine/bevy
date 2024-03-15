pub mod dependencies;
mod function_feature;

use bevy_ecs::component::{Component, TableStorage};
use bevy_ecs::world::World;
use bevy_render::renderer::RenderContext;
pub use function_feature::*;

use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::Mutex;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{SystemParam, SystemParamItem};
use bevy_render::render_graph::{
    Node, NodeRunError, RenderGraphContext, RenderLabel, RenderSubGraph,
};
use bevy_render::render_resource::{WgpuFeatures, WgpuLimits};
use bevy_utils::all_tuples;

pub trait Feature<G: RenderSubGraph>: Sized + Send + Sync + 'static {
    type CompatibilityKey;
    type Sig: FeatureSignature<true>;

    fn check_compatibility(
        &self,
        features: WgpuFeatures,
        limits: WgpuLimits,
    ) -> Self::CompatibilityKey;

    fn dependencies(
        &self,
        compatibility: Self::CompatibilityKey,
    ) -> impl FeatureDependencies<G, FeatureInput<G, Self>>;

    fn build(&self, _compatibility: Self::CompatibilityKey, _app: &mut App) {} // for adding systems not associated with the main View

    fn build_feature<'b>(
        &self,
        compatibility: Self::CompatibilityKey,
        builder: &mut FeatureBuilder<'b, G, Self>,
        inputs: IOHandles<'b, true, FeatureInput<G, Self>>,
    ) -> IOHandles<'b, true, FeatureOutput<G, Self>>;
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

pub struct FeatureBuilder<'b, G: RenderSubGraph, F: Feature<G>> {
    data: PhantomData<&'b (G, F)>,
}

pub struct IOHandle<'a, T: Send + Sync + 'static> {
    data: PhantomData<&'a T>,
}

impl<'b> Default for IOHandle<'b, ()> {
    fn default() -> Self {
        Self { data: PhantomData }
    }
}

impl<'b, G: RenderSubGraph, F: Feature<G>> FeatureBuilder<'b, G, F> {
    pub fn add_sub_feature<Marker: 'static, S: IntoSubFeature<Marker>>(
        &mut self,
        stage: FeatureStage,
        input: IOHandles<'b, true, SubFeatureInput<S::SubFeature>>,
        sub_feature: S,
    ) -> IOHandles<'b, false, SubFeatureOutput<S::SubFeature>> {
        let (_, _, _) = (stage, input, sub_feature);
        todo!()
    }
}

type IOHandles<'b, const MULT: bool, F> = <F as FeatureIO<MULT>>::Handles<'b>;

pub trait FeatureIO<const MULT: bool>: Send + Sync + 'static {
    type Handles<'b>;

    fn type_ids() -> Vec<TypeId>;
}

impl<A: Send + Sync + 'static> FeatureIO<false> for A {
    type Handles<'b> = IOHandle<'b, A>;

    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<Self>()]
    }
}

macro_rules! impl_multi_feature_io {
    ($($T: ident),*) => {
        impl <$($T: FeatureIO<false>),*> FeatureIO<true> for ($($T,)*) {
            type Handles<'b> = ($(IOHandle<'b, $T>,)*);

            fn type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$T>()),*]
            }
        }
    };
}

all_tuples!(impl_multi_feature_io, 0, 16, T);

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

use self::dependencies::{hole, FeatureDependencies};

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

pub struct Blit<L: RenderLabel>(PhantomData<L>);

impl<L: RenderLabel> Default for Blit<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<G: RenderSubGraph, L: RenderLabel> Feature<G> for Blit<L> {
    type CompatibilityKey = Compatibility;

    type Sig = Sig![(u8, u32, u32) => (u32, u32)];

    fn check_compatibility(
        &self,
        _features: WgpuFeatures,
        _limits: WgpuLimits,
    ) -> Self::CompatibilityKey {
        Compatibility::Full
    }

    fn dependencies(
        &self,
        _compatibility: Self::CompatibilityKey,
    ) -> impl FeatureDependencies<G, FeatureInput<G, Self>> {
        (hole(), hole(), hole()) //Deps!(_, _, _)
    }

    fn build_feature<'b>(
        &self,
        _compatibility: Self::CompatibilityKey,
        builder: &mut FeatureBuilder<'b, G, Self>,
        (_, b, c): IOHandles<'b, true, FeatureInput<G, Self>>,
    ) -> IOHandles<'b, true, FeatureOutput<G, Self>> {
        let thing = builder.add_sub_feature(FeatureStage::Extract, (b,), |_, (c,)| c + 4);
        let thing2 = builder.add_sub_feature(FeatureStage::Dispatch, (c,), |_, (c,)| c + 4);
        (thing, thing2)
    }
}

//implementation time!!!!!

struct FeatureResult<G: RenderSubGraph, F: Feature<G>, S: SubFeature> {
    value: SubFeatureOutput<S>,
    data: PhantomData<(F, G)>,
}

impl<G: RenderSubGraph, F: Feature<G>, S: SubFeature> Component for FeatureResult<G, F, S> {
    type Storage = TableStorage;
}

struct SubFeatureNode<G: RenderSubGraph, F: Feature<G>, S: SubFeature> {
    sub_feature: Mutex<S>,
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
