pub mod dependencies;
mod function_feature;

pub use function_feature::*;

use std::any::Any;
use std::marker::PhantomData;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{SystemParam, SystemParamItem};
use bevy_render::render_graph::{RenderLabel, RenderSubGraph};
use bevy_render::render_resource::{WgpuFeatures, WgpuLimits};
use bevy_utils::all_tuples;

//todo: mutable param access in the thingamabob

pub trait Feature<G: RenderSubGraph>: Sized + 'static {
    type CompatibilityKey;
    type Sig: FeatureSignature;

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
        inputs: IOHandle<'b, FeatureInput<G, Self>>,
    ) -> IOHandle<'b, FeatureOutput<G, Self>>;
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

pub struct IOHandle<'a, T: FeatureIO> {
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
        input: IOHandle<'b, SubFeatureInput<S::SubFeature>>,
        sub_feature: S,
    ) -> IOHandle<'b, SubFeatureOutput<S::SubFeature>> {
        let (_, _, _) = (stage, input, sub_feature);
        todo!()
    }
}

pub trait FeatureIO: Any + Send + Sync + 'static {}

impl<A: Any + Send + Sync + 'static> FeatureIO for A {}

pub trait IOHandleTuple {
    type Tupled;

    fn tupled(self) -> Self::Tupled;
    fn untupled(tuple: Self::Tupled) -> Self;
}

macro_rules! impl_handle_tuple {
    ($($T: ident),*) => {
        impl <'b, $($T: FeatureIO),*> IOHandleTuple for IOHandle<'b, ($($T,)*)> {
            type Tupled = ($(IOHandle<'b, $T>,)*);
            fn untupled(_tuple: Self::Tupled) -> Self {
                todo!()
            }
            fn tupled(self) -> Self::Tupled {
                todo!()
            }
        }
    };
}

all_tuples!(impl_handle_tuple, 1, 16, T);

/*macro_rules! impl_as_io_handles {
    ($($T: ident),*) => {
        impl<$($T: Send + Sync + 'static),*> FeatureIO for ($($T,)*) //have to impl for 1-tuple
        //rather than raw type because of the evil
        {
            #[allow(unused_parens)]
            type AsIOHandles<'a> = ($(IOHandle<'a, $T>,)*);

            #[inline]
            fn as_type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$T>()),*]
            }
        }
    };
}

all_tuples!(impl_as_io_handles, 1, 32, T);*/

pub trait FeatureSignature: 'static {
    type In: FeatureIO;
    type Out: FeatureIO;
}

pub struct FeatureSigData<I: FeatureIO, O: FeatureIO>(PhantomData<(I, O)>);

impl<I: FeatureIO, O: FeatureIO> FeatureSignature for FeatureSigData<I, O> {
    type In = I;
    type Out = O;
}

impl FeatureSignature for () {
    type In = ();
    type Out = ();
}

#[macro_export]
macro_rules! FeatureSig_Macro {
    [$i: ty => $o: ty] => {
        $crate::render_feature::FeatureSigData<$i, $o>
    };
}

pub use FeatureSig_Macro as FeatureSig;

use self::dependencies::{empty, FeatureDependencies};

//type SubFeatureSig<G, F, S> = <S as RenderFeatureStageMarker>::SubFeatureSig<G, F>;
type FeatureInput<G, F> = <<F as Feature<G>>::Sig as FeatureSignature>::In;
type FeatureOutput<G, F> = <<F as Feature<G>>::Sig as FeatureSignature>::Out;
type SubFeatureInput<F> = <<F as SubFeature>::Sig as FeatureSignature>::In;
type SubFeatureOutput<F> = <<F as SubFeature>::Sig as FeatureSignature>::Out;

pub trait SubFeature: 'static {
    //type Stage: RenderFeatureStageMarker;
    type Sig: FeatureSignature;
    type Param: SystemParam;

    fn run(
        &mut self,
        view_entity: Entity,
        input: SubFeatureInput<Self>,
        param: &mut SystemParamItem<Self::Param>,
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

    type Sig = FeatureSig![(u8, u32, u32) => u32];

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
        (empty(), empty(), empty())
    }

    fn build_feature<'b>(
        &self,
        _compatibility: Self::CompatibilityKey,
        builder: &mut FeatureBuilder<'b, G, Self>,
        input: IOHandle<'b, FeatureInput<G, Self>>,
    ) -> IOHandle<'b, FeatureOutput<G, Self>> {
        let (_, u2, _) = input.tupled();
        let thing = builder.add_sub_feature(FeatureStage::Extract, u2, |_, c| c + 4);
        thing
    }
}
