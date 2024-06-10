//! This example shows how to manually render 2d items using "mid level render apis" with a custom
//! pipeline for 2d meshes.
//! It doesn't use the [`Material2d`] abstraction, but changes the vertex buffer to include vertex color.
//! Check out the "mesh2d" example for simpler / higher level 2d meshes.
//!
//! [`Material2d`]: bevy::sprite::Material2d

use bevy::{
    color::palettes::basic::YELLOW,
    core_pipeline::core_2d::Transparent2d,
    math::FloatOrd,
    prelude::*,
    render::{
        mesh::{GpuMesh, Indices, MeshVertexAttribute},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, SetItemPipeline,
            ViewSortedRenderPhases,
        },
        render_resource::{
            BlendState, ColorTargetState, ColorWrites, Face, FragmentState, FrontFace,
            MultisampleState, PipelineCache, PolygonMode, PrimitiveState, PrimitiveTopology,
            RenderPipelineDescriptor, SpecializedRenderPipeline, SpecializedRenderPipelines,
            TextureFormat, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
        },
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    sprite::{
        extract_mesh2d, DrawMesh2d, Material2dBindGroupId, Mesh2dHandle, Mesh2dPipeline,
        Mesh2dPipelineKey, Mesh2dTransforms, MeshFlags, RenderMesh2dInstance, SetMesh2dBindGroup,
        SetMesh2dViewBindGroup, WithMesh2d,
    },
    utils::EntityHashMap,
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ColoredMesh2dPlugin))
        .add_systems(Startup, star)
        .run();
}

fn star(
    mut commands: Commands,
    // We will add a new Mesh for the star being created
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Let's define the mesh for the object we want to draw: a nice star.
    // We will specify here what kind of topology is used to define the mesh,
    // that is, how triangles are built from the vertices. We will use a
    // triangle list, meaning that each vertex of the triangle has to be
    // specified. We set `RenderAssetUsages::RENDER_WORLD`, meaning this mesh
    // will not be accessible in future frames from the `meshes` resource, in
    // order to save on memory once it has been uploaded to the GPU.
    let mut star = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Vertices need to have a position attribute. We will use the following
    // vertices (I hope you can spot the star in the schema).
    //
    //        1
    //
    //     10   2
    // 9      0      3
    //     8     4
    //        6
    //   7        5
    //
    // These vertices are specified in 3D space.
    let mut v_pos = vec![[0.0, 0.0, 0.0]];
    for i in 0..10 {
        // The angle between each vertex is 1/10 of a full rotation.
        let a = i as f32 * PI / 5.0;
        // The radius of inner vertices (even indices) is 100. For outer vertices (odd indices) it's 200.
        let r = (1 - i % 2) as f32 * 100.0 + 100.0;
        // Add the vertex position.
        v_pos.push([r * a.sin(), r * a.cos(), 0.0]);
    }
    // Set the position attribute
    star.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);
    // And a RGB color attribute as well
    let mut v_color: Vec<u32> = vec![LinearRgba::BLACK.as_u32()];
    v_color.extend_from_slice(&[LinearRgba::from(YELLOW).as_u32(); 10]);
    star.insert_attribute(
        MeshVertexAttribute::new("Vertex_Color", 1, VertexFormat::Uint32),
        v_color,
    );

    // Now, we specify the indices of the vertex that are going to compose the
    // triangles in our star. Vertices in triangles have to be specified in CCW
    // winding (that will be the front face, colored). Since we are using
    // triangle list, we will specify each triangle as 3 vertices
    //   First triangle: 0, 2, 1
    //   Second triangle: 0, 3, 2
    //   Third triangle: 0, 4, 3
    //   etc
    //   Last triangle: 0, 1, 10
    let mut indices = vec![0, 1, 10];
    for i in 2..=10 {
        indices.extend_from_slice(&[0, i, i - 1]);
    }
    star.insert_indices(Indices::U32(indices));

    // We can now spawn the entities for the star and the camera
    commands.spawn((
        // We use a marker component to identify the custom colored meshes
        ColoredMesh2d,
        // The `Handle<Mesh>` needs to be wrapped in a `Mesh2dHandle` to use 2d rendering instead of 3d
        Mesh2dHandle(meshes.add(star)),
        // This bundle's components are needed for something to be rendered
        SpatialBundle::INHERITED_IDENTITY,
    ));

    // Spawn the camera
    commands.spawn(Camera2dBundle::default());
}

