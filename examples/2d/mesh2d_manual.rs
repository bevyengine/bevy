//! This example shows how to manually render 2d items using "mid level render apis" with a custom
//! pipeline for 2d meshes.
//! It doesn't use the [`Material2d`] abstraction, but changes the vertex buffer to include vertex color.
//! Check out the "mesh2d" example for simpler / higher level 2d meshes.
//!
//! [`Material2d`]: bevy::sprite_render::Material2d

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::basic::YELLOW,
    core_pipeline::{core_2d::CORE_2D_DEPTH_FORMAT, Core2d, Core2dSystems},
    ecs::{
        entity::EntityHash,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::{ops, FloatOrd},
    mesh::{BaseMeshPipelineKey, Indices, MeshVertexAttribute, VertexBufferLayout},
    platform::collections::HashSet,
    prelude::*,
    render::{
        batching::{no_gpu_preprocessing::batch_and_prepare_sorted_render_phase, GetBatchData},
        camera::ExtractedCamera,
        diagnostic::RecordDiagnostics as _,
        material_bind_groups::{MaterialBindGroupIndex, MaterialBindGroupSlot, MaterialBindingId},
        mesh::{
            allocator::MeshAllocator, MeshMetadataFallbackBuffer, RenderMesh, RenderMeshBufferInfo,
        },
        render_asset::RenderAssets,
        render_phase::{
            sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
            DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
            SetItemPipeline, SortedPhaseItem, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, Face, FragmentState, MultisampleState,
            PipelineCache, PrimitiveState, PrimitiveTopology, RenderPassDescriptor,
            RenderPipelineDescriptor, SpecializedRenderPipeline, SpecializedRenderPipelines,
            StencilFaceState, StencilState, StoreOp, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::{RenderContext, ViewQuery},
        sync_component::{SyncComponent, SyncComponentPlugin},
        sync_world::{MainEntity, MainEntityHashMap, RenderEntity},
        view::{
            ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewDepthStencilTexture,
            ViewTarget,
        },
        Extract, Render, RenderApp, RenderStartup, RenderSystems,
    },
    sprite_render::{
        extract_mesh2d, init_mesh_2d_pipeline, Mesh2dBindGroup, Mesh2dPipeline, Mesh2dPipelineKey,
        Mesh2dTransforms, Mesh2dUniform, MeshFlags, RenderMesh2dInstance, SetMesh2dViewBindGroup,
    },
};
use indexmap::IndexMap;
use std::{f32::consts::PI, ops::Range};

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
        v_pos.push([r * ops::sin(a), r * ops::cos(a), 0.0]);
    }
    // Set the position attribute
    star.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);
    // And a RGB color attribute as well. A built-in `Mesh::ATTRIBUTE_COLOR` exists, but we
    // use a custom vertex attribute here for demonstration purposes.
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
        // The `Handle<Mesh>` needs to be wrapped in a `Mesh2d` for 2D rendering
        Mesh2d(meshes.add(star)),
    ));

    commands.spawn(Camera2d);
}

/// A marker component for colored 2d meshes
#[derive(Component, Default)]
pub struct ColoredMesh2d;

impl SyncComponent<RenderApp> for ColoredMesh2d {
    type Target = Self;
}

/// Custom pipeline for 2d meshes with vertex colors
#[derive(Resource)]
pub struct ColoredMesh2dPipeline {
    /// This pipeline wraps the standard [`Mesh2dPipeline`]
    mesh2d_pipeline: Mesh2dPipeline,
    /// The shader asset handle.
    shader: Handle<Shader>,
}

fn init_colored_mesh_2d_pipeline(
    mut commands: Commands,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    colored_mesh2d_shader: Res<ColoredMesh2dShader>,
) {
    commands.insert_resource(ColoredMesh2dPipeline {
        mesh2d_pipeline: mesh2d_pipeline.clone(),
        // Clone the shader from the shader resource we inserted in the plugin.
        shader: colored_mesh2d_shader.0.clone(),
    });
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

        let format = key.target_format();

        RenderPipelineDescriptor {
            vertex: VertexState {
                // Use our custom shader
                shader: self.shader.clone(),
                // Use our custom vertex buffer
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                // Use our custom shader
                shader: self.shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            // Use the two standard uniforms for 2d meshes
            layout: vec![
                // Bind group 0 is the view uniform
                self.mesh2d_pipeline.view_layout.clone(),
                // Bind group 1 is the mesh uniform
                self.mesh2d_pipeline.mesh_layout.clone(),
            ],
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                topology: BaseMeshPipelineKey::from_bits_retain(key.bits()).primitive_topology(),
                strip_index_format: BaseMeshPipelineKey::from_bits_retain(key.bits())
                    .strip_index_format(),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_2D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("colored_mesh2d_pipeline".into()),
            ..default()
        }
    }
}

