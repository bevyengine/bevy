use super::{
    pipelines::MeshletPipelines,
    resource_manager::{MeshletViewBindGroups, MeshletViewResources},
};
use crate::{
    meshlet::resource_manager::ResourceManager, LightEntity, ShadowView, ViewLightEntities,
};
use bevy_color::LinearRgba;
use bevy_core_pipeline::prepass::PreviousViewUniformOffset;
use bevy_ecs::{
    query::QueryState,
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
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
        &'static PreviousViewUniformOffset,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
        &'static ViewLightEntities,
    )>,
    view_light_query: QueryState<(
        &'static ShadowView,
        &'static LightEntity,
        &'static ViewUniformOffset,
        &'static PreviousViewUniformOffset,
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

    // TODO: Reuse compute/render passes between logical passes where possible, as they're expensive
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
            previous_view_offset,
            meshlet_view_bind_groups,
            meshlet_view_resources,
            lights,
        )) = self.main_view_query.get_manual(world, graph.view_entity())
        else {
            return Ok(());
        };

        let Some((
            clear_visibility_buffer_pipeline,
            clear_visibility_buffer_shadow_view_pipeline,
            first_instance_cull_pipeline,
            second_instance_cull_pipeline,
            first_bvh_cull_pipeline,
            second_bvh_cull_pipeline,
            first_meshlet_cull_pipeline,
            second_meshlet_cull_pipeline,
            downsample_depth_first_pipeline,
            downsample_depth_second_pipeline,
            downsample_depth_first_shadow_view_pipeline,
            downsample_depth_second_shadow_view_pipeline,
            visibility_buffer_software_raster_pipeline,
            visibility_buffer_software_raster_shadow_view_pipeline,
            visibility_buffer_hardware_raster_pipeline,
            visibility_buffer_hardware_raster_shadow_view_pipeline,
            visibility_buffer_hardware_raster_shadow_view_unclipped_pipeline,
            resolve_depth_pipeline,
            resolve_depth_shadow_view_pipeline,
            resolve_material_depth_pipeline,
            remap_1d_to_2d_dispatch_pipeline,
            fill_counts_pipeline,
        )) = MeshletPipelines::get(world)
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        render_context
            .command_encoder()
            .push_debug_group("meshlet_visibility_buffer_raster");
        let time_span = diagnostics.time_span(
            render_context.command_encoder(),
            "meshlet_visibility_buffer_raster",
        );

        let resource_manager = world.get_resource::<ResourceManager>().unwrap();
        render_context.command_encoder().clear_buffer(
            &resource_manager.visibility_buffer_raster_cluster_prev_counts,
            0,
            None,
        );

        clear_visibility_buffer_pass(
            render_context,
            &meshlet_view_bind_groups.clear_visibility_buffer,
            clear_visibility_buffer_pipeline,
            meshlet_view_resources.view_size,
        );

        render_context
            .command_encoder()
            .push_debug_group("meshlet_first_pass");
        first_cull(
            render_context,
            meshlet_view_bind_groups,
            meshlet_view_resources,
            view_offset,
            previous_view_offset,
            first_instance_cull_pipeline,
            first_bvh_cull_pipeline,
            first_meshlet_cull_pipeline,
            remap_1d_to_2d_dispatch_pipeline,
        );
        raster_pass(
            true,
            render_context,
            &meshlet_view_resources.visibility_buffer_software_raster_indirect_args,
            &meshlet_view_resources.visibility_buffer_hardware_raster_indirect_args,
            &meshlet_view_resources.dummy_render_target.default_view,
            meshlet_view_bind_groups,
            view_offset,
            visibility_buffer_software_raster_pipeline,
            visibility_buffer_hardware_raster_pipeline,
            fill_counts_pipeline,
            Some(camera),
            meshlet_view_resources.rightmost_slot,
        );
        render_context.command_encoder().pop_debug_group();

        meshlet_view_resources.depth_pyramid.downsample_depth(
            "downsample_depth",
            render_context,
            meshlet_view_resources.view_size,
            &meshlet_view_bind_groups.downsample_depth,
            downsample_depth_first_pipeline,
            downsample_depth_second_pipeline,
        );

        render_context
            .command_encoder()
            .push_debug_group("meshlet_second_pass");
        second_cull(
            render_context,
            meshlet_view_bind_groups,
            meshlet_view_resources,
            view_offset,
            previous_view_offset,
            second_instance_cull_pipeline,
            second_bvh_cull_pipeline,
            second_meshlet_cull_pipeline,
            remap_1d_to_2d_dispatch_pipeline,
        );
        raster_pass(
            false,
            render_context,
            &meshlet_view_resources.visibility_buffer_software_raster_indirect_args,
            &meshlet_view_resources.visibility_buffer_hardware_raster_indirect_args,
            &meshlet_view_resources.dummy_render_target.default_view,
            meshlet_view_bind_groups,
            view_offset,
            visibility_buffer_software_raster_pipeline,
            visibility_buffer_hardware_raster_pipeline,
            fill_counts_pipeline,
            Some(camera),
            meshlet_view_resources.rightmost_slot,
        );
        render_context.command_encoder().pop_debug_group();

        resolve_depth(
            render_context,
            view_depth.get_attachment(StoreOp::Store),
            meshlet_view_bind_groups,
            resolve_depth_pipeline,
            camera,
        );
        resolve_material_depth(
            render_context,
            meshlet_view_resources,
            meshlet_view_bind_groups,
            resolve_material_depth_pipeline,
            camera,
        );
        meshlet_view_resources.depth_pyramid.downsample_depth(
            "downsample_depth",
            render_context,
            meshlet_view_resources.view_size,
            &meshlet_view_bind_groups.downsample_depth,
            downsample_depth_first_pipeline,
            downsample_depth_second_pipeline,
        );
        render_context.command_encoder().pop_debug_group();

        for light_entity in &lights.lights {
            let Ok((
                shadow_view,
                light_type,
                view_offset,
                previous_view_offset,
                meshlet_view_bind_groups,
                meshlet_view_resources,
            )) = self.view_light_query.get_manual(world, *light_entity)
            else {
                continue;
            };

            let shadow_visibility_buffer_hardware_raster_pipeline =
                if let LightEntity::Directional { .. } = light_type {
                    visibility_buffer_hardware_raster_shadow_view_unclipped_pipeline
                } else {
                    visibility_buffer_hardware_raster_shadow_view_pipeline
                };

            render_context.command_encoder().push_debug_group(&format!(
                "meshlet_visibility_buffer_raster: {}",
                shadow_view.pass_name
            ));
            let pass_span = diagnostics.time_span(
                render_context.command_encoder(),
                shadow_view.pass_name.clone(),
            );
            clear_visibility_buffer_pass(
                render_context,
                &meshlet_view_bind_groups.clear_visibility_buffer,
                clear_visibility_buffer_shadow_view_pipeline,
                meshlet_view_resources.view_size,
            );

            render_context
                .command_encoder()
                .push_debug_group("meshlet_first_pass");
            first_cull(
                render_context,
                meshlet_view_bind_groups,
                meshlet_view_resources,
                view_offset,
                previous_view_offset,
                first_instance_cull_pipeline,
                first_bvh_cull_pipeline,
                first_meshlet_cull_pipeline,
                remap_1d_to_2d_dispatch_pipeline,
            );
            raster_pass(
                true,
                render_context,
                &meshlet_view_resources.visibility_buffer_software_raster_indirect_args,
                &meshlet_view_resources.visibility_buffer_hardware_raster_indirect_args,
                &meshlet_view_resources.dummy_render_target.default_view,
                meshlet_view_bind_groups,
                view_offset,
                visibility_buffer_software_raster_shadow_view_pipeline,
                shadow_visibility_buffer_hardware_raster_pipeline,
                fill_counts_pipeline,
                None,
                meshlet_view_resources.rightmost_slot,
            );
            render_context.command_encoder().pop_debug_group();

            meshlet_view_resources.depth_pyramid.downsample_depth(
                "downsample_depth",
                render_context,
                meshlet_view_resources.view_size,
                &meshlet_view_bind_groups.downsample_depth,
                downsample_depth_first_shadow_view_pipeline,
                downsample_depth_second_shadow_view_pipeline,
            );

            render_context
                .command_encoder()
                .push_debug_group("meshlet_second_pass");
            second_cull(
                render_context,
                meshlet_view_bind_groups,
                meshlet_view_resources,
                view_offset,
                previous_view_offset,
                second_instance_cull_pipeline,
                second_bvh_cull_pipeline,
                second_meshlet_cull_pipeline,
                remap_1d_to_2d_dispatch_pipeline,
            );
            raster_pass(
                false,
                render_context,
                &meshlet_view_resources.visibility_buffer_software_raster_indirect_args,
                &meshlet_view_resources.visibility_buffer_hardware_raster_indirect_args,
                &meshlet_view_resources.dummy_render_target.default_view,
                meshlet_view_bind_groups,
                view_offset,
                visibility_buffer_software_raster_shadow_view_pipeline,
                shadow_visibility_buffer_hardware_raster_pipeline,
                fill_counts_pipeline,
                None,
                meshlet_view_resources.rightmost_slot,
            );
            render_context.command_encoder().pop_debug_group();

            resolve_depth(
                render_context,
                shadow_view.depth_attachment.get_attachment(StoreOp::Store),
                meshlet_view_bind_groups,
                resolve_depth_shadow_view_pipeline,
                camera,
            );
            meshlet_view_resources.depth_pyramid.downsample_depth(
                "downsample_depth",
                render_context,
                meshlet_view_resources.view_size,
                &meshlet_view_bind_groups.downsample_depth,
                downsample_depth_first_shadow_view_pipeline,
                downsample_depth_second_shadow_view_pipeline,
            );
            render_context.command_encoder().pop_debug_group();
            pass_span.end(render_context.command_encoder());
        }

        time_span.end(render_context.command_encoder());

        Ok(())
    }
}

