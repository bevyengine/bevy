//! Writes a raymarched signed distance field directly into the deferred gbuffer, so
//! Bevy's standard deferred PBR lighting shades it as if it were a mesh. This example assumes
//! prior familiarity with raymarching and is intended to demonstrate integration between a full-screen
//! pass and the deferred renderer.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    core_pipeline::{
        core_3d::CORE_3D_DEPTH_FORMAT,
        deferred::{
            copy_lighting_id::copy_deferred_lighting_id, node::late_deferred_prepass,
            DEFERRED_LIGHTING_PASS_ID_FORMAT, DEFERRED_PREPASS_FORMAT,
        },
        prepass::{DeferredPrepass, DepthPrepass, ViewPrepassTextures},
        Core3d, Core3dSystems, FullscreenShader,
    },
    pbr::{
        per_view_shadow_pass, shared_shadow_pass, DefaultOpaqueRendererMethod, ShadowView,
        ViewLightEntities, LATE_SHADOW_PASS,
    },
    prelude::*,
    render::{
        globals::{GlobalsBuffer, GlobalsUniform},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderContext, ViewQuery},
        view::{ViewDepthStencilTexture, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderApp, RenderStartup,
    },
};

fn main() {
    App::new()
        // Render everything through the deferred pipeline
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .add_plugins((DefaultPlugins, DeferredRaymarchPlugin, FreeCameraPlugin))
        .add_systems(Startup, setup)
        .run();
}

const SHADER_ASSET_PATH: &str = "shaders/deferred_raymarch.wgsl";

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(4.0, 3.0, 6.0).looking_at(Vec3::new(0.0, 0.2, 0.0), Vec3::Y),
        // Deferred rendering requires MSAA to be off.
        Msaa::Off,
        DepthPrepass,
        DeferredPrepass,
        AmbientLight {
            brightness: 200.0,
            ..default()
        },
        FreeCamera::default(),
    ));

    // A ground plane that catches the SDF's shadow
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -1.5, 0.0),
    ));

    // A "regular" mesh cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.2))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.4, 0.9),
            perceptual_roughness: 0.4,
            ..default()
        })),
        Transform::from_xyz(2.2, -0.9, 0.5),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

struct DeferredRaymarchPlugin;

impl Plugin for DeferredRaymarchPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_raymarch_pipelines)
            .add_systems(
                Core3d,
                (
                    // The gbuffer write must run after the mesh deferred prepass has
                    // filled the gbuffer, but *before* `copy_deferred_lighting_id`
                    // bakes the lighting-pass ids into the depth routing texture the
                    // lighting pass reads
                    raymarch_gbuffer_pass
                        .in_set(Core3dSystems::Prepass)
                        .after(late_deferred_prepass)
                        .before(copy_deferred_lighting_id),
                    // Write the SDF into the shadow maps after the mesh shadow passes
                    // have drawn, so it casts shadows like any other caster
                    raymarch_directional_shadow_pass
                        .after(per_view_shadow_pass::<LATE_SHADOW_PASS>)
                        .before(Core3dSystems::MainPass),
                    raymarch_shared_shadow_pass
                        .after(shared_shadow_pass::<LATE_SHADOW_PASS>)
                        .before(Core3dSystems::MainPass),
                ),
            );
    }
}

#[derive(Resource)]
struct RaymarchGBufferPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline_id: CachedRenderPipelineId,
}

#[derive(Resource)]
struct RaymarchShadowPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline_id: CachedRenderPipelineId,
}

fn init_raymarch_pipelines(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "raymarch_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<GlobalsUniform>(false),
            ),
        ),
    );

    let shader = asset_server.load::<Shader>(SHADER_ASSET_PATH);
    let vertex_state = fullscreen_shader.to_vertex_state();

    // Writing depth lets the SDF sort against meshes
    let depth_stencil = DepthStencilState {
        format: CORE_3D_DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(CompareFunction::GreaterEqual),
        stencil: StencilState::default(),
        bias: DepthBiasState::default(),
    };

    let gbuffer_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("raymarch_gbuffer_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state.clone(),
        fragment: Some(FragmentState {
            shader: shader.clone(),
            entry_point: Some("fragment".into()),
            targets: vec![
                Some(ColorTargetState {
                    format: DEFERRED_PREPASS_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: DEFERRED_LIGHTING_PASS_ID_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            ],
            ..default()
        }),
        depth_stencil: Some(depth_stencil.clone()),
        ..default()
    });

    let shadow_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("raymarch_shadow_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            entry_point: Some("fragment_shadow".into()),
            targets: vec![],
            ..default()
        }),
        depth_stencil: Some(depth_stencil),
        ..default()
    });

    commands.insert_resource(RaymarchGBufferPipeline {
        layout: layout.clone(),
        pipeline_id: gbuffer_pipeline_id,
    });
    commands.insert_resource(RaymarchShadowPipeline {
        layout,
        pipeline_id: shadow_pipeline_id,
    });
}

