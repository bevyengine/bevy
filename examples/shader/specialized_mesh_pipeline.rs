//! Demonstrates how to define and use specialized mesh pipeline
//!
//! This example shows how to use the built-in [`SpecializedMeshPipeline`]
//! functionality with a custom [`RenderCommand`] to allow custom mesh rendering with
//! more flexibility than the material api.
//!
//! [`SpecializedMeshPipeline`] let's you customize the entire pipeline used when rendering a mesh.

use bevy::{
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT},
    ecs::{component::Tick, system::StaticSystemParam},
    math::{vec3, vec4},
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey, RenderMeshInstances,
        SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        batching::{
            gpu_preprocessing::{
                self, PhaseBatchedInstanceBuffers, PhaseIndirectParametersBuffers,
                PreprocessWorkItem, UntypedPhaseBatchedInstanceBuffers,
            },
            GetBatchData, GetFullBatchData,
        },
        experimental::occlusion_culling::OcclusionCulling,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology, RenderMesh},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, SetItemPipeline,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Face, FragmentState,
            FrontFace, MultisampleState, PipelineCache, PolygonMode, PrimitiveState,
            RenderPipelineDescriptor, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines, TextureFormat, VertexState,
        },
        view::NoIndirectDrawing,
        view::{self, ExtractedView, RenderVisibleEntities, ViewTarget, VisibilityClass},
        Render, RenderApp, RenderSet,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/specialized_mesh_pipeline.wgsl";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CustomRenderedMeshPipelinePlugin)
        .add_systems(Startup, setup)
        .run();
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // Build a custom triangle mesh with colors
    // We define a custom mesh because the examples only uses a limited
    // set of vertex attributes for simplicity
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2]))
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            vec3(-0.5, -0.5, 0.0),
            vec3(0.5, -0.5, 0.0),
            vec3(0.0, 0.25, 0.0),
        ],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_COLOR,
        vec![
            vec4(1.0, 0.0, 0.0, 1.0),
            vec4(0.0, 1.0, 0.0, 1.0),
            vec4(0.0, 0.0, 1.0, 1.0),
        ],
    );

    // spawn 3 triangles to show that batching works
    for (x, y) in [-0.5, 0.0, 0.5].into_iter().zip([-0.25, 0.5, -0.25]) {
        // Spawn an entity with all the required components for it to be rendered with our custom pipeline
        commands.spawn((
            // We use a marker component to identify the mesh that will be rendered
            // with our specialized pipeline
            CustomRenderedEntity,
            // We need to add the mesh handle to the entity
            Mesh3d(meshes.add(mesh.clone())),
            Transform::from_xyz(x, y, 0.0),
        ));
    }

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        // Move the camera back a bit to see all the triangles
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// When writing custom rendering code it's generally recommended to use a plugin.
// The main reason for this is that it gives you access to the finish() hook
// which is called after rendering resources are initialized.
struct CustomRenderedMeshPipelinePlugin;
impl Plugin for CustomRenderedMeshPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<CustomRenderedEntity>::default());

        // We make sure to add these to the render app, not the main app.
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            // This is needed to tell bevy about your custom pipeline
            .init_resource::<SpecializedMeshPipelines<CustomMeshPipeline>>()
            // We need to use a custom draw command so we need to register it
            .add_render_command::<Opaque3d, DrawSpecializedPipelineCommands>()
            .add_systems(Render, queue_custom_mesh_pipeline.in_set(RenderSet::Queue));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        // Creating this pipeline needs the RenderDevice and RenderQueue
        // which are only available once rendering plugins are initialized.
        render_app.init_resource::<CustomMeshPipeline>();
    }
}

/// A marker component that represents an entity that is to be rendered using
/// our specialized pipeline.
///
/// Note the [`ExtractComponent`] trait implementation: this is necessary to
/// tell Bevy that this object should be pulled into the render world. Also note
/// the `on_add` hook, which is needed to tell Bevy's `check_visibility` system
/// that entities with this component need to be examined for visibility.
#[derive(Clone, Component, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = view::add_visibility_class::<CustomRenderedEntity>)]
struct CustomRenderedEntity;

/// The custom draw commands that Bevy executes for each entity we enqueue into
/// the render phase.
type DrawSpecializedPipelineCommands = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform at bind group 0
    SetMeshViewBindGroup<0>,
    // Set the mesh uniform at bind group 1
    SetMeshBindGroup<1>,
    // Draw the mesh
    DrawMesh,
);

