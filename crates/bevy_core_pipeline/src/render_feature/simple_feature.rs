use bevy_ecs::system::SystemParam;
use bevy_render::render_graph::RenderSubGraph;
use std::marker::PhantomData;

use crate::render_feature::{
    RenderComponent, RenderFeatureDependencies, RenderFeatureStageMarker, RenderSubFeature,
};

pub struct SimpleFeature<G, S, I, O, P, F>
where
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    I: 'static,
    O: RenderComponent,
    P: SystemParam + 'static,
    F: Fn(I) -> O + 'static,
{
    deps: Box<dyn RenderFeatureDependencies<G, S, I>>,
    fun: F,
    data: PhantomData<(fn(I) -> O, P)>,
}

impl<G, S, I, O, P, F> SimpleFeature<G, S, I, O, P, F>
where
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    I: 'static,
    O: RenderComponent,
    P: SystemParam + 'static,
    F: Fn(I) -> O + 'static,
{
    pub fn new(deps: impl RenderFeatureDependencies<G, S, I> + 'static, fun: F) -> Self {
        SimpleFeature {
            deps: Box::new(deps),
            fun,
            data: PhantomData,
        }
    }
}

impl<G, S, I, O, P, F> RenderSubFeature<G> for SimpleFeature<G, S, I, O, P, F>
where
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    I: 'static,
    O: RenderComponent,
    P: SystemParam + 'static,
    F: Fn(I) -> O + 'static,
{
    type Stage = S;
    type In = I;
    type Out = O;
    type Param = P;

    fn default_dependencies(
        &self,
    ) -> &(impl RenderFeatureDependencies<G, Self::Stage, Self::In> + ?Sized) {
        &*self.deps
    }

    fn run(&self, deps: Self::In) -> Self::Out {
        (self.fun)(deps)
    }
}