fn raymarch_bind_group(
    ctx: &RenderContext,
    pipeline_cache: &PipelineCache,
    layout: &BindGroupLayoutDescriptor,
    view_uniforms: &ViewUniforms,
    globals: &GlobalsBuffer,
) -> Option<BindGroup> {
    let view_binding = view_uniforms.uniforms.binding()?;
    let globals_binding = globals.buffer.binding()?;
    Some(ctx.render_device().create_bind_group(
        "raymarch_bind_group",
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((view_binding, globals_binding)),
    ))
}

fn raymarch_gbuffer_pass(
    view: ViewQuery<(
        &ViewUniformOffset,
        &ViewDepthStencilTexture,
        &ViewPrepassTextures,
    )>,
    pipeline: Option<Res<RaymarchGBufferPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_uniforms: Res<ViewUniforms>,
    globals: Res<GlobalsBuffer>,
    mut ctx: RenderContext,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
        return;
    };

    let (view_uniform_offset, view_depth, view_prepass_textures) = view.into_inner();

    let (Some(deferred), Some(lighting_pass_id)) = (
        &view_prepass_textures.deferred,
        &view_prepass_textures.deferred_lighting_pass_id,
    ) else {
        return;
    };

    let Some(bind_group) = raymarch_bind_group(
        &ctx,
        &pipeline_cache,
        &pipeline.layout,
        &view_uniforms,
        &globals,
    ) else {
        return;
    };

    {
        // We load rather than clear because we only want to overwrite the pixels which the deferred mesh
        // prepass didn't write
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("raymarch_gbuffer_pass"),
            color_attachments: &[
                Some(deferred.get_attachment()),
                Some(lighting_pass_id.get_attachment()),
            ],
            depth_stencil_attachment: Some(view_depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_render_pipeline(render_pipeline);
        pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        pass.draw(0..3, 0..1);
    }

    // The deferred lighting pass reconstructs world position from the prepass depth
    // texture, not the depth attachment we just wrote, so we have to do a copy.
    if let Some(prepass_depth) = &view_prepass_textures.depth {
        ctx.command_encoder().copy_texture_to_texture(
            view_depth.texture().as_image_copy(),
            prepass_depth.texture.texture.as_image_copy(),
            view_prepass_textures.size,
        );
    }
}

fn raymarch_directional_shadow_pass(
    view: ViewQuery<&ViewLightEntities>,
    shadow_views: Query<(&ShadowView, &ViewUniformOffset)>,
    pipeline: Option<Res<RaymarchShadowPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_uniforms: Res<ViewUniforms>,
    globals: Res<GlobalsBuffer>,
    mut ctx: RenderContext,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
        return;
    };

    let view_lights = view.into_inner();
    for light_entity in view_lights.lights.iter().copied() {
        let Ok((shadow_view, view_uniform_offset)) = shadow_views.get(light_entity) else {
            continue;
        };
        draw_raymarch_shadow(
            &mut ctx,
            &pipeline_cache,
            &pipeline.layout,
            &view_uniforms,
            &globals,
            render_pipeline,
            shadow_view,
            view_uniform_offset,
        );
    }
}

fn raymarch_shared_shadow_pass(
    view: ViewQuery<(&ShadowView, &ViewUniformOffset)>,
    pipeline: Option<Res<RaymarchShadowPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_uniforms: Res<ViewUniforms>,
    globals: Res<GlobalsBuffer>,
    mut ctx: RenderContext,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
        return;
    };

    let (shadow_view, view_uniform_offset) = view.into_inner();
    draw_raymarch_shadow(
        &mut ctx,
        &pipeline_cache,
        &pipeline.layout,
        &view_uniforms,
        &globals,
        render_pipeline,
        shadow_view,
        view_uniform_offset,
    );
}

fn draw_raymarch_shadow(
    ctx: &mut RenderContext,
    pipeline_cache: &PipelineCache,
    layout: &BindGroupLayoutDescriptor,
    view_uniforms: &ViewUniforms,
    globals: &GlobalsBuffer,
    render_pipeline: &RenderPipeline,
    shadow_view: &ShadowView,
    view_uniform_offset: &ViewUniformOffset,
) {
    let Some(bind_group) = raymarch_bind_group(ctx, pipeline_cache, layout, view_uniforms, globals)
    else {
        return;
    };

    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("raymarch_shadow_pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(shadow_view.depth_attachment.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    pass.set_render_pipeline(render_pipeline);
    pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
    pass.draw(0..3, 0..1);
}
