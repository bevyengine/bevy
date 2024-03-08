use std::marker::PhantomData;

use bevy_app::App;
use bevy_ecs::component::{Component, ComponentStorage, SparseStorage};
use bevy_ecs::system::SystemParam;
use bevy_render::render_graph::{RenderGraphError, RenderSubGraph};
use bevy_utils::all_tuples_with_size;
pub use null_feature::*;

use crate::core_3d::graph::SubGraph3d;

mod null_feature;
mod simple_feature;
pub use simple_feature::*;

pub trait RenderFeature<G: RenderSubGraph>: 'static {
    //todo: type conflicts when selecting features for dependencies? must be clear that type
    //equality is used to decide dependencies
    type SpecializePipelines: RenderSubFeature<G, Stage = stages::SpecializePipelines>;
    type PrepareResources: RenderSubFeature<G, Stage = stages::PrepareResources>;
    type PrepareBindGroups: RenderSubFeature<G, Stage = stages::PrepareBindGroups>;
    type Dispatch: RenderSubFeature<G, Stage = stages::Dispatch, Out = Result<(), RenderGraphError>>;

    fn build(&self, app: &mut App) {}

    fn additional_sub_features(&self) -> impl RenderSubFeatures<G, stages::Dispatch>;

    fn specialize_pipelines(&self) -> Self::SpecializePipelines;

    fn prepare_resources(&self) -> Self::PrepareResources;

    fn prepare_bind_groups(&self) -> Self::PrepareBindGroups;

    fn dispatch(&self) -> Self::Dispatch;
}

type GetFeatureStage<G, F, S> = <S as RenderFeatureStageMarker>::FeatureSubFeature<G, F>;

pub trait RenderSubFeature<G: RenderSubGraph>: 'static {
    type Stage: RenderFeatureStageMarker + ?Sized;
    type In;
    type Out: RenderComponent;
    type Param: SystemParam;

    fn enabled(&self) -> bool {
        true
    } //mainly for the purpose of disabling stages completely

    fn default_dependencies(
        &self,
    ) -> &(impl RenderFeatureDependencies<G, Self::Stage, Self::In> + ?Sized);

    fn run(&self, deps: Self::In) -> Self::Out;
}

//todo: add derive
pub trait RenderComponent: Send + Sync + 'static {
    type Storage: ComponentStorage;
}

impl RenderComponent for () {
    type Storage = SparseStorage;
}

struct RenderFeatureResult<G: RenderSubGraph, F: RenderFeature<G>, S: RenderFeatureStageMarker> {
    value: <<S as RenderFeatureStageMarker>::FeatureSubFeature<G, F> as RenderSubFeature<G>>::Out,
}

impl<G: RenderSubGraph, F: RenderFeature<G>, S: RenderFeatureStageMarker> Component
    for RenderFeatureResult<G, F, S>
{
    type Storage =
        <<GetFeatureStage<G, F, S> as RenderSubFeature<G>>::Out as RenderComponent>::Storage;
}

//stages stuff

pub trait RenderFeatureStageMarker: 'static {
    const STAGE: RenderFeatureStage;

    type FeatureSubFeature<G: RenderSubGraph, F: RenderFeature<G>>: RenderSubFeature<
        G,
        Stage = Self,
    >;
}

pub trait NotAfter<O: RenderFeatureStageMarker>: RenderFeatureStageMarker {}

pub enum RenderFeatureStage {
    SpecializePipelines,
    PrepareResources,
    PrepareBindGroups,
    Dispatch,
}

pub mod stages {
    use bevy_render::render_graph::RenderSubGraph;

    use super::{NotAfter, RenderFeature, RenderFeatureStage, RenderFeatureStageMarker};

    pub struct SpecializePipelines;

    pub struct PrepareResources;

    pub struct PrepareBindGroups;

    pub struct Dispatch;

    macro_rules! impl_not_after {
        ($T: ident) => {
            impl NotAfter<$T> for $T {}
        };
        ($T:ident, $S1:ident) => {
            impl NotAfter<$T> for $T {}
            impl NotAfter<$S1> for $T {}
        };
        ($T:ident, $S1:ident, $($SN:ident),+) => {
            impl NotAfter<$S1> for $T {}
            impl_not_after!($T, $($SN),+);
        };
    }

    impl RenderFeatureStageMarker for SpecializePipelines {
        const STAGE: RenderFeatureStage = RenderFeatureStage::SpecializePipelines;

        type FeatureSubFeature<G: RenderSubGraph, F: RenderFeature<G>> =
            <F as RenderFeature<G>>::SpecializePipelines;
    }

    impl_not_after!(
        SpecializePipelines,
        PrepareResources,
        PrepareBindGroups,
        Dispatch
    );

    impl RenderFeatureStageMarker for PrepareResources {
        const STAGE: RenderFeatureStage = RenderFeatureStage::PrepareResources;

        type FeatureSubFeature<G: RenderSubGraph, F: RenderFeature<G>> =
            <F as RenderFeature<G>>::PrepareResources;
    }

