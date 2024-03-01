use super::{
    gpu_scene::{MeshletViewBindGroups, MeshletViewResources},
    pipelines::MeshletPipelines,
};
use crate::{LightEntity, ShadowView, ViewLightEntities};
use bevy_color::LinearRgba;
use bevy_ecs::{
    query::QueryState,
    world::{FromWorld, World},
};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::*,
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewUniformOffset},
};

/// Rasterize meshlets into a depth buffer, and optional visibility buffer + material depth buffer for shading passes.
pub struct MeshletVisibilityBufferRasterPassNode {
    main_view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewDepthTexture,
        &'static ViewUniformOffset,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
        &'static ViewLightEntities,
    )>,
    view_light_query: QueryState<(
        &'static ShadowView,
        &'static LightEntity,
        &'static ViewUniformOffset,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    )>,
}

impl FromWorld for MeshletVisibilityBufferRasterPassNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            view_light_query: QueryState::new(world),
        }
    }
}

impl Node for MeshletVisibilityBufferRasterPassNode {
    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.view_light_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Ok((
            camera,
            view_depth,
            view_offset,
            meshlet_view_bind_groups,
            meshlet_view_resources,
            lights,
        )) = self.main_view_query.get_manual(world, graph.view_entity())
        else {
            return Ok(());
        };

        let Some((
            culling_first_pipeline,
            culling_second_pipeline,
            write_index_buffer_first_pipeline,
            write_index_buffer_second_pipeline,
            downsample_depth_pipeline,
            visibility_buffer_raster_pipeline,
            visibility_buffer_raster_depth_only_pipeline,
            visibility_buffer_raster_depth_only_clamp_ortho,
            copy_material_depth_pipeline,
        )) = MeshletPipelines::get(world)
        else {
            return Ok(());
        };

        let culling_workgroups = meshlet_view_resources.scene_meshlet_count.div_ceil(128);
        let write_index_buffer_workgroups = (meshlet_view_resources.scene_meshlet_count as f32)
            .cbrt()
            .ceil() as u32;

        render_context
            .command_encoder()
            .push_debug_group("meshlet_visibility_buffer_raster_pass");
        if meshlet_view_resources.occlusion_buffer_needs_clearing {
            render_context.command_encoder().clear_buffer(
                &meshlet_view_resources.occlusion_buffer,
                0,
                None,
            );
        }
        cull_pass(
            "meshlet_culling_first_pass",
            render_context,
            meshlet_view_bind_groups,
            view_offset,
            culling_first_pipeline,
            culling_workgroups,
        );
        write_index_buffer_pass(
            "meshlet_write_index_buffer_first_pass",
            render_context,
            &meshlet_view_bind_groups.write_index_buffer_first,
            write_index_buffer_first_pipeline,
            write_index_buffer_workgroups,
        );
        render_context.command_encoder().clear_buffer(
            &meshlet_view_resources.occlusion_buffer,
            0,
            None,
        );
        raster_pass(
            true,
            render_context,
            meshlet_view_resources,
            &meshlet_view_resources.visibility_buffer_draw_command_buffer_first,
            view_depth.get_attachment(StoreOp::Store),
            meshlet_view_bind_groups,
            view_offset,
            visibility_buffer_raster_pipeline,
            Some(camera),
        );
        downsample_depth(
            render_context,
            meshlet_view_resources,
            meshlet_view_bind_groups,
            downsample_depth_pipeline,
        );
        cull_pass(
            "meshlet_culling_second_pass",
            render_context,
            meshlet_view_bind_groups,
            view_offset,
            culling_second_pipeline,
            culling_workgroups,
        );
        write_index_buffer_pass(
            "meshlet_write_index_buffer_second_pass",
            render_context,
            &meshlet_view_bind_groups.write_index_buffer_second,
            write_index_buffer_second_pipeline,
            write_index_buffer_workgroups,
        );
        raster_pass(
            false,
            render_context,
            meshlet_view_resources,
            &meshlet_view_resources.visibility_buffer_draw_command_buffer_second,
            view_depth.get_attachment(StoreOp::Store),
            meshlet_view_bind_groups,
            view_offset,
            visibility_buffer_raster_pipeline,
            Some(camera),
        );
        copy_material_depth_pass(
            render_context,
            meshlet_view_resources,
            meshlet_view_bind_groups,
            copy_material_depth_pipeline,
            camera,
        );
        render_context.command_encoder().pop_debug_group();