// This contains the state needed to specialize a mesh pipeline
#[derive(Resource)]
struct CustomMeshPipeline {
    /// The base mesh pipeline defined by bevy
    ///
    /// This isn't required, but if you want to use a bevy `Mesh` it's easier when you
    /// have access to the base `MeshPipeline` that bevy already defines
    mesh_pipeline: MeshPipeline,
    /// Stores the shader used for this pipeline directly on the pipeline.
    /// This isn't required, it's only done like this for simplicity.
    shader_handle: Handle<Shader>,
}
impl FromWorld for CustomMeshPipeline {
    fn from_world(world: &mut World) -> Self {
        // Load the shader
        let shader_handle: Handle<Shader> = world.resource::<AssetServer>().load(SHADER_ASSET_PATH);
        Self {
            mesh_pipeline: MeshPipeline::from_world(world),
            shader_handle,
        }
    }
}

impl SpecializedMeshPipeline for CustomMeshPipeline {
    /// Pipeline use keys to determine how to specialize it.
    /// The key is also used by the pipeline cache to determine if
    /// it needs to create a new pipeline or not
    ///
    /// In this example we just use the base `MeshPipelineKey` defined by bevy, but this could be anything.
    /// For example, if you want to make a pipeline with a procedural shader you could add the Handle<Shader> to the key.
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        mesh_key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        // Define the vertex attributes based on a standard bevy [`Mesh`]
        let mut vertex_attributes = Vec::new();
        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            // Make sure this matches the shader location
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }
        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            // Make sure this matches the shader location
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(1));
        }
        // This will automatically generate the correct `VertexBufferLayout` based on the vertex attributes
        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        Ok(RenderPipelineDescriptor {
            label: Some("Specialized Mesh Pipeline".into()),
            layout: vec![
                // Bind group 0 is the view uniform
                self.mesh_pipeline
                    .get_view_layout(MeshPipelineViewLayoutKey::from(mesh_key))
                    .clone(),
                // Bind group 1 is the mesh uniform
                self.mesh_pipeline.mesh_layouts.model_only.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                // Customize how to store the meshes' vertex attributes in the vertex buffer
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    // This isn't required, but bevy supports HDR and non-HDR rendering
                    // so it's generally recommended to specialize the pipeline for that
                    format: if mesh_key.contains(MeshPipelineKey::HDR) {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    // For this example we only use opaque meshes,
                    // but if you wanted to use alpha blending you would need to set it here
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: mesh_key.primitive_topology(),
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                ..default()
            },
            // Note that if your view has no depth buffer this will need to be
            // changed.
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: default(),
                bias: default(),
            }),
            // It's generally recommended to specialize your pipeline for MSAA,
            // but it's not always possible
            multisample: MultisampleState {
                count: mesh_key.msaa_samples(),
                ..MultisampleState::default()
            },
            zero_initialize_workgroup_memory: false,
        })
    }
}

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
fn queue_custom_mesh_pipeline(
    pipeline_cache: Res<PipelineCache>,
    custom_mesh_pipeline: Res<CustomMeshPipeline>,
    (mut opaque_render_phases, opaque_draw_functions): (
        ResMut<ViewBinnedRenderPhases<Opaque3d>>,
        Res<DrawFunctions<Opaque3d>>,
    ),
    mut specialized_mesh_pipelines: ResMut<SpecializedMeshPipelines<CustomMeshPipeline>>,
    views: Query<(
        &RenderVisibleEntities,
        &ExtractedView,
        &Msaa,
        Has<NoIndirectDrawing>,
        Has<OcclusionCulling>,
    )>,
    (render_meshes, render_mesh_instances): (
        Res<RenderAssets<RenderMesh>>,
        Res<RenderMeshInstances>,
    ),
    param: StaticSystemParam<<MeshPipeline as GetBatchData>::Param>,
    mut phase_batched_instance_buffers: ResMut<
        PhaseBatchedInstanceBuffers<Opaque3d, <MeshPipeline as GetBatchData>::BufferData>,
    >,
    mut phase_indirect_parameters_buffers: ResMut<PhaseIndirectParametersBuffers<Opaque3d>>,
    mut change_tick: Local<Tick>,
) {
    let system_param_item = param.into_inner();

    let UntypedPhaseBatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ref mut late_indexed_indirect_parameters_buffer,
        ref mut late_non_indexed_indirect_parameters_buffer,
        ..
    } = phase_batched_instance_buffers.buffers;

    // Get the id for our custom draw function
    let draw_function_id = opaque_draw_functions
        .read()
        .id::<DrawSpecializedPipelineCommands>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (view_visible_entities, view, msaa, no_indirect_drawing, gpu_occlusion_culling) in
        views.iter()
    {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        // Create *work item buffers* if necessary. Work item buffers store the
        // indices of meshes that are to be rendered when indirect drawing is
        // enabled.
        let work_item_buffer = gpu_preprocessing::get_or_create_work_item_buffer::<Opaque3d>(
            work_item_buffers,
            view.retained_view_entity,
            no_indirect_drawing,
            gpu_occlusion_culling,
        );

        // Initialize those work item buffers in preparation for this new frame.
        gpu_preprocessing::init_work_item_buffers(
            work_item_buffer,
            late_indexed_indirect_parameters_buffer,
            late_non_indexed_indirect_parameters_buffer,
        );

        // Create the key based on the view. In this case we only care about MSAA and HDR
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        // Set up a slot to hold information about the batch set we're going to
        // create. If there are any of our custom meshes in the scene, we'll
        // need this information in order for Bevy to kick off the rendering.
        let mut mesh_batch_set_info = None;

        // Find all the custom rendered entities that are visible from this
        // view.
        for &(render_entity, visible_entity) in
            view_visible_entities.get::<CustomRenderedEntity>().iter()
        {
            // Get the mesh instance
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(visible_entity)
            else {
                continue;
            };

            // Get the mesh data
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            // Specialize the key for the current mesh entity
            // For this example we only specialize based on the mesh topology
            // but you could have more complex keys and that's where you'd need to create those keys
            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            // Initialize the batch set information if this was the first custom
            // mesh we saw. We'll need that information later to create the
            // batch set.
            if mesh_batch_set_info.is_none() {
                mesh_batch_set_info = Some(MeshBatchSetInfo {
                    indirect_parameters_index: phase_indirect_parameters_buffers
                        .buffers
                        .allocate(mesh.indexed(), 1),
                    is_indexed: mesh.indexed(),
                });
            }
            let mesh_info = mesh_batch_set_info.unwrap();

            // Allocate some input and output indices. We'll need these to
            // create the *work item* below.
            let Some(input_index) =
                MeshPipeline::get_binned_index(&system_param_item, visible_entity)
            else {
                continue;
            };
            let output_index = data_buffer.add() as u32;

            // Finally, we can specialize the pipeline based on the key
            let pipeline_id = specialized_mesh_pipelines
                .specialize(
                    &pipeline_cache,
                    &custom_mesh_pipeline,
                    mesh_key,
                    &mesh.layout,
                )
                // This should never with this example, but if your pipeline specialization
                // can fail you need to handle the error here
                .expect("Failed to specialize mesh pipeline");

            // Bump the change tick so that Bevy is forced to rebuild the bin.
            let next_change_tick = change_tick.get() + 1;
            change_tick.set(next_change_tick);

            // Add the mesh with our specialized pipeline
            opaque_phase.add(
                Opaque3dBatchSetKey {
                    draw_function: draw_function_id,
                    pipeline: pipeline_id,
                    material_bind_group_index: None,
                    vertex_slab: default(),
                    index_slab: None,
                    lightmap_slab: None,
                },
                // The asset ID is arbitrary; we simply use [`AssetId::invalid`],
                // but you can use anything you like. Note that the asset ID need
                // not be the ID of a [`Mesh`].
                Opaque3dBinKey {
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                },
                (render_entity, visible_entity),
                mesh_instance.current_uniform_index,
                // This example supports batching, but if your pipeline doesn't
                // support it you can use `BinnedRenderPhaseType::UnbatchableMesh`
                BinnedRenderPhaseType::BatchableMesh,
                *change_tick,
            );

            // Create a *work item*. A work item tells the Bevy renderer to
            // transform the mesh on GPU.
            work_item_buffer.push(
                mesh.indexed(),
                PreprocessWorkItem {
                    input_index: input_index.into(),
                    output_or_indirect_parameters_index: if no_indirect_drawing {
                        output_index
                    } else {
                        mesh_info.indirect_parameters_index
                    },
                },
            );
        }

        // Now if there were any meshes, we need to add a command to the
        // indirect parameters buffer, so that the renderer will end up
        // enqueuing a command to draw the mesh.
        if let Some(mesh_info) = mesh_batch_set_info {
            phase_indirect_parameters_buffers
                .buffers
                .add_batch_set(mesh_info.is_indexed, mesh_info.indirect_parameters_index);
        }
    }
}

// If we end up having any custom meshes to draw, this contains information
// needed to create the batch set.
#[derive(Clone, Copy)]
struct MeshBatchSetInfo {
    /// The first index of the mesh batch in the indirect parameters buffer.
    indirect_parameters_index: u32,
    /// Whether the mesh is indexed (has an index buffer).
    is_indexed: bool,
}
