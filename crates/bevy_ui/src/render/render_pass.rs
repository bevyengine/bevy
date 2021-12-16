use bevy_core::FloatOrd;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_render::{
    camera::ExtractedCameraNames,
    render_graph::*,
    render_phase::*,
    render_resource::{
        CachedPipelineId, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor,
    },
    renderer::*,
    view::*,
};

use super::{draw_ui_graph, UiBatch, UiImageBindGroups, UiMeta, CAMERA_UI};

pub struct UiPassDriverNode;

impl bevy_render::render_graph::Node for UiPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        if let Some(camera_ui) = extracted_cameras.entities.get(CAMERA_UI) {
            graph.run_sub_graph(draw_ui_graph::NAME, vec![SlotValue::Entity(*camera_ui)])?;
        }

        Ok(())
    }
}

pub struct UiPassNode {
    query:
        QueryState<(&'static RenderPhase<TransparentUi>, &'static ViewTarget), With<ExtractedView>>,
}

impl UiPassNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl bevy_render::render_graph::Node for UiPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(UiPassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (transparent_phase, target) = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");
        let pass_descriptor = RenderPassDescriptor {
            label: Some("ui_pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: &target.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        let draw_functions = world
            .get_resource::<DrawFunctions<TransparentUi>>()
            .unwrap();

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        let mut draw_functions = draw_functions.write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in transparent_phase.items.iter() {
            let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
            draw_function.draw(world, &mut tracked_pass, view_entity, item);
        }
        Ok(())
    }
}

pub struct TransparentUi {
    pub sort_key: FloatOrd,
    pub entity: Entity,
    pub pipeline: CachedPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for TransparentUi {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for TransparentUi {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedPipelinePhaseItem for TransparentUi {
    #[inline]
    fn cached_pipeline(&self) -> CachedPipelineId {
        self.pipeline
    }
}

pub type DrawUi = (
    SetItemPipeline,
    SetUiViewBindGroup<0>,
    SetUiTextureBindGroup<1>,
    DrawUiNode,
);

pub struct SetUiViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetUiViewBindGroup<I> {
    type Param = (SRes<UiMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (ui_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap(); // TODO: store bind group as component?
        pass.set_bind_group(
            I,
            ui_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}
pub struct SetUiTextureBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetUiTextureBindGroup<I> {
    type Param = (SRes<UiImageBindGroups>, SQuery<Read<UiBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batch = query_batch.get(item).unwrap();
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(1, image_bind_groups.values.get(&batch.image).unwrap(), &[]);
        RenderCommandResult::Success
    }
}
pub struct DrawUiNode;
impl EntityRenderCommand for DrawUiNode {
    type Param = (SRes<UiMeta>, SQuery<Read<UiBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (ui_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batch = query_batch.get(item).unwrap();

        pass.set_vertex_buffer(0, ui_meta.into_inner().vertices.buffer().unwrap().slice(..));
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}
