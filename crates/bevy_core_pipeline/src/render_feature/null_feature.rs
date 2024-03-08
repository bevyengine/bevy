use bevy_render::render_graph::RenderSubGraph;
use std::marker::PhantomData;

use crate::render_feature::{
    RenderComponent, RenderFeatureDependencies, RenderFeatureStageMarker, RenderSubFeature,
};

#[derive(Default)]
pub struct NullFeature<S: RenderFeatureStageMarker, O: RenderComponent = ()>(PhantomData<(S, O)>);

impl<G, S, O> RenderSubFeature<G> for NullFeature<S, O>
where
    G: RenderSubGraph,
    S: RenderFeatureStageMarker,
    O: RenderComponent,
{
    type Stage = S;
    type In = ();
    type Out = O;
    type Param = ();

    fn enabled(&self) -> bool {
        false
    }

    fn default_dependencies(
        &self,
    ) -> &(impl RenderFeatureDependencies<G, Self::Stage, Self::In> + ?Sized) {
        &()
    }

    fn run(&self, _deps: Self::In) -> Self::Out {
        unreachable!()
    }
}
