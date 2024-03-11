use std::marker::PhantomData;

use crate::core_3d::graph::SubGraph3d;
use bevy_ecs::{
    entity::Entity,
    system::{SystemParam, SystemParamItem},
    world::World,
};
use bevy_render::render_graph::RenderSubGraph;

use crate::render_feature::{
    stages, FeatureSig, RenderFeatureDependencies, RenderFeatureStageMarker, RenderSubFeature,
};

use super::RenderFeatureSignature;

pub trait SimpleFeatureFunction<
    Stage: RenderFeatureStageMarker,
    Sig: RenderFeatureSignature,
    P: SystemParam + 'static,
>: Fn(Entity, Sig::In, SystemParamItem<P>) -> Sig::Out + Send + Sync + 'static
{
}

impl<
        Stage: RenderFeatureStageMarker,
        Sig: RenderFeatureSignature,
        P: SystemParam + 'static,
        F: Fn(Entity, Sig::In, SystemParamItem<P>) -> Sig::Out + Send + Sync + 'static,
    > SimpleFeatureFunction<Stage, Sig, P> for F
{
}

pub struct SimpleFeature<G, Stage, Sig, P, F>
where
    G: RenderSubGraph,
    Stage: RenderFeatureStageMarker,
    Sig: RenderFeatureSignature,
    P: SystemParam + 'static,
    F: SimpleFeatureFunction<Stage, Sig, P>,
{
    deps: Box<dyn RenderFeatureDependencies<G, Stage, Sig::In>>,
    run: F,
    data: PhantomData<fn(P)>,
}

impl<G, Stage, Sig, P, F> SimpleFeature<G, Stage, Sig, P, F>
where
    G: RenderSubGraph,
    Stage: RenderFeatureStageMarker,
    Sig: RenderFeatureSignature,
    P: SystemParam + 'static,
    F: SimpleFeatureFunction<Stage, Sig, P>,
{
    pub fn new(deps: impl RenderFeatureDependencies<G, Stage, Sig::In> + 'static, run: F) -> Self {
        Self {
            deps: Box::new(deps),
            run,
            data: PhantomData,
        }
    }
}

impl<G, Stage, Sig, P, F> RenderSubFeature<G> for SimpleFeature<G, Stage, Sig, P, F>
where
    G: RenderSubGraph,
    Stage: RenderFeatureStageMarker,
    Sig: RenderFeatureSignature,
    P: SystemParam + 'static,
    F: SimpleFeatureFunction<Stage, Sig, P>,
{
    type Stage = Stage;
    type Sig = Sig;
    type Param = P;

    fn default_dependencies(&self) -> impl RenderFeatureDependencies<G, Self::Stage, Sig::In> {
        &*self.deps
    }

    fn run(
        &self,
        view_entity: Entity,
        input: super::FeatureInput<G, Self>,
        param: SystemParamItem<Self::Param>,
    ) -> super::FeatureOutput<G, Self> {
        (self.run)(view_entity, input, param)
    }
}
