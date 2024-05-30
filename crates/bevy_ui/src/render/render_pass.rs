use std::ops::Range;

use super::{UiBatch, UiImageBindGroups, UiMeta};
use crate::DefaultCameraView;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_math::FloatOrd;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::*,
    render_phase::*,
    render_resource::{CachedRenderPipelineId, RenderPassDescriptor},
    renderer::*,
    view::*,
};

pub struct UiPassNode {
    ui_view_query: QueryState<(&'static ViewTarget, &'static ExtractedCamera), With<ExtractedView>>,
    default_camera_view_query: QueryState<&'static DefaultCameraView>,
}

impl UiPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            ui_view_query: world.query_filtered(),
            default_camera_view_query: world.query(),
        }
    }
}

impl Node for UiPassNode {
    fn update(&mut self, world: &mut World) {
        self.ui_view_query.update_archetypes(world);
        self.default_camera_view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let input_view_entity = graph.view_entity();

        let Some(transparent_render_phases) =
            world.get_resource::<ViewSortedRenderPhases<TransparentUi>>()
        else {
            return Ok(());
        };

        let Some(transparent_phase) = transparent_render_phases.get(&input_view_entity) else {
            return Ok(());
        };

        let Ok((target, camera)) = self.ui_view_query.get_manual(world, input_view_entity) else {
            return Ok(());
        };
        if transparent_phase.items.is_empty() {
            return Ok(());
        }

        // use the "default" view entity if it is defined
        let view_entity = if let Ok(default_view) = self
            .default_camera_view_query
            .get_manual(world, input_view_entity)
        {
            default_view.0
        } else {
            input_view_entity
        };
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("ui_pass"),
            color_attachments: &[Some(target.get_unsampled_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }
        transparent_phase.render(&mut render_pass, world, view_entity);

        Ok(())
    }
}

pub struct TransparentUi {
    pub sort_key: (FloatOrd, u32),
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for TransparentUi {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for TransparentUi {
    type SortKey = (FloatOrd, u32);

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        items.sort_by_key(|item| item.sort_key());
    }
}

impl CachedRenderPipelinePhaseItem for TransparentUi {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
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
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetUiViewBindGroup<I> {
    type Param = SRes<UiMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: Option<()>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            ui_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}
pub struct SetUiTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetUiTextureBindGroup<I> {
    type Param = SRes<UiImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<UiBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiBatch>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batch else {
            return RenderCommandResult::Failure;
        };

        pass.set_bind_group(I, image_bind_groups.values.get(&batch.image).unwrap(), &[]);
        RenderCommandResult::Success
    }
}
pub struct DrawUiNode;
impl<P: PhaseItem> RenderCommand<P> for DrawUiNode {
    type Param = SRes<UiMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<UiBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiBatch>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Failure;
        };

        let ui_meta = ui_meta.into_inner();
        // Store the vertices
        pass.set_vertex_buffer(0, ui_meta.vertices.buffer().unwrap().slice(..));
        // Define how to "connect" the vertices
        pass.set_index_buffer(
            ui_meta.indices.buffer().unwrap().slice(..),
            0,
            bevy_render::render_resource::IndexFormat::Uint32,
        );
        // Draw the vertices
        pass.draw_indexed(batch.range.clone(), 0, 0..1);
        RenderCommandResult::Success
    }
}