// TODO: Replace this with vkCmdClearColorImage once wgpu supports it
fn clear_visibility_buffer_pass(
    render_context: &mut RenderContext,
    clear_visibility_buffer_bind_group: &BindGroup,
    clear_visibility_buffer_pipeline: &ComputePipeline,
    view_size: UVec2,
) {
    let command_encoder = render_context.command_encoder();
    let mut clear_visibility_buffer_pass =
        command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("clear_visibility_buffer"),
            timestamp_writes: None,
        });
    clear_visibility_buffer_pass.set_pipeline(clear_visibility_buffer_pipeline);
    clear_visibility_buffer_pass.set_push_constants(0, bytemuck::bytes_of(&view_size));
    clear_visibility_buffer_pass.set_bind_group(0, clear_visibility_buffer_bind_group, &[]);
    clear_visibility_buffer_pass.dispatch_workgroups(
        view_size.x.div_ceil(16),
        view_size.y.div_ceil(16),
        1,
    );
}

fn first_cull(
    render_context: &mut RenderContext,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    meshlet_view_resources: &MeshletViewResources,
    view_offset: &ViewUniformOffset,
    previous_view_offset: &PreviousViewUniformOffset,
    first_instance_cull_pipeline: &ComputePipeline,
    first_bvh_cull_pipeline: &ComputePipeline,
    first_meshlet_cull_pipeline: &ComputePipeline,
    remap_1d_to_2d_pipeline: Option<&ComputePipeline>,
) {
    let workgroups = meshlet_view_resources.scene_instance_count.div_ceil(128);
    cull_pass(
        "meshlet_first_instance_cull",
        render_context,
        &meshlet_view_bind_groups.first_instance_cull,
        view_offset,
        previous_view_offset,
        first_instance_cull_pipeline,
        &[meshlet_view_resources.scene_instance_count],
    )
    .dispatch_workgroups(workgroups, 1, 1);

    render_context
        .command_encoder()
        .push_debug_group("meshlet_first_bvh_cull");
    let mut ping = true;
    for _ in 0..meshlet_view_resources.max_bvh_depth {
        cull_pass(
            "meshlet_first_bvh_cull_dispatch",
            render_context,
            if ping {
                &meshlet_view_bind_groups.first_bvh_cull_ping
            } else {
                &meshlet_view_bind_groups.first_bvh_cull_pong
            },
            view_offset,
            previous_view_offset,
            first_bvh_cull_pipeline,
            &[ping as u32, meshlet_view_resources.rightmost_slot],
        )
        .dispatch_workgroups_indirect(
            if ping {
                &meshlet_view_resources.first_bvh_cull_dispatch_front
            } else {
                &meshlet_view_resources.first_bvh_cull_dispatch_back
            },
            0,
        );
        render_context.command_encoder().clear_buffer(
            if ping {
                &meshlet_view_resources.first_bvh_cull_count_front
            } else {
                &meshlet_view_resources.first_bvh_cull_count_back
            },
            0,
            Some(4),
        );
        render_context.command_encoder().clear_buffer(
            if ping {
                &meshlet_view_resources.first_bvh_cull_dispatch_front
            } else {
                &meshlet_view_resources.first_bvh_cull_dispatch_back
            },
            0,
            Some(4),
        );
        ping = !ping;
    }
    render_context.command_encoder().pop_debug_group();

    let mut pass = cull_pass(
        "meshlet_first_meshlet_cull",
        render_context,
        &meshlet_view_bind_groups.first_meshlet_cull,
        view_offset,
        previous_view_offset,
        first_meshlet_cull_pipeline,
        &[meshlet_view_resources.rightmost_slot],
    );
    pass.dispatch_workgroups_indirect(&meshlet_view_resources.front_meshlet_cull_dispatch, 0);
    remap_1d_to_2d(
        pass,
        remap_1d_to_2d_pipeline,
        meshlet_view_bind_groups.remap_1d_to_2d_dispatch.as_ref(),
    );
}

