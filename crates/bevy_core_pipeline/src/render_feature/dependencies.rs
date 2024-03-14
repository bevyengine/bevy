use super::{Feature, FeatureIO};
use crate::render_feature::{FeatureSignature, SubFeature};
use bevy_render::render_graph::RenderSubGraph;
use bevy_utils::all_tuples_with_size;
use std::marker::PhantomData;

pub trait RenderFeatureDependency<G: RenderSubGraph, I: Send + Sync + 'static> {}
pub trait FeatureDependencies<G: RenderSubGraph, I: FeatureIO> {}

macro_rules! impl_feature_dependencies {
    ($N: expr, $(($Dep: ident, $In: ident)),*) => {
        #[allow(unused_parens)]
        impl<G: RenderSubGraph, $($Dep: RenderFeatureDependency<G, $In>,)* $($In: Send + Sync + 'static),*> FeatureDependencies<G, ($($In,)*)> for ($($Dep),*) {}
    };
}

all_tuples_with_size!(impl_feature_dependencies, 1, 32, Dep, In);

struct SelectType<F, T>(PhantomData<fn(F) -> T>);

macro_rules! SelectDeps {
    [$($F:ty as {$($S:ty),+}),+] => {
        ($($(SelectType<$F, $S>),+),+)
    }
}

trait SelectDependencies<G> {
    type Out;
}

macro_rules! impl_select_dependencies {
    ($N: expr, $(($F: ident, $I: ident)),*) => {
        #[allow(unused_parens)]
        impl<G: RenderSubGraph, $($F: Feature<G>,)* $($I),*> SelectDependencies<G> for ($(SelectType<$F, $I>),*) {
            #[allow(unused_parens)]
            type Out = ($($I),*);
        }
    };
}

all_tuples_with_size!(impl_select_dependencies, 1, 32, F, I);

impl<G: RenderSubGraph> FeatureDependencies<G, ()> for () {}

pub struct PassDependency<F>(PhantomData<fn(F) -> ()>);

pub fn pass<D>() -> PassDependency<D> {
    PassDependency(PhantomData)
}

impl<G: RenderSubGraph, D: SelectDependencies<G>> RenderFeatureDependency<G, D::Out>
    for PassDependency<D>
where
    D::Out: Send + Sync + 'static, //todo: where F::Out includes D::Out?
{
}

/*impl<G: RenderSubGraph, S: RenderFeatureStageMarker, I, F: RenderSubFeatures<G, S, Out = I>>
    RenderFeatureDependencies<G, S, I> for PassDependency<F>
{
}*/

pub struct DependencyAdapter<A, D>(A, PhantomData<fn(D) -> ()>);

pub fn adapt<A, F>(adapter: A) -> DependencyAdapter<A, F> {
    DependencyAdapter(adapter, PhantomData)
}

impl<G, A, F, I> RenderFeatureDependency<G, I> for DependencyAdapter<A, F>
where
    G: RenderSubGraph,
    A: Fn(F::Out) -> I,
    F: SelectDependencies<G>,
    I: Send + Sync + 'static,
{
}

/*impl<I, F, G, S, A> RenderFeatureDependencies<G, S, I> for DependencyAdapter<A, F>
where
    F: RenderSubFeatures<G, S>,
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    A: Fn(F::Out) -> I,
{
}*/

// a "hole" or unfilled dependency. If left unfilled, will panic! at .build() time.
pub struct EmptyDependency;

pub fn empty() -> EmptyDependency {
    EmptyDependency
}

impl<G: RenderSubGraph, I: Send + Sync + 'static> RenderFeatureDependency<G, I>
    for EmptyDependency
{
}
/*impl<G: RenderSubGraph, I> RenderFeatureDependencies<G, I> for EmptyDependency {}

//todo: probably not great, a patch for the implementation of SimpleFeature::default_dependencies()
impl<G: RenderSubGraph, I> RenderFeatureDependencies<G, I>
    for &dyn RenderFeatureDependencies<G, I>
{
}*/
