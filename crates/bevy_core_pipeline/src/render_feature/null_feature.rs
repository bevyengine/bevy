use bevy_ecs::prelude::World;
use bevy_ecs::{entity::Entity, system::SystemParamItem};
use bevy_render::render_graph::RenderSubGraph;
use std::marker::PhantomData;

use crate::render_feature::{
    FeatureInput, FeatureOutput, RenderComponent, RenderFeatureDependencies,
    RenderFeatureStageMarker, RenderSubFeature,
};

use super::FeatureSig;

pub struct NullFeature<S: RenderFeatureStageMarker, O: RenderComponent = ()>(PhantomData<(S, O)>);

impl<S: RenderFeatureStageMarker, O: RenderComponent> Default for NullFeature<S, O> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<G, S, O> RenderSubFeature<G> for NullFeature<S, O>
where
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    O: RenderComponent,
{
    type Stage = S;
    type Sig = FeatureSig![() => O];
    type Param = ();

    fn enabled(&self) -> bool {
        false
    }

    fn default_dependencies(
        &self,
    ) -> impl RenderFeatureDependencies<G, Self::Stage, FeatureInput<G, Self>> {
    }

    fn run(
        &self,
        _view_entity: Entity,
        _input: FeatureInput<G, Self>,
        _param: SystemParamItem<Self::Param>,
    ) -> FeatureOutput<G, Self> {
        unreachable!()
    }
}
