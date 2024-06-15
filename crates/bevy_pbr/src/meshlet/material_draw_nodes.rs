use super::{
    gpu_scene::{MeshletViewBindGroups, MeshletViewResources},
    material_draw_prepare::{
        MeshletViewMaterialsDeferredGBufferPrepass, MeshletViewMaterialsMainOpaquePass,
        MeshletViewMaterialsPrepass,
    },
    MeshletGpuScene,
};
use crate::{
    MeshViewBindGroup, PrepassViewBindGroup, ViewFogUniformOffset, ViewLightProbesUniformOffset,
    ViewLightsUniformOffset, ViewScreenSpaceReflectionsUniformOffset,
};
use bevy_core_pipeline::prepass::{PreviousViewUniformOffset, ViewPrepassTextures};
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
        StoreOp,
    },
    renderer::RenderContext,
    view::{ViewTarget, ViewUniformOffset},
};

/// Fullscreen shading pass based on the visibility buffer generated from rasterizing meshlets.
#[derive(Default)]
pub struct MeshletMainOpaquePass3dNode;
impl ViewNode for MeshletMainOpaquePass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static MeshViewBindGroup,
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
        &'static ViewLightProbesUniformOffset,
        &'static ViewScreenSpaceReflectionsUniformOffset,
        &'static MeshletViewMaterialsMainOpaquePass,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            target,
            mesh_view_bind_group,
            view_uniform_offset,
            view_lights_offset,
            view_fog_offset,
            view_light_probes_offset,
            view_ssr_offset,
            meshlet_view_materials,
            meshlet_view_bind_groups,
            meshlet_view_resources,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if meshlet_view_materials.is_empty() {
            return Ok(());
        }

        let (
            Some(meshlet_gpu_scene),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_draw_bind_group),
        ) = (
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_draw.as_ref(),
        )
        else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("meshlet_main_opaque_pass_3d"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &meshlet_material_depth.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        render_pass.set_bind_group(
            0,
            &mesh_view_bind_group.value,
            &[
                view_uniform_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
                **view_light_probes_offset,
                **view_ssr_offset,
            ],
        );
        render_pass.set_bind_group(1, meshlet_material_draw_bind_group, &[]);

        // 1 fullscreen triangle draw per material
        for (material_id, material_pipeline_id, material_bind_group) in
            meshlet_view_materials.iter()
        {
            if meshlet_gpu_scene.material_present_in_scene(material_id) {
                if let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
                {
                    let x = *material_id * 3;
                    render_pass.set_render_pipeline(material_pipeline);
                    render_pass.set_bind_group(2, material_bind_group, &[]);
                    render_pass.draw(x..(x + 3), 0..1);
                }
            }
        }

        Ok(())
    }
}

