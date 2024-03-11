use crate::render_feature::{RenderFeature, RenderFeatureSignature};
use bevy_render::render_graph::RenderSubGraph;

pub trait RenderFeatureStageMarker: Send + Sync + 'static {
    const STAGE: RenderFeatureStage;

    type SubFeatureSig<G: RenderSubGraph, F: RenderFeature<G>>: RenderFeatureSignature;
}

pub trait NotAfter<O: RenderFeatureStageMarker>: RenderFeatureStageMarker {}

pub enum RenderFeatureStage {
    Extract,
    SpecializePipelines,
    PrepareResources,
    PrepareBindGroups,
    Dispatch,
}

pub struct Extract;
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

impl RenderFeatureStageMarker for Extract {
    const STAGE: RenderFeatureStage = RenderFeatureStage::Extract;

    type SubFeatureSig<G: RenderSubGraph, F: RenderFeature<G>> = <F as RenderFeature<G>>::Extract;
}

impl_not_after!(
    Extract,
    SpecializePipelines,
    PrepareResources,
    PrepareBindGroups,
    Dispatch
);

impl RenderFeatureStageMarker for SpecializePipelines {
    const STAGE: RenderFeatureStage = RenderFeatureStage::SpecializePipelines;

    type SubFeatureSig<G: RenderSubGraph, F: RenderFeature<G>> =
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

    type SubFeatureSig<G: RenderSubGraph, F: RenderFeature<G>> =
        <F as RenderFeature<G>>::PrepareResources;
}

impl_not_after!(PrepareResources, PrepareBindGroups, Dispatch);

impl RenderFeatureStageMarker for PrepareBindGroups {
    const STAGE: RenderFeatureStage = RenderFeatureStage::PrepareBindGroups;

    type SubFeatureSig<G: RenderSubGraph, F: RenderFeature<G>> =
        <F as RenderFeature<G>>::PrepareBindGroups;
}

impl_not_after!(PrepareBindGroups, Dispatch);

impl RenderFeatureStageMarker for Dispatch {
    const STAGE: RenderFeatureStage = RenderFeatureStage::Dispatch;

    type SubFeatureSig<G: RenderSubGraph, F: RenderFeature<G>> = <F as RenderFeature<G>>::Dispatch;
}

impl_not_after!(Dispatch);