fn second_cull(
    render_context: &mut RenderContext,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    meshlet_view_resources: &MeshletViewResources,
    view_offset: &ViewUniformOffset,
    previous_view_offset: &PreviousViewUniformOffset,
    second_instance_cull_pipeline: &ComputePipeline,
    second_bvh_cull_pipeline: &ComputePipeline,
    second_meshlet_cull_pipeline: &ComputePipeline,
    remap_1d_to_2d_pipeline: Option<&ComputePipeline>,
) {
    cull_pass(
        "meshlet_second_instance_cull",
        render_context,
        &meshlet_view_bind_groups.second_instance_cull,
        view_offset,
        previous_view_offset,
        second_instance_cull_pipeline,
        &[meshlet_view_resources.scene_instance_count],
    )
    .dispatch_workgroups_indirect(&meshlet_view_resources.second_pass_dispatch, 0);

    render_context
        .command_encoder()
        .push_debug_group("meshlet_second_bvh_cull");
    let mut ping = true;
    for _ in 0..meshlet_view_resources.max_bvh_depth {
        cull_pass(
            "meshlet_second_bvh_cull_dispatch",
            render_context,
            if ping {
                &meshlet_view_bind_groups.second_bvh_cull_ping
            } else {
                &meshlet_view_bind_groups.second_bvh_cull_pong
            },
            view_offset,
            previous_view_offset,
            second_bvh_cull_pipeline,
            &[ping as u32, meshlet_view_resources.rightmost_slot],
        )
        .dispatch_workgroups_indirect(
            if ping {
                &meshlet_view_resources.second_bvh_cull_dispatch_front
            } else {
                &meshlet_view_resources.second_bvh_cull_dispatch_back
            },
            0,
        );
        ping = !ping;
    }
    render_context.command_encoder().pop_debug_group();

    let mut pass = cull_pass(
        "meshlet_second_meshlet_cull",
        render_context,
        &meshlet_view_bind_groups.second_meshlet_cull,
        view_offset,
        previous_view_offset,
        second_meshlet_cull_pipeline,
        &[meshlet_view_resources.rightmost_slot],
    );
    pass.dispatch_workgroups_indirect(&meshlet_view_resources.back_meshlet_cull_dispatch, 0);
    remap_1d_to_2d(
        pass,
        remap_1d_to_2d_pipeline,
        meshlet_view_bind_groups.remap_1d_to_2d_dispatch.as_ref(),
    );
}