/// Fullscreen pass to generate prepass textures based on the visibility buffer generated from rasterizing meshlets.
#[derive(Default)]
pub struct MeshletPrepassNode;
impl ViewNode for MeshletPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        Option<&'static PreviousViewUniformOffset>,
        &'static MeshletViewMaterialsPrepass,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            view_prepass_textures,
            view_uniform_offset,
            previous_view_uniform_offset,
            meshlet_view_materials,
            meshlet_view_bind_groups,
            meshlet_view_resources,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if meshlet_view_materials.is_empty() {
            return Ok(());
        }

        let (
            Some(prepass_view_bind_group),
            Some(meshlet_gpu_scene),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_draw_bind_group),
        ) = (
            world.get_resource::<PrepassViewBindGroup>(),
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_draw.as_ref(),
        )
        else {
            return Ok(());
        };

        let color_attachments = vec![
            view_prepass_textures
                .normal
                .as_ref()
                .map(|normals_texture| normals_texture.get_attachment()),
            view_prepass_textures
                .motion_vectors
                .as_ref()
                .map(|motion_vectors_texture| motion_vectors_texture.get_attachment()),
            // Use None in place of Deferred attachments
            None,
            None,
        ];

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("meshlet_prepass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &meshlet_material_depth.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        if let Some(previous_view_uniform_offset) = previous_view_uniform_offset {
            render_pass.set_bind_group(
                0,
                prepass_view_bind_group.motion_vectors.as_ref().unwrap(),
                &[
                    view_uniform_offset.offset,
                    previous_view_uniform_offset.offset,
                ],
            );
        } else {
            render_pass.set_bind_group(
                0,
                prepass_view_bind_group.no_motion_vectors.as_ref().unwrap(),
                &[view_uniform_offset.offset],
            );
        }

        render_pass.set_bind_group(1, meshlet_material_draw_bind_group, &[]);

        // 1 fullscreen triangle draw per material
        for (material_id, material_pipeline_id, material_bind_group) in
            meshlet_view_materials.iter()
        {
            if meshlet_gpu_scene.material_present_in_scene(material_id) {
                if let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
                {
                    let x = *material_id * 3;
                    render_pass.set_render_pipeline(material_pipeline);
                    render_pass.set_bind_group(2, material_bind_group, &[]);
                    render_pass.draw(x..(x + 3), 0..1);
                }
            }
        }

        Ok(())
    }
}

/// Fullscreen pass to generate a gbuffer based on the visibility buffer generated from rasterizing meshlets.
#[derive(Default)]
pub struct MeshletDeferredGBufferPrepassNode;
impl ViewNode for MeshletDeferredGBufferPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        Option<&'static PreviousViewUniformOffset>,
        &'static MeshletViewMaterialsDeferredGBufferPrepass,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            view_prepass_textures,
            view_uniform_offset,
            previous_view_uniform_offset,
            meshlet_view_materials,
            meshlet_view_bind_groups,
            meshlet_view_resources,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if meshlet_view_materials.is_empty() {
            return Ok(());
        }

        let (
            Some(prepass_view_bind_group),
            Some(meshlet_gpu_scene),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_draw_bind_group),
        ) = (
            world.get_resource::<PrepassViewBindGroup>(),
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_draw.as_ref(),
        )
        else {
            return Ok(());
        };

        let color_attachments = vec![
            view_prepass_textures
                .normal
                .as_ref()
                .map(|normals_texture| normals_texture.get_attachment()),
            view_prepass_textures
                .motion_vectors
                .as_ref()
                .map(|motion_vectors_texture| motion_vectors_texture.get_attachment()),
            view_prepass_textures
                .deferred
                .as_ref()
                .map(|deferred_texture| deferred_texture.get_attachment()),
            view_prepass_textures
                .deferred_lighting_pass_id
                .as_ref()
                .map(|deferred_lighting_pass_id| deferred_lighting_pass_id.get_attachment()),
        ];

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("meshlet_deferred_prepass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &meshlet_material_depth.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        if let Some(previous_view_uniform_offset) = previous_view_uniform_offset {
            render_pass.set_bind_group(
                0,
                prepass_view_bind_group.motion_vectors.as_ref().unwrap(),
                &[
                    view_uniform_offset.offset,
                    previous_view_uniform_offset.offset,
                ],
            );
        } else {
            render_pass.set_bind_group(
                0,
                prepass_view_bind_group.no_motion_vectors.as_ref().unwrap(),
                &[view_uniform_offset.offset],
            );
        }

        render_pass.set_bind_group(1, meshlet_material_draw_bind_group, &[]);

        // 1 fullscreen triangle draw per material
        for (material_id, material_pipeline_id, material_bind_group) in
            meshlet_view_materials.iter()
        {
            if meshlet_gpu_scene.material_present_in_scene(material_id) {
                if let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
                {
                    let x = *material_id * 3;
                    render_pass.set_render_pipeline(material_pipeline);
                    render_pass.set_bind_group(2, material_bind_group, &[]);
                    render_pass.draw(x..(x + 3), 0..1);
                }
            }
        }

        Ok(())
    }
}
