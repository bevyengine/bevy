use super::{
    material_pipeline_prepare::{
        MeshletViewMaterialsDeferredGBufferPrepass, MeshletViewMaterialsMainOpaquePass,
        MeshletViewMaterialsPrepass,
    },
    resource_manager::{MeshletViewBindGroups, MeshletViewResources},
    InstanceManager,
};
use crate::{
    MeshViewBindGroup, PrepassViewBindGroup, ViewEnvironmentMapUniformOffset, ViewFogUniformOffset,
    ViewLightProbesUniformOffset, ViewLightsUniformOffset, ViewScreenSpaceReflectionsUniformOffset,
};
use bevy_camera::MainPassResolutionOverride;
use bevy_camera::Viewport;
use bevy_core_pipeline::prepass::{
    MotionVectorPrepass, PreviousViewUniformOffset, ViewPrepassTextures,
};
use bevy_ecs::{
    query::{Has, QueryItem},
    world::World,
};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
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
        &'static ViewEnvironmentMapUniformOffset,
        Option<&'static MainPassResolutionOverride>,
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
            view_environment_map_offset,
            resolution_override,
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
            Some(instance_manager),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_shade_bind_group),
        ) = (
            world.get_resource::<InstanceManager>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_shade.as_ref(),
        )
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("meshlet_material_opaque_3d_pass"),
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
        let pass_span = diagnostics.pass_span(&mut render_pass, "meshlet_material_opaque_3d_pass");
        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        render_pass.set_bind_group(
            0,
            &mesh_view_bind_group.main,
            &[
                view_uniform_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
                **view_light_probes_offset,
                **view_ssr_offset,
                **view_environment_map_offset,
            ],
        );
        render_pass.set_bind_group(1, &mesh_view_bind_group.binding_array, &[]);
        render_pass.set_bind_group(2, meshlet_material_shade_bind_group, &[]);

        // 1 fullscreen triangle draw per material
        for (material_id, material_pipeline_id, material_bind_group) in
            meshlet_view_materials.iter()
        {
            if instance_manager.material_present_in_scene(material_id)
                && let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
            {
                let x = *material_id * 3;
                render_pass.set_render_pipeline(material_pipeline);
                render_pass.set_bind_group(3, material_bind_group, &[]);
                render_pass.draw(x..(x + 3), 0..1);
            }
        }

        pass_span.end(&mut render_pass);

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
        &'static PreviousViewUniformOffset,
        Option<&'static MainPassResolutionOverride>,
        Has<MotionVectorPrepass>,
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
            resolution_override,
            view_has_motion_vector_prepass,
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
            Some(instance_manager),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_shade_bind_group),
        ) = (
            world.get_resource::<PrepassViewBindGroup>(),
            world.get_resource::<InstanceManager>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_shade.as_ref(),
        )
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

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
            label: Some("meshlet_material_prepass"),
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
        let pass_span = diagnostics.pass_span(&mut render_pass, "meshlet_material_prepass");
        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        if view_has_motion_vector_prepass {
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

        render_pass.set_bind_group(1, &prepass_view_bind_group.empty_bind_group, &[]);
        render_pass.set_bind_group(2, meshlet_material_shade_bind_group, &[]);

        // 1 fullscreen triangle draw per material
        for (material_id, material_pipeline_id, material_bind_group) in
            meshlet_view_materials.iter()
        {
            if instance_manager.material_present_in_scene(material_id)
                && let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
            {
                let x = *material_id * 3;
                render_pass.set_render_pipeline(material_pipeline);
                render_pass.set_bind_group(2, material_bind_group, &[]);
                render_pass.draw(x..(x + 3), 0..1);
            }
        }

        pass_span.end(&mut render_pass);

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
        &'static PreviousViewUniformOffset,
        Option<&'static MainPassResolutionOverride>,
        Has<MotionVectorPrepass>,
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
            resolution_override,
            view_has_motion_vector_prepass,
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
            Some(instance_manager),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_shade_bind_group),
        ) = (
            world.get_resource::<PrepassViewBindGroup>(),
            world.get_resource::<InstanceManager>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_shade.as_ref(),
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

        let diagnostics = render_context.diagnostic_recorder();

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("meshlet_material_deferred_prepass"),
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
        let pass_span =
            diagnostics.pass_span(&mut render_pass, "meshlet_material_deferred_prepass");
        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        if view_has_motion_vector_prepass {
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

        render_pass.set_bind_group(1, &prepass_view_bind_group.empty_bind_group, &[]);
        render_pass.set_bind_group(2, meshlet_material_shade_bind_group, &[]);

        // 1 fullscreen triangle draw per material
        for (material_id, material_pipeline_id, material_bind_group) in
            meshlet_view_materials.iter()
        {
            if instance_manager.material_present_in_scene(material_id)
                && let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
            {
                let x = *material_id * 3;
                render_pass.set_render_pipeline(material_pipeline);
                render_pass.set_bind_group(2, material_bind_group, &[]);
                render_pass.draw(x..(x + 3), 0..1);
            }
        }

        pass_span.end(&mut render_pass);

        Ok(())
    }
}