fn cull_pass<'a>(
    label: &'static str,
    render_context: &'a mut RenderContext,
    bind_group: &'a BindGroup,
    view_offset: &'a ViewUniformOffset,
    previous_view_offset: &'a PreviousViewUniformOffset,
    pipeline: &'a ComputePipeline,
    push_constants: &[u32],
) -> ComputePass<'a> {
    let command_encoder = render_context.command_encoder();
    let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some(label),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(
        0,
        bind_group,
        &[view_offset.offset, previous_view_offset.offset],
    );
    pass.set_push_constants(0, bytemuck::cast_slice(push_constants));
    pass
}

fn remap_1d_to_2d(
    mut pass: ComputePass,
    pipeline: Option<&ComputePipeline>,
    bind_group: Option<&BindGroup>,
) {
    if let (Some(pipeline), Some(bind_group)) = (pipeline, bind_group) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}

fn raster_pass(
    first_pass: bool,
    render_context: &mut RenderContext,
    visibility_buffer_software_raster_indirect_args: &Buffer,
    visibility_buffer_hardware_raster_indirect_args: &Buffer,
    dummy_render_target: &TextureView,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    view_offset: &ViewUniformOffset,
    visibility_buffer_software_raster_pipeline: &ComputePipeline,
    visibility_buffer_hardware_raster_pipeline: &RenderPipeline,
    fill_counts_pipeline: &ComputePipeline,
    camera: Option<&ExtractedCamera>,
    raster_cluster_rightmost_slot: u32,
) {
    let mut software_pass =
        render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some(if first_pass {
                    "raster_software_first"
                } else {
                    "raster_software_second"
                }),
                timestamp_writes: None,
            });
    software_pass.set_pipeline(visibility_buffer_software_raster_pipeline);
    software_pass.set_bind_group(
        0,
        &meshlet_view_bind_groups.visibility_buffer_raster,
        &[view_offset.offset],
    );
    software_pass.dispatch_workgroups_indirect(visibility_buffer_software_raster_indirect_args, 0);
    drop(software_pass);

    let mut hardware_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(if first_pass {
            "raster_hardware_first"
        } else {
            "raster_hardware_second"
        }),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: dummy_render_target,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(LinearRgba::BLACK.into()),
                store: StoreOp::Discard,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    if let Some(viewport) = camera.and_then(|camera| camera.viewport.as_ref()) {
        hardware_pass.set_camera_viewport(viewport);
    }
    hardware_pass.set_render_pipeline(visibility_buffer_hardware_raster_pipeline);
    hardware_pass.set_push_constants(
        ShaderStages::VERTEX,
        0,
        &raster_cluster_rightmost_slot.to_le_bytes(),
    );
    hardware_pass.set_bind_group(
        0,
        &meshlet_view_bind_groups.visibility_buffer_raster,
        &[view_offset.offset],
    );
    hardware_pass.draw_indirect(visibility_buffer_hardware_raster_indirect_args, 0);
    drop(hardware_pass);

    let mut fill_counts_pass =
        render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("fill_counts"),
                timestamp_writes: None,
            });
    fill_counts_pass.set_pipeline(fill_counts_pipeline);
    fill_counts_pass.set_bind_group(0, &meshlet_view_bind_groups.fill_counts, &[]);
    fill_counts_pass.dispatch_workgroups(1, 1, 1);
}

