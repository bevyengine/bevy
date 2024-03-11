use std::marker::PhantomData;

use bevy_app::App;
use bevy_ecs::component::{Component, ComponentStorage, SparseStorage};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{QueryState, With};
use bevy_ecs::system::{SystemParam, SystemParamItem};
use bevy_ecs::world::{FromWorld, World};
use bevy_render::render_graph::{
    Node, NodeRunError, RenderGraphContext, RenderGraphError, RenderSubGraph,
};
pub use null_feature::*;

mod simple_feature;
pub use simple_feature::*;
mod dependencies;
mod stages;
use crate::render_feature::stages::NotAfter;
pub use dependencies::*;
use stages::RenderFeatureStageMarker;
mod null_feature;

//todo: WGPU limits, etc,
pub trait RenderFeature<G: RenderSubGraph>: 'static {
    //todo: type conflicts when selecting features for dependencies? must be clear that type
    //equality is used to decide dependencies
    type Extract: RenderFeatureSignature<In = ()>;
    type SpecializePipelines: RenderFeatureSignature;
    type PrepareResources: RenderFeatureSignature;
    type PrepareBindGroups: RenderFeatureSignature;
    type Dispatch: RenderFeatureSignature<Out = Result<(), RenderGraphError>>;

    fn build(&self, _app: &mut App) {} // for adding systems not associated with the main View

    fn additional_sub_features(&self) -> impl RenderSubFeatures<G, stages::Dispatch>;

    fn extract(&self) -> impl RenderSubFeature<G, Stage = stages::Extract, Sig = Self::Extract>;

    fn specialize_pipelines(
        &self,
    ) -> impl RenderSubFeature<G, Stage = stages::SpecializePipelines, Sig = Self::SpecializePipelines>;

    //todo: this wouldn't allow parallelizing resource creation with separate systems
    fn prepare_resources(
        &self,
    ) -> impl RenderSubFeature<G, Stage = stages::PrepareResources, Sig = Self::PrepareResources>;

    fn prepare_bind_groups(
        &self,
    ) -> impl RenderSubFeature<G, Stage = stages::PrepareBindGroups, Sig = Self::PrepareBindGroups>;

    fn dispatch(&self) -> impl RenderSubFeature<G, Stage = stages::Dispatch, Sig = Self::Dispatch>;
}

pub trait RenderFeatureSignature: 'static {
    type In;
    type Out: RenderComponent;
}

impl<I: 'static, O: RenderComponent + 'static> RenderFeatureSignature for (I, O) {
    type In = I;
    type Out = O;
}

#[macro_export]
macro_rules! FeatureSig_Macro {
    [$i: ty => $o: ty] => {
        ($i, $o)
    };
}

pub use FeatureSig_Macro as FeatureSig;

type SubFeatureSig<G, F, S> = <S as RenderFeatureStageMarker>::SubFeatureSig<G, F>;

type FeatureInput<G, F> = <<F as RenderSubFeature<G>>::Sig as RenderFeatureSignature>::In;
type FeatureOutput<G, F> = <<F as RenderSubFeature<G>>::Sig as RenderFeatureSignature>::Out;

pub trait RenderSubFeature<G: RenderSubGraph>: 'static {
    type Stage: RenderFeatureStageMarker;
    type Sig: RenderFeatureSignature;
    type Param: SystemParam;

    //mainly for the purpose of disabling stages completely
    fn enabled(&self) -> bool {
        true
    }

    fn default_dependencies(
        &self,
    ) -> impl dependencies::RenderFeatureDependencies<G, Self::Stage, FeatureInput<G, Self>>;

    fn run(
        &self,
        view_entity: Entity,
        input: FeatureInput<G, Self>,
        param: SystemParamItem<Self::Param>,
    ) -> FeatureOutput<G, Self>;
}

pub trait IntoRenderSubFeatureConfigs<G: RenderSubGraph, F: RenderSubFeature<G>> {
    fn override_dep<In, D: RenderSubFeature<G>>(&mut self)
    where
        D::Stage: NotAfter<F::Stage>,
        D::Sig: RenderFeatureSignature<Out = In>;

    fn override_dep_with<In, D: RenderSubFeatures<G, F::Stage>, A: Fn(D::Out) -> In>(
        &mut self,
        adapter: A,
    );

    fn after(&mut self, features: impl RenderSubFeatures<G, F::Stage>);
}

/*pub struct RenderFeatureConfigs<G: RenderSubGraph, F: RenderFeature<G>> {
    extract: RenderSubFeatureConfigs<G, F::Extract>,
    specialize_pipelines: RenderSubFeatureConfigs<G, F::SpecializePipelines>,
    prepare_resources: RenderSubFeatureConfigs<G, F::PrepareResources>,
    prepare_bind_groups: RenderSubFeatureConfigs<G, F::PrepareBindGroups>,
    dispatch: RenderSubFeatureConfigs<G, F::Dispatch>,
    data: PhantomData<(G, F)>,
}

impl<G: RenderSubGraph, F: RenderFeature<G>> RenderFeatureConfigs<G, F> {
    pub fn with_extract(
        &mut self,
        fun: impl FnOnce(&mut RenderSubFeatureConfigs<G, F::Extract>),
    ) -> &mut Self {
        fun(&mut self.extract);
        self
    }

    pub fn with_specialize_pipelines(
        &mut self,
        fun: impl FnOnce(&mut RenderSubFeatureConfigs<G, F::SpecializePipelines>),
    ) -> &mut Self {
        fun(&mut self.specialize_pipelines);
        self
    }

    pub fn with_prepare_resources(
        &mut self,
        fun: impl FnOnce(&mut RenderSubFeatureConfigs<G, F::PrepareResources>),
    ) -> &mut Self {
        fun(&mut self.prepare_resources);
        self
    }

    pub fn with_prepare_bind_groups(
        &mut self,
        fun: impl FnOnce(&mut RenderSubFeatureConfigs<G, F::PrepareBindGroups>),
    ) -> &mut Self {
        fun(&mut self.prepare_bind_groups);
        self
    }

    pub fn with_dispatch(
        &mut self,
        fun: impl FnOnce(&mut RenderSubFeatureConfigs<G, F::Dispatch>),
    ) -> &mut Self {
        fun(&mut self.dispatch);
        self
    }
}

pub struct RenderSubFeatureConfigs<G: RenderSubGraph, F: RenderSubFeature<G>> {
    sub_feature: F,
    data: PhantomData<G>,
}

impl<G: RenderSubGraph, F: RenderSubFeature<G>> RenderSubFeatureConfigs<G, F> {
    pub fn after<D: RenderSubFeatures<G, F::Stage>>(&mut self) -> &mut Self {
        todo!()
    }
}*/

//todo: add derive
pub trait RenderComponent: Send + Sync + 'static {
    type Storage: ComponentStorage;
}

impl RenderComponent for () {
    type Storage = SparseStorage;
}

struct RenderFeatureResult<G: RenderSubGraph, F: RenderFeature<G>, S: RenderFeatureStageMarker> {
    value: <SubFeatureSig<G, F, S> as RenderFeatureSignature>::Out,
}

impl<G: RenderSubGraph, F: RenderFeature<G>, S: RenderFeatureStageMarker> Component
    for RenderFeatureResult<G, F, S>
{
    type Storage =
        <<SubFeatureSig<G, F, S> as RenderFeatureSignature>::Out as RenderComponent>::Storage;
}

//dependency stuff
pub trait RenderSubFeatures<G: RenderSubGraph, S: RenderFeatureStageMarker> {
    type Out;
}