// This specifies how to render a colored 2d mesh
type DrawTransparentColoredMesh2d = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform as bind group 0
    SetMesh2dViewBindGroup<0>,
    // Set the mesh uniform as bind group 1
    SetColoredMesh2dBindGroup<1>,
    // Draw the mesh
    DrawColoredMesh2d,
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

/// A resource holding the shader asset handle for the pipeline to take. There are many ways to get
/// the shader into the pipeline - this is just one option.
#[derive(Resource)]
struct ColoredMesh2dShader(Handle<Shader>);

/// Our custom pipeline needs its own instance storage
#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenderColoredMesh2dInstances(MainEntityHashMap<RenderMesh2dInstance>);

impl Plugin for ColoredMesh2dPlugin {
    fn build(&self, app: &mut App) {
        // Load our custom shader
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        // Here, we construct and add the shader asset manually. There are many ways to load this
        // shader, including `embedded_asset`/`load_embedded_asset`.
        let shader = shaders.add(Shader::from_wgsl(COLORED_MESH2D_SHADER, file!()));

        app.add_plugins(SyncComponentPlugin::<ColoredMesh2d>::default());

        // Register our custom draw function, and add our render systems
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<DrawFunctions<TransparentColoredMesh2d>>()
            // Declare a render phase, `TransparentColoredMesh2d`, to go with
            // our pipeline.
            .init_resource::<ViewSortedRenderPhases<TransparentColoredMesh2d>>()
            // Declare the pipeline itself.
            .init_resource::<SpecializedRenderPipelines<ColoredMesh2dPipeline>>()
            // Declare the render-world resource that will hold the instances.
            .init_resource::<RenderColoredMesh2dInstances>()
            .insert_resource(ColoredMesh2dShader(shader))
            // Declare a new render command.
            .add_render_command::<TransparentColoredMesh2d, DrawTransparentColoredMesh2d>()
            .add_systems(
                RenderStartup,
                init_colored_mesh_2d_pipeline.after(init_mesh_2d_pipeline),
            )
            .add_systems(
                ExtractSchedule,
                (
                    extract_colored_mesh2d.after(extract_mesh2d),
                    extract_colored_mesh2d_camera_phases,
                ),
            )
            .add_systems(
                Render,
                (
                    sort_phase_system::<TransparentColoredMesh2d>.in_set(RenderSystems::PhaseSort),
                    queue_colored_mesh2d.in_set(RenderSystems::QueueMeshes),
                    // Make sure to prepare the render phase.
                    batch_and_prepare_sorted_render_phase::<
                        TransparentColoredMesh2d,
                        ColoredMesh2dPipeline,
                    >
                        .in_set(RenderSystems::PrepareResources),
                ),
            )
            .add_systems(
                Core2d,
                // Add the draw command to draw the items in our custom phase.
                main_colored_transparent_pass_2d.in_set(Core2dSystems::MainPass),
            );
    }
}

/// Our own [`PhaseItem`].
///
/// Every render phase must be in 1:1 correspondence with a pipeline. Since we
/// have our own custom pipeline, we must also declare a custom render phase to
/// go with it.
struct TransparentColoredMesh2d {
    sort_key: FloatOrd,
    entity: (Entity, MainEntity),
    pipeline: CachedRenderPipelineId,
    draw_function: DrawFunctionId,
    batch_range: Range<u32>,
    extra_index: PhaseItemExtraIndex,
    /// Whether the mesh in question is indexed (uses an index buffer in
    /// addition to its vertex buffer).
    indexed: bool,
}

impl PhaseItem for TransparentColoredMesh2d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
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
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for TransparentColoredMesh2d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut IndexMap<(Entity, MainEntity), TransparentColoredMesh2d, EntityHash>) {
        items.sort_by_key(|_, item| item.sort_key());
    }

    fn recalculate_sort_keys(
        _: &mut IndexMap<(Entity, MainEntity), Self, EntityHash>,
        _: &ExtractedView,
    ) {
        // Sort keys are precalculated for 2D phase items.
    }

    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for TransparentColoredMesh2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

