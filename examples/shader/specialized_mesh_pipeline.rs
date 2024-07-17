//! Demonstrates how to define and use specialized mesh pipeline
//!
//! This example shows how to use the built-in [`SpecializedMeshPipeline`]
//! functionality with a custom [`RenderCommand`] to allow custom mesh rendering with
//! more flexibility than the material api.
//!
//! [`SpecializedMeshPipeline`] let's you customize the entire pipeline used when rendering a mesh.

use bevy::{
    core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT},
    math::{vec3, vec4},
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey, RenderMeshInstances,
        SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
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
        texture::BevyDefault as _,
        view::{self, ExtractedView, ViewTarget, VisibilitySystems, VisibleEntities},
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
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
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
            meshes.add(mesh.clone()),
            // This bundle's components are needed for something to be rendered
            SpatialBundle {
                transform: Transform::from_xyz(x, y, 0.0),
                ..SpatialBundle::INHERITED_IDENTITY
            },
        ));
    }

    // Spawn the camera.
    commands.spawn(Camera3dBundle {
        // Move the camera back a bit to see all the triangles
        transform: Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

// When writing custom rendering code it's generally recommended to use a plugin.
// The main reason for this is that it gives you access to the finish() hook
// which is called after rendering resources are initialized.
struct CustomRenderedMeshPipelinePlugin;
impl Plugin for CustomRenderedMeshPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<CustomRenderedEntity>::default())
            .add_systems(
                PostUpdate,
                // Make sure to tell Bevy to check our entity for visibility. Bevy won't
                // do this by default, for efficiency reasons.
                // This will do things like frustum culling and hierarchy visibility
                view::check_visibility::<WithCustomRenderedEntity>
                    .in_set(VisibilitySystems::CheckVisibility),
            );

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
/// Note the [`ExtractComponent`] trait implementation. This is necessary to
/// tell Bevy that this object should be pulled into the render world.
#[derive(Clone, Component, ExtractComponent)]
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

/// A query filter that tells [`view::check_visibility`] about our custom
/// rendered entity.
type WithCustomRenderedEntity = With<CustomRenderedEntity>;

// This contains the state needed to speciazlize a mesh pipeline
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
            // This is isn't required, but if you can support MSAA
            // it's generally recommended to specialize your pipeline for it
            multisample: MultisampleState {
                count: mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        })
    }
}

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
#[allow(clippy::too_many_arguments)]
fn queue_custom_mesh_pipeline(
    pipeline_cache: Res<PipelineCache>,
    custom_mesh_pipeline: Res<CustomMeshPipeline>,
    msaa: Res<Msaa>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    mut specialized_mesh_pipelines: ResMut<SpecializedMeshPipelines<CustomMeshPipeline>>,
    views: Query<(Entity, &VisibleEntities, &ExtractedView), With<ExtractedView>>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
) {
    // Get the id for our custom draw function
    let draw_function_id = opaque_draw_functions
        .read()
        .id::<DrawSpecializedPipelineCommands>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (view_entity, view_visible_entities, view) in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view_entity) else {
            continue;
        };

        // Create the key based on the view. In this case we only care about MSAA and HDR
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        // Find all the custom rendered entities that are visible from this
        // view.
        for &visible_entity in view_visible_entities
            .get::<WithCustomRenderedEntity>()
            .iter()
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

            // Add the mesh with our specialized pipeline
            opaque_phase.add(
                Opaque3dBinKey {
                    draw_function: draw_function_id,
                    pipeline: pipeline_id,
                    // The asset ID is arbitrary; we simply use [`AssetId::invalid`],
                    // but you can use anything you like. Note that the asset ID need
                    // not be the ID of a [`Mesh`].
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                    material_bind_group_id: None,
                    lightmap_image: None,
                },
                visible_entity,
                // This example supports batching, but if your pipeline doesn't
                // support it you can use `BinnedRenderPhaseType::UnbatchableMesh`
                BinnedRenderPhaseType::BatchableMesh,
            );
        }
    }
}
