//! Mesh Shaders, at a high level, replace the classic vertex shader with a compute shader.
//! This allows generating geometry directly on the GPU and passing those primitives directly
//! to the fragment shader without using multiple pipelines or intermediary buffers (to pass
//! data from a compute shader to a render pipeline).
//!
//! A `MeshPipeline` contains:
//! - an optional task shader (also known as amplification shader)
//! - a mesh shader
//! - a fragment shader
//!
//! Draw calls dispatch either the task shader (if one is defined) or the mesh shader (if no task shader is defined).
//! If a task shader runs, it can run some light processing before dictating how many mesh shaders to dispatch from the gpu.
//!
//! There is a task payload to pass some data between task and mesh shaders, but you should *not* use this for large amounts of data.
//! There are hard limits typically in the low tens of kb.
//! Adding more data to the task payload is a performance consideration, and [some documentation](https://developer.nvidia.com/blog/advanced-api-performance-mesh-shaders/#not_recommended) suggests keeping the size under 236 bytes.
//!
//! A mesh shader generates geometry and passes the primitives directly to the fragment shader.
//!
//! The fragment shader operates as usual.
//!
//! This is a mesh shader example that runs every frame and renders hardcoded cube mesh data at dynamic world-space coordinates defined by the mesh workgroup id.
//! The amount of mesh shaders dispatched grows (unbounded) each frame, so will eventually exceed the platform limits and consumes more resources over time.
//! This intentionally shows performance considerations, although the code here is illustrative and not optimized.
//!
//! The color of each cube is controlled by the coordinate it is rendered at combined with the task payload color.
//! There are a couple of commented-out lines of code that render the uv or normal colors from the fragment shader, which can optionally be enabled.
//!
//! The cube primitives are also culled based on the time global, in a sine wave pattern.
//!
//! The shaders are split out into separate files intentionally, to make it easier to differentiate when specific logic is being used.
//!
#[cfg(feature = "free_camera")]
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::{
    camera::{MainPassResolutionOverride, Viewport},
    core_pipeline::{
        core_3d::{main_opaque_pass_3d, CORE_3D_DEPTH_FORMAT},
        Core3d, Core3dSystems,
    },
    material::descriptor::{MeshPipelineDescriptor, MeshState, TaskState},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        globals::{GlobalsBuffer, GlobalsUniform},
        render_resource::{
            binding_types::uniform_buffer, BindGroupEntries, BindGroupLayoutDescriptor,
            BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            CompareFunction, DepthBiasState, DepthStencilState, FragmentState, PipelineCache,
            RenderPassDescriptor, ShaderStages, StencilState, StoreOp, TextureFormat,
        },
        renderer::RenderContext,
        settings::{RenderCreation, WgpuFeatures, WgpuLimits, WgpuSettings},
        view::{
            ExtractedView, ViewDepthStencilTexture, ViewTarget, ViewUniform, ViewUniformOffset,
            ViewUniforms,
        },
        RenderApp, RenderPlugin, RenderStartup,
    },
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    features: WgpuFeatures::EXPERIMENTAL_MESH_SHADER
                        | WgpuFeatures::PASSTHROUGH_SHADERS,
                    limits: WgpuLimits::default().using_recommended_minimum_mesh_shader_values(),
                    ..default()
                })),
                ..default()
            }),
            MeshShaderDemoPlugin,
            #[cfg(feature = "free_camera")]
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-9.0, 1.0, -9.0).looking_at(Vec3::new(9., 4., 9.), Vec3::Y),
        // disable msaa for simplicity
        Msaa::Off,
        #[cfg(feature = "free_camera")]
        FreeCamera::default(),
    ));
}

struct MeshShaderDemoPlugin;
impl Plugin for MeshShaderDemoPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_mesh_pipelines)
            .add_systems(
                Core3d,
                draw_mesh_shader_cubes
                    .after(main_opaque_pass_3d)
                    .in_set(Core3dSystems::MainPass),
            );
    }
}

fn init_mesh_pipelines(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let task_shader = asset_server.load::<Shader>("shaders/mesh_shader_intro/task.wesl");
    let mesh_shader = asset_server.load::<Shader>("shaders/mesh_shader_intro/mesh.wesl");
    let fragment_shader = asset_server.load::<Shader>("shaders/mesh_shader_intro/fragment.wesl");

    let layout = BindGroupLayoutDescriptor::new(
        "custom_mesh_shader_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::MESH | ShaderStages::TASK | ShaderStages::FRAGMENT,
            (
                uniform_buffer::<GlobalsUniform>(false),
                uniform_buffer::<ViewUniform>(true),
            ),
        ),
    );

    let depth_stencil = DepthStencilState {
        format: CORE_3D_DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(CompareFunction::GreaterEqual),
        stencil: StencilState::default(),
        bias: DepthBiasState::default(),
    };

    let cached_pipeline_id = pipeline_cache.queue_mesh_pipeline(MeshPipelineDescriptor {
        label: Some("custom_mesh_shader_pipeline".into()),
        layout: vec![layout.clone()],
        immediate_size: 0,
        task: Some(TaskState {
            shader: task_shader,
            entry_point: Some("task".into()),
            ..default()
        }),
        mesh: MeshState {
            shader: mesh_shader,
            entry_point: Some("mesh".into()),
            ..default()
        },
        primitive: Default::default(),
        depth_stencil: Some(depth_stencil),
        multisample: Default::default(),
        fragment: Some(FragmentState {
            shader: fragment_shader,
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        zero_initialize_workgroup_memory: false,
    });

    commands.insert_resource(MyMeshShaderDrawNode {
        mesh_pipeline: cached_pipeline_id,
        layout,
    });
}

#[derive(Resource)]
struct MyMeshShaderDrawNode {
    mesh_pipeline: CachedRenderPipelineId,
    layout: BindGroupLayoutDescriptor,
}

/// The underlying `create_mesh_pipeline` returns a `RenderPipeline`, which means
/// mesh shaders can re-use the `RenderPass` infrastructure from other examples to
/// start a `TrackedRenderPass`.
fn draw_mesh_shader_cubes(
    mut views: Query<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewDepthStencilTexture,
        &ViewUniformOffset,
        Option<&MainPassResolutionOverride>,
    )>,
    mut render_context: RenderContext,
    data: Res<MyMeshShaderDrawNode>,
    view_uniforms: Res<ViewUniforms>,
    globals: Res<GlobalsBuffer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let Some(mesh_pipeline) = pipeline_cache.get_render_pipeline(data.mesh_pipeline) else {
        return;
    };

    for (camera, _, target, depth, view_uniform_offset, resolution_override) in &mut views {
        let Some(view_binding) = view_uniforms.uniforms.binding() else {
            return;
        };
        let Some(globals_binding) = globals.buffer.binding() else {
            return;
        };
        let bind_group = render_context.render_device().create_bind_group(
            "custom_task_mesh_bind_group",
            &pipeline_cache.get_bind_group_layout(&data.layout),
            &BindGroupEntries::sequential((globals_binding, view_binding)),
        );

        {
            let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("custom_mesh_shader_pass"),
                // Write directly to the view target
                color_attachments: &[Some(target.get_color_attachment())],
                depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_render_pipeline(&mesh_pipeline);
            pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
            if let Some(viewport) =
                Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
            {
                pass.set_camera_viewport(&viewport);
            }

            // Since this MeshPipeline has a task shader, this call
            // dispatches the task shader workgroup
            pass.draw_mesh_tasks(1, 1, 1);
        }
    }
}