impl GetBatchData for ColoredMesh2dPipeline {
    type Param = (SRes<RenderColoredMesh2dInstances>, SRes<MeshAllocator>);
    type BatchSetCompareData = AssetId<Mesh>;
    type BatchCompareData = Option<MaterialBindGroupIndex>;
    type BufferData = Mesh2dUniform;

    fn get_batch_data(
        (mesh_instances, mesh_allocator): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(
        Self::BufferData,
        Option<(Self::BatchSetCompareData, Self::BatchCompareData)>,
    )> {
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let metadata_index = mesh_allocator
            .mesh_metadata_slice(&mesh_instance.mesh_asset_id)
            .map(|mesh_metadata_slice| mesh_metadata_slice.range.start);

        Some((
            Mesh2dUniform::from_components(
                &mesh_instance.transforms,
                MaterialBindGroupSlot(0),
                mesh_instance.tag,
                metadata_index,
            ),
            mesh_instance
                .automatic_batching
                .then_some((mesh_instance.mesh_asset_id, None)),
        ))
    }
}

/// Prepares our custom render phase for a new frame.
fn extract_colored_mesh2d_camera_phases(
    mut colored_mesh2d_render_phases: ResMut<ViewSortedRenderPhases<TransparentColoredMesh2d>>,
    cameras_2d: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (main_entity, camera) in &cameras_2d {
        if !camera.is_active {
            continue;
        }

        // This is the main 2D camera, so we use the first subview index (0).
        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);

        colored_mesh2d_render_phases.prepare_for_new_frame(retained_view_entity);

        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views.
    colored_mesh2d_render_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

/// Extract the [`ColoredMesh2d`] marker component into the render app
pub fn extract_colored_mesh2d(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    // When extracting, you must use `Extract` to mark the `SystemParam`s
    // which should be taken from the main world.
    query: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                &ViewVisibility,
                &GlobalTransform,
                &Mesh2d,
            ),
            With<ColoredMesh2d>,
        >,
    >,
    mut render_mesh_instances: ResMut<RenderColoredMesh2dInstances>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, render_entity, view_visibility, transform, handle) in &query {
        if !view_visibility.get() {
            continue;
        }

        let transforms = Mesh2dTransforms {
            world_from_local: transform.affine().into(),
            flags: MeshFlags::empty().bits(),
        };

        values.push((render_entity, ColoredMesh2d));
        render_mesh_instances.insert(
            entity.into(),
            RenderMesh2dInstance {
                mesh_asset_id: handle.0.id(),
                transforms,
                // This is unused here.
                material_bindings_index: MaterialBindingId::default(),
                automatic_batching: false,
                tag: 0,
            },
        );
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}

/// Queue the 2d meshes marked with [`ColoredMesh2d`] using our custom pipeline and draw function
fn queue_colored_mesh2d(
    transparent_draw_functions: Res<DrawFunctions<TransparentColoredMesh2d>>,
    colored_mesh2d_pipeline: Res<ColoredMesh2dPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ColoredMesh2dPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderColoredMesh2dInstances>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentColoredMesh2d>>,
    views: Query<(&RenderVisibleEntities, &ExtractedView, &Msaa)>,
) {
    if render_mesh_instances.is_empty() {
        return;
    }

    // Iterate each view (a camera is a view)
    for (visible_entities, view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let draw_colored_mesh2d = transparent_draw_functions
            .read()
            .id::<DrawTransparentColoredMesh2d>();

        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_target_format(view.target_format);

        // Queue all entities visible to that view
        let Some(visible_entities) = visible_entities.get::<Mesh2d>() else {
            continue;
        };
        for (render_entity, visible_entity) in visible_entities.iter_visible() {
            if let Some(mesh_instance) = render_mesh_instances.get(visible_entity) {
                let mesh2d_handle = mesh_instance.mesh_asset_id;
                let mesh2d_transforms = &mesh_instance.transforms;
                // Get our specialized pipeline
                let mut mesh2d_key = mesh_key;
                let Some(mesh) = render_meshes.get(mesh2d_handle) else {
                    continue;
                };
                mesh2d_key |= Mesh2dPipelineKey::from(
                    BaseMeshPipelineKey::from_primitive_topology_and_strip_index(
                        mesh.primitive_topology(),
                        mesh.index_format(),
                    )
                    .bits(),
                );

                let pipeline_id =
                    pipelines.specialize(&pipeline_cache, &colored_mesh2d_pipeline, mesh2d_key);

                let mesh_z = mesh2d_transforms.world_from_local.translation.z;
                transparent_phase.add_retained(TransparentColoredMesh2d {
                    entity: (*render_entity, *visible_entity),
                    draw_function: draw_colored_mesh2d,
                    pipeline: pipeline_id,
                    // The 2d render items are sorted according to their z value before rendering,
                    // in order to get correct transparency
                    sort_key: FloatOrd(mesh_z),
                    // This material is not batched
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::None,
                    indexed: mesh.indexed(),
                });
            }
        }
    }
}

/// The render node system that draws all items in the
/// [`TransparentColoredMesh2d`] phase.
fn main_colored_transparent_pass_2d(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewDepthStencilTexture,
    )>,
    transparent_phases: Res<ViewSortedRenderPhases<TransparentColoredMesh2d>>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (camera, extracted_view, target, depth) = view.into_inner();

