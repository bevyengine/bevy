use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_render::{
    extract_component::DynamicUniformIndex,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    view::{ViewTarget, ViewUniformOffset},
};

use crate::ViewLightsUniformOffset;

use super::{
    resources::{AtmosphereBindGroups, AtmosphereTransformsOffset, RenderSkyPipelineId},
    Atmosphere, AtmosphereSettings,
};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum AtmosphereNode {
    RenderLuts,
    RenderSky,
}

#[derive(Default)]
pub(super) struct AtmosphereLutsNode {}

impl ViewNode for AtmosphereLutsNode {
    type ViewQuery = (
        Read<AtmosphereSettings>,
        Read<AtmosphereBindGroups>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (
            _settings,
            _bind_groups,
            _atmosphere_uniforms_offset,
            _settings_uniforms_offset,
            _atmosphere_transforms_offset,
            _view_uniforms_offset,
            _lights_uniforms_offset,
        ): QueryItem<Self::ViewQuery>,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct RenderSkyNode;

impl ViewNode for RenderSkyNode {
    type ViewQuery = (
        Read<AtmosphereBindGroups>,
        Read<ViewTarget>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<RenderSkyPipelineId>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        (
            _atmosphere_bind_groups,
            _view_target,
            _atmosphere_uniforms_offset,
            _settings_uniforms_offset,
            _atmosphere_transforms_offset,
            _view_uniforms_offset,
            _lights_uniforms_offset,
            _render_sky_pipeline_id,
        ): QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}