fn resolve_depth(
    render_context: &mut RenderContext,
    depth_stencil_attachment: RenderPassDepthStencilAttachment,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    resolve_depth_pipeline: &RenderPipeline,
    camera: &ExtractedCamera,
) {
    let mut resolve_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("resolve_depth"),
        color_attachments: &[],
        depth_stencil_attachment: Some(depth_stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    if let Some(viewport) = &camera.viewport {
        resolve_pass.set_camera_viewport(viewport);
    }
    resolve_pass.set_render_pipeline(resolve_depth_pipeline);
    resolve_pass.set_bind_group(0, &meshlet_view_bind_groups.resolve_depth, &[]);
    resolve_pass.draw(0..3, 0..1);
}

fn resolve_material_depth(
    render_context: &mut RenderContext,
    meshlet_view_resources: &MeshletViewResources,
    meshlet_view_bind_groups: &MeshletViewBindGroups,
    resolve_material_depth_pipeline: &RenderPipeline,
    camera: &ExtractedCamera,
) {
    if let (Some(material_depth), Some(resolve_material_depth_bind_group)) = (
        meshlet_view_resources.material_depth.as_ref(),
        meshlet_view_bind_groups.resolve_material_depth.as_ref(),
    ) {
        let mut resolve_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("resolve_material_depth"),
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
            resolve_pass.set_camera_viewport(viewport);
        }
        resolve_pass.set_render_pipeline(resolve_material_depth_pipeline);
        resolve_pass.set_bind_group(0, resolve_material_depth_bind_group, &[]);
        resolve_pass.draw(0..3, 0..1);
    }
}