    let Some(transparent_phase) = transparent_phases.get(&extracted_view.retained_view_entity)
    else {
        return;
    };

    #[cfg(feature = "trace")]
    let _span = info_span!("main_colored_transparent_pass_2d").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let color_attachments = [Some(target.get_color_attachment())];
    // NOTE: For the transparent pass we load the depth buffer. There should be no
    // need to write to it, but store is set to `true` as a workaround for issue #3776,
    // https://github.com/bevyengine/bevy/issues/3776
    // so that wgpu does not clear the depth buffer.
    // As the opaque and alpha mask passes run first, opaque meshes can occlude
    // transparent ones.
    let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

    {
        let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("main_colored_transparent_pass_2d"),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_pass, "main_colored_transparent_pass_2d");

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        if !transparent_phase.items.is_empty() {
            #[cfg(feature = "trace")]
            let _transparent_span = info_span!("colored_transparent_main_pass_2d").entered();
            if let Err(err) = transparent_phase.render(&mut render_pass, world, view_entity) {
                error!(
                    "Error encountered while rendering the colored transparent 2D phase {err:?}"
                );
            }
        }

        pass_span.end(&mut render_pass);
    }
}

/// The render command that sets the right bind group.
///
/// Since the normal `SetMesh2dBindGroup` render command is hardwired to use
/// `RenderMesh2dInstances`, we need to replace it with our own render command.
struct SetColoredMesh2dBindGroup<const I: usize>;

impl<P, const I: usize> RenderCommand<P> for SetColoredMesh2dBindGroup<I>
where
    P: PhaseItem,
{
    type Param = (
        SRes<Mesh2dBindGroup>,
        SRes<RenderColoredMesh2dInstances>,
        SRes<MeshAllocator>,
        SRes<MeshMetadataFallbackBuffer>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (mesh2d_bind_group, render_mesh2d_instances, mesh_allocator,metadata_fallback_buffer): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();
        let mesh2d_bind_group = mesh2d_bind_group.into_inner();

        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let metadata_slab_id = mesh_allocator
            .key_to_slab
            .get(&bevy_render::mesh::allocator::MeshAllocationKey::new(
                *mesh_asset_id,
                bevy_render::mesh::allocator::ElementClass::Metadata,
            ))
            .cloned()
            .unwrap_or(metadata_fallback_buffer.slab_id);
        let Some(bind_group) = &mesh2d_bind_group.value.get(&metadata_slab_id) else {
            return RenderCommandResult::Failure(
                "The mesh2d bind group wasn't set in the render phase.",
            );
        };

        let mut dynamic_offsets: [u32; 1] = Default::default();
        let mut offset_count = 0;
        if let PhaseItemExtraIndex::DynamicOffset(dynamic_offset) = item.extra_index() {
            dynamic_offsets[offset_count] = dynamic_offset;
            offset_count += 1;
        }
        pass.set_bind_group(I, bind_group, &dynamic_offsets[..offset_count]);
        RenderCommandResult::Success
    }
}

/// The render command that draws all the meshes in our custom render phase.
struct DrawColoredMesh2d;

impl<P: PhaseItem> RenderCommand<P> for DrawColoredMesh2d {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderColoredMesh2dInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (meshes, render_mesh2d_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

        let batch_range = item.batch_range();
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), *index_format);

                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    batch_range.clone(),
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, batch_range.clone());
            }
        }
        RenderCommandResult::Success
    }
}