/// A marker component for colored 2d meshes
#[derive(Component, Default)]
pub struct ColoredMesh2d;

/// Custom pipeline for 2d meshes with vertex colors
#[derive(Resource)]
pub struct ColoredMesh2dPipeline {
    /// this pipeline wraps the standard [`Mesh2dPipeline`]
    mesh2d_pipeline: Mesh2dPipeline,
}

impl FromWorld for ColoredMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh2d_pipeline: Mesh2dPipeline::from_world(world),
        }
    }
}

// We implement `SpecializedPipeline` to customize the default rendering from `Mesh2dPipeline`
impl SpecializedRenderPipeline for ColoredMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Customize how to store the meshes' vertex attributes in the vertex buffer
        // Our meshes only have position and color
        let formats = vec![
            // Position
            VertexFormat::Float32x3,
            // Color
            VertexFormat::Uint32,
        ];

        let vertex_layout =
            VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, formats);

        let format = match key.contains(Mesh2dPipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                // Use our custom shader
                shader: COLORED_MESH2D_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: vec![],
                // Use our custom vertex buffer
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                // Use our custom shader
                shader: COLORED_MESH2D_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            // Use the two standard uniforms for 2d meshes
            layout: vec![
                // Bind group 0 is the view uniform
                self.mesh2d_pipeline.view_layout.clone(),
                // Bind group 1 is the mesh uniform
                self.mesh2d_pipeline.mesh_layout.clone(),
            ],
            push_constant_ranges: vec![],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("colored_mesh2d_pipeline".into()),
        }
    }
}

// This specifies how to render a colored 2d mesh
type DrawColoredMesh2d = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform as bind group 0
    SetMesh2dViewBindGroup<0>,
    // Set the mesh uniform as bind group 1
    SetMesh2dBindGroup<1>,
    // Draw the mesh
    DrawMesh2d,
);

// The custom shader can be inline like here, included from another file at build time
// using `include_str!()`, or loaded like any other asset with `asset_server.load()`.
const COLORED_MESH2D_SHADER: &str = r"
// Import the standard 2d mesh uniforms and set their bind groups
#import bevy_sprite::mesh2d_functions

// The structure of the vertex buffer is as specified in `specialize()`
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: u32,
};

struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex
    @builtin(position) clip_position: vec4<f32>,
    // We pass the vertex color to the fragment shader in location 0
    @location(0) color: vec4<f32>,
};