        for light_entity in &lights.lights {
            let Ok((
                shadow_view,
                light_type,
                view_offset,
                meshlet_view_bind_groups,
                meshlet_view_resources,
            )) = self.view_light_query.get_manual(world, *light_entity)
            else {
                continue;
            };

            let shadow_visibility_buffer_pipeline = match light_type {
                LightEntity::Directional { .. } => visibility_buffer_raster_depth_only_clamp_ortho,
                _ => visibility_buffer_raster_depth_only_pipeline,
            };

            render_context.command_encoder().push_debug_group(&format!(
                "meshlet_visibility_buffer_raster_pass: {}",
                shadow_view.pass_name
            ));
            if meshlet_view_resources.occlusion_buffer_needs_clearing {
                render_context.command_encoder().clear_buffer(
                    &meshlet_view_resources.occlusion_buffer,
                    0,
                    None,
                );
            }
            cull_pass(
                "meshlet_culling_first_pass",
                render_context,
                meshlet_view_bind_groups,
                view_offset,
                culling_first_pipeline,
                culling_workgroups,
            );
            write_index_buffer_pass(
                "meshlet_write_index_buffer_first_pass",
                render_context,
                &meshlet_view_bind_groups.write_index_buffer_first,
                write_index_buffer_first_pipeline,
                write_index_buffer_workgroups,
            );
            render_context.command_encoder().clear_buffer(
                &meshlet_view_resources.occlusion_buffer,
                0,
                None,
            );
            raster_pass(
                true,
                render_context,
                meshlet_view_resources,
                &meshlet_view_resources.visibility_buffer_draw_command_buffer_first,
                shadow_view.depth_attachment.get_attachment(StoreOp::Store),
                meshlet_view_bind_groups,
                view_offset,
                shadow_visibility_buffer_pipeline,
                None,
            );
            downsample_depth(
                render_context,
                meshlet_view_resources,
                meshlet_view_bind_groups,
                downsample_depth_pipeline,
            );
            cull_pass(
                "meshlet_culling_second_pass",
                render_context,
                meshlet_view_bind_groups,
                view_offset,
                culling_second_pipeline,
                culling_workgroups,
            );
            write_index_buffer_pass(
                "meshlet_write_index_buffer_second_pass",
                render_context,
                &meshlet_view_bind_groups.write_index_buffer_second,
                write_index_buffer_second_pipeline,
                write_index_buffer_workgroups,
            );
            raster_pass(
                false,
                render_context,
                meshlet_view_resources,
                &meshlet_view_resources.visibility_buffer_draw_command_buffer_second,
                shadow_view.depth_attachment.get_attachment(StoreOp::Store),
                meshlet_view_bind_groups,
                view_offset,
                shadow_visibility_buffer_pipeline,
                None,
            );
            render_context.command_encoder().pop_debug_group();
        }

        Ok(())
    }
}

fn cull_pass(
    label: &'static str,
    render_context: &mut RenderContext,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    view_offset: &ViewUniformOffset,
    culling_pipeline: &ComputePipeline,
    culling_workgroups: u32,
) {
    let command_encoder = render_context.command_encoder();
    let mut cull_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some(label),
        timestamp_writes: None,
    });
    cull_pass.set_bind_group(0, &meshlet_view_bind_groups.culling, &[view_offset.offset]);
    cull_pass.set_pipeline(culling_pipeline);
    cull_pass.dispatch_workgroups(culling_workgroups, 1, 1);
}

fn write_index_buffer_pass(
    label: &'static str,
    render_context: &mut RenderContext,
    write_index_buffer_bind_group: &BindGroup,
    write_index_buffer_pipeline: &ComputePipeline,
    write_index_buffer_workgroups: u32,
) {
    let command_encoder = render_context.command_encoder();
    let mut cull_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some(label),
        timestamp_writes: None,
    });
    cull_pass.set_bind_group(0, write_index_buffer_bind_group, &[]);
    cull_pass.set_pipeline(write_index_buffer_pipeline);
    cull_pass.dispatch_workgroups(
        write_index_buffer_workgroups,
        write_index_buffer_workgroups,
        write_index_buffer_workgroups,
    );
}

