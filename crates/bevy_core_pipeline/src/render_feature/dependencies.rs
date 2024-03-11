use crate::render_feature::{
    NotAfter, RenderFeatureSignature, RenderFeatureStageMarker, RenderSubFeature, RenderSubFeatures,
};
use bevy_render::render_graph::RenderSubGraph;
use bevy_utils::all_tuples_with_size;
use std::marker::PhantomData;

pub trait RenderFeatureDependency<G: RenderSubGraph, S: RenderFeatureStageMarker, I> {}
pub trait RenderFeatureDependencies<G: RenderSubGraph, S: RenderFeatureStageMarker, I> {}

macro_rules! impl_render_sub_features { //todo: defines instance for 1-tuple rather than raw value?
    ($N: expr, $($F: ident),*) => {
        impl<G: RenderSubGraph, S: RenderFeatureStageMarker, $($F: RenderSubFeature<G>),*> RenderSubFeatures<G, S> for ($($F,)*)
        where
            $(<$F as RenderSubFeature<G>>::Stage: NotAfter<S>),*
        {
            type Out = ($(super::FeatureOutput<G, $F>,)*);
        }
    };
}

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

impl<F: RenderSubFeature<G>, G: RenderSubGraph, S: RenderFeatureStageMarker, I>
    RenderFeatureDependency<G, S, I> for PassDependency<F>
where
    F::Sig: RenderFeatureSignature<Out = I>,
    F::Stage: NotAfter<S>,
{
}

impl<G: RenderSubGraph, S: RenderFeatureStageMarker, I, F: RenderSubFeatures<G, S, Out = I>>
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
    A: Fn(F::Out) -> I,
{
}

impl<I, F, G, S, A> RenderFeatureDependencies<G, S, I> for DependencyAdapter<A, F>
where
    F: RenderSubFeatures<G, S>,
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    A: Fn(F::Out) -> I,
{
}

// a "hole" or unfilled dependency. If left unfilled, will panic! at .build() time.
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

//todo: probably not great, a patch for the implementation of SimpleFeature::default_dependencies()
impl<G: RenderSubGraph, S: RenderFeatureStageMarker, I> RenderFeatureDependencies<G, S, I>
    for &dyn RenderFeatureDependencies<G, S, I>
{
}