/// Entry point for the vertex shader
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Project the world position of the mesh into screen position
    let model = mesh2d_functions::get_world_from_local(vertex.instance_index);
    out.clip_position = mesh2d_functions::mesh2d_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    // Unpack the `u32` from the vertex buffer into the `vec4<f32>` used by the fragment shader
    out.color = vec4<f32>((vec4<u32>(vertex.color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
    return out;
}

// The input of the fragment shader must correspond to the output of the vertex shader for all `location`s
struct FragmentInput {
    // The color is interpolated between vertices by default
    @location(0) color: vec4<f32>,
};

/// Entry point for the fragment shader
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return in.color;
}
";

/// Plugin that renders [`ColoredMesh2d`]s
pub struct ColoredMesh2dPlugin;

/// Handle to the custom shader with a unique random ID
pub const COLORED_MESH2D_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(13828845428412094821);

/// Our custom pipeline needs its own instance storage
#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenderColoredMesh2dInstances(EntityHashMap<Entity, RenderMesh2dInstance>);

impl Plugin for ColoredMesh2dPlugin {
    fn build(&self, app: &mut App) {
        // Load our custom shader
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        shaders.insert(
            &COLORED_MESH2D_SHADER_HANDLE,
            Shader::from_wgsl(COLORED_MESH2D_SHADER, file!()),
        );

        // Register our custom draw function, and add our render systems
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_render_command::<Transparent2d, DrawColoredMesh2d>()
            .init_resource::<SpecializedRenderPipelines<ColoredMesh2dPipeline>>()
            .init_resource::<RenderColoredMesh2dInstances>()
            .add_systems(
                ExtractSchedule,
                extract_colored_mesh2d.after(extract_mesh2d),
            )
            .add_systems(Render, queue_colored_mesh2d.in_set(RenderSet::QueueMeshes));
    }

    fn finish(&self, app: &mut App) {
        // Register our custom pipeline
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ColoredMesh2dPipeline>();
    }
}

/// Extract the [`ColoredMesh2d`] marker component into the render app
pub fn extract_colored_mesh2d(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    // When extracting, you must use `Extract` to mark the `SystemParam`s
    // which should be taken from the main world.
    query: Extract<
        Query<(Entity, &ViewVisibility, &GlobalTransform, &Mesh2dHandle), With<ColoredMesh2d>>,
    >,
    mut render_mesh_instances: ResMut<RenderColoredMesh2dInstances>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, view_visibility, transform, handle) in &query {
        if !view_visibility.get() {
            continue;
        }

        let transforms = Mesh2dTransforms {
            world_from_local: (&transform.affine()).into(),
            flags: MeshFlags::empty().bits(),
        };

        values.push((entity, ColoredMesh2d));
        render_mesh_instances.insert(
            entity,
            RenderMesh2dInstance {
                mesh_asset_id: handle.0.id(),
                transforms,
                material_bind_group_id: Material2dBindGroupId::default(),
                automatic_batching: false,
            },
        );
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

/// Queue the 2d meshes marked with [`ColoredMesh2d`] using our custom pipeline and draw function
#[allow(clippy::too_many_arguments)]
pub fn queue_colored_mesh2d(
    transparent_draw_functions: Res<DrawFunctions<Transparent2d>>,
    colored_mesh2d_pipeline: Res<ColoredMesh2dPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ColoredMesh2dPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<GpuMesh>>,
    render_mesh_instances: Res<RenderColoredMesh2dInstances>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut views: Query<(Entity, &VisibleEntities, &ExtractedView)>,
) {
    if render_mesh_instances.is_empty() {
        return;
    }
    // Iterate each view (a camera is a view)
    for (view_entity, visible_entities, view) in &mut views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let draw_colored_mesh2d = transparent_draw_functions.read().id::<DrawColoredMesh2d>();

        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        // Queue all entities visible to that view
        for visible_entity in visible_entities.iter::<WithMesh2d>() {
            if let Some(mesh_instance) = render_mesh_instances.get(visible_entity) {
                let mesh2d_handle = mesh_instance.mesh_asset_id;
                let mesh2d_transforms = &mesh_instance.transforms;
                // Get our specialized pipeline
                let mut mesh2d_key = mesh_key;
                if let Some(mesh) = render_meshes.get(mesh2d_handle) {
                    mesh2d_key |=
                        Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology());
                }

                let pipeline_id =
                    pipelines.specialize(&pipeline_cache, &colored_mesh2d_pipeline, mesh2d_key);

                let mesh_z = mesh2d_transforms.world_from_local.translation.z;
                transparent_phase.add(Transparent2d {
                    entity: *visible_entity,
                    draw_function: draw_colored_mesh2d,
                    pipeline: pipeline_id,
                    // The 2d render items are sorted according to their z value before rendering,
                    // in order to get correct transparency
                    sort_key: FloatOrd(mesh_z),
                    // This material is not batched
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::NONE,
                });
            }
        }
    }
}