    impl_not_after!(PrepareResources, PrepareBindGroups, Dispatch);

    impl RenderFeatureStageMarker for PrepareBindGroups {
        const STAGE: RenderFeatureStage = RenderFeatureStage::PrepareBindGroups;

        type FeatureSubFeature<G: RenderSubGraph, F: RenderFeature<G>> =
            <F as RenderFeature<G>>::PrepareBindGroups;
    }

    impl_not_after!(PrepareBindGroups, Dispatch);

    impl RenderFeatureStageMarker for Dispatch {
        const STAGE: RenderFeatureStage = RenderFeatureStage::Dispatch;

        type FeatureSubFeature<G: RenderSubGraph, F: RenderFeature<G>> =
            <F as RenderFeature<G>>::Dispatch;
    }

    impl_not_after!(Dispatch);
}

//dependency stuff

pub trait RenderSubFeatures<G: RenderSubGraph, S: RenderFeatureStageMarker> {
    type Out;
}

pub trait RenderFeatureDependency<G: RenderSubGraph, S: RenderFeatureStageMarker + ?Sized, I> {}
pub trait RenderFeatureDependencies<G: RenderSubGraph, S: RenderFeatureStageMarker + ?Sized, I> {}

macro_rules! impl_render_sub_features { //todo: defines instance for 1-tuple rather than raw value?
    ($N: expr, $($F: ident),*) => {
        impl<G: RenderSubGraph, S: RenderFeatureStageMarker, $($F: RenderSubFeature<G>),*> RenderSubFeatures<G, S> for ($($F,)*)
        where
            $(<$F as RenderSubFeature<G>>::Stage: NotAfter<S>),*
        {
            type Out = ($(<$F as RenderSubFeature<G>>::Out,)*);
        }
    };
}

/*impl<G: RenderSubGraph, S: RenderFeatureStageMarker, F: RenderSubFeature<G>> RenderSubFeatures<G, S>
    for F
where
    <F as RenderSubFeature<G>>::Stage: NotAfter<S>,
{
    type Out = <F as RenderSubFeature<G>>::Out;
}*/

all_tuples_with_size!(impl_render_sub_features, 1, 32, F);

macro_rules! impl_render_feature_dependencies {
    ($N: expr, $(($Dep: ident, $In: ident)),*) => {
        impl<G: RenderSubGraph, S: RenderFeatureStageMarker, $($Dep: RenderFeatureDependency<G, S, $In>),*, $($In),*> RenderFeatureDependencies<G, S, ($($In,)*)> for ($($Dep,)*) {}
    };
}

all_tuples_with_size!(impl_render_feature_dependencies, 1, 32, Dep, In);

impl<G: RenderSubGraph, S: RenderFeatureStageMarker> RenderFeatureDependencies<G, S, ()> for () {}

pub struct PassDependency<F>(PhantomData<fn(F) -> ()>);

pub fn pass<F>() -> PassDependency<F> {
    PassDependency(PhantomData)
}

impl<F: RenderSubFeature<G, Out = I>, G: RenderSubGraph, S: RenderFeatureStageMarker, I>
    RenderFeatureDependency<G, S, I> for PassDependency<F>
where
    <F as RenderSubFeature<G>>::Stage: NotAfter<S>,
{
}

impl<F: RenderSubFeatures<G, S, Out = I>, G: RenderSubGraph, S: RenderFeatureStageMarker, I>
    RenderFeatureDependencies<G, S, I> for PassDependency<F>
{
}

pub struct DependencyAdapter<A, F>(A, PhantomData<fn(F) -> ()>);

pub fn adapt<A, F>(adapter: A) -> DependencyAdapter<A, F> {
    DependencyAdapter(adapter, PhantomData)
}

impl<I, F, G, S, A> RenderFeatureDependency<G, S, I> for DependencyAdapter<A, F>
where
    F: RenderSubFeatures<G, S>,
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    A: Fn(<F as RenderSubFeatures<G, S>>::Out) -> I,
{
}

impl<I, F, G, S, A> RenderFeatureDependencies<G, S, I> for DependencyAdapter<A, F>
where
    F: RenderSubFeatures<G, S>,
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    A: Fn(<F as RenderSubFeatures<G, S>>::Out) -> I,
{
}

// a "hole" or unfilled dependency. If left unfilled, will panic! at build time.
pub struct EmptyDependency;

pub fn empty() -> EmptyDependency {
    EmptyDependency
}

impl<G: RenderSubGraph, S: RenderFeatureStageMarker, I> RenderFeatureDependency<G, S, I>
    for EmptyDependency
{
}
impl<G: RenderSubGraph, S: RenderFeatureStageMarker, I> RenderFeatureDependencies<G, S, I>
    for EmptyDependency
{
}

pub fn thing() -> impl RenderFeatureDependencies<SubGraph3d, stages::PrepareBindGroups, (u128, u64)>
{
    adapt::<
        _,
        (
            NullFeature<stages::SpecializePipelines>,
            NullFeature<stages::PrepareResources>,
        ),
    >(|_| (3, 4))
}