#[allow(clippy::too_many_arguments)]
fn raster_pass(
    first_pass: bool,
    render_context: &mut RenderContext,
    meshlet_view_resources: &MeshletViewResources,
    visibility_buffer_draw_command_buffer: &Buffer,
    depth_stencil_attachment: RenderPassDepthStencilAttachment,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    view_offset: &ViewUniformOffset,
    visibility_buffer_raster_pipeline: &RenderPipeline,
    camera: Option<&ExtractedCamera>,
) {
    let mut color_attachments_filled = [None, None];
    if let (Some(visibility_buffer), Some(material_depth_color)) = (
        meshlet_view_resources.visibility_buffer.as_ref(),
        meshlet_view_resources.material_depth_color.as_ref(),
    ) {
        let load = if first_pass {
            LoadOp::Clear(LinearRgba::BLACK.into())
        } else {
            LoadOp::Load
        };
        color_attachments_filled = [
            Some(RenderPassColorAttachment {
                view: &visibility_buffer.default_view,
                resolve_target: None,
                ops: Operations {
                    load,
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &material_depth_color.default_view,
                resolve_target: None,
                ops: Operations {
                    load,
                    store: StoreOp::Store,
                },
            }),
        ];
    }

    let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(if first_pass {
            "meshlet_visibility_buffer_raster_first_pass"
        } else {
            "meshlet_visibility_buffer_raster_second_pass"
        }),
        color_attachments: if color_attachments_filled[0].is_none() {
            &[]
        } else {
            &color_attachments_filled
        },
        depth_stencil_attachment: Some(depth_stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    if let Some(viewport) = camera.and_then(|camera| camera.viewport.as_ref()) {
        draw_pass.set_camera_viewport(viewport);
    }

    draw_pass.set_index_buffer(
        meshlet_view_resources
            .visibility_buffer_draw_index_buffer
            .slice(..),
        0,
        IndexFormat::Uint32,
    );
    draw_pass.set_bind_group(
        0,
        &meshlet_view_bind_groups.visibility_buffer_raster,
        &[view_offset.offset],
    );
    draw_pass.set_render_pipeline(visibility_buffer_raster_pipeline);
    draw_pass.draw_indexed_indirect(visibility_buffer_draw_command_buffer, 0);
}

fn downsample_depth(
    render_context: &mut RenderContext,
    meshlet_view_resources: &MeshletViewResources,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    downsample_depth_pipeline: &RenderPipeline,
) {
    render_context
        .command_encoder()
        .push_debug_group("meshlet_downsample_depth");

    for i in 0..meshlet_view_resources.depth_pyramid_mips.len() {
        let downsample_pass = RenderPassDescriptor {
            label: Some("meshlet_downsample_depth_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &meshlet_view_resources.depth_pyramid_mips[i],
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(LinearRgba::BLACK.into()),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut downsample_pass = render_context.begin_tracked_render_pass(downsample_pass);
        downsample_pass.set_bind_group(0, &meshlet_view_bind_groups.downsample_depth[i], &[]);
        downsample_pass.set_render_pipeline(downsample_depth_pipeline);
        downsample_pass.draw(0..3, 0..1);
    }

    render_context.command_encoder().pop_debug_group();
}

fn copy_material_depth_pass(
    render_context: &mut RenderContext,
    meshlet_view_resources: &MeshletViewResources,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    copy_material_depth_pipeline: &RenderPipeline,
    camera: &ExtractedCamera,
) {
    if let (Some(material_depth), Some(copy_material_depth_bind_group)) = (
        meshlet_view_resources.material_depth.as_ref(),
        meshlet_view_bind_groups.copy_material_depth.as_ref(),
    ) {
        let mut copy_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("meshlet_copy_material_depth_pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &material_depth.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(viewport) = &camera.viewport {
            copy_pass.set_camera_viewport(viewport);
        }

        copy_pass.set_bind_group(0, copy_material_depth_bind_group, &[]);
        copy_pass.set_render_pipeline(copy_material_depth_pipeline);
        copy_pass.draw(0..3, 0..1);
    }
}
