//! Demonstrates how to enqueue custom draw commands in a render phase.
//!
//! This example shows how to use the built-in
//! [`bevy_render::render_phase::BinnedRenderPhase`] functionality with a
//! custom [`RenderCommand`] to allow inserting arbitrary GPU drawing logic
//! into Bevy's pipeline. This is not the only way to add custom rendering code
//! into Bevy—render nodes are another, lower-level method—but it does allow
//! for better reuse of parts of Bevy's built-in mesh rendering logic.

use std::mem;

use bevy::{
    core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT},
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::vec3,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, CachedRenderPipelinePhaseItem, DrawFunctions,
            PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            BufferUsages, CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, FragmentState, IndexFormat, MultisampleState, PipelineCache,
            PrimitiveState, RawBufferVec, RenderPipelineDescriptor, SpecializedRenderPipeline,
            SpecializedRenderPipelines, TextureFormat, VertexAttribute, VertexBufferLayout,
            VertexFormat, VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault as _,
        view::ExtractedView,
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

/// A marker component that represents an entity that is to be rendered using
/// our custom phase item.
///
/// Note the [`ExtractComponent`] trait implementation. This is necessary to
/// tell Bevy that this object should be pulled into the render world.
#[derive(Clone, Component, ExtractComponent)]
struct CustomRenderedEntity;

/// A specialized render pipeline for our custom phase.
#[derive(Component)]
struct CustomPhaseRenderPipeline {
    pipeline_id: CachedRenderPipelineId,
}

/// Holds a reference to our shader.
///
/// This is loaded at app creation time.
#[derive(Resource)]
struct CustomPhasePipeline {
    shader: Handle<Shader>,
}

/// A [`RenderCommand`] that sets the
/// [`bevy_render::render_resource::RenderPipeline`] corresponding to our custom
/// phase item.
struct SetCustomPhaseItemPipeline;

impl<P> RenderCommand<P> for SetCustomPhaseItemPipeline
where
    P: CachedRenderPipelinePhaseItem,
{
    type Param = SRes<PipelineCache>;

    type ViewQuery = ();

    type ItemQuery = Read<CustomPhaseRenderPipeline>;

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        maybe_custom_phase_render_pipeline_id: Option<ROQueryItem<'w, Self::ItemQuery>>,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // Borrow check workaround.
        let pipeline_cache = pipeline_cache.into_inner();

        // The render pipeline needs to be in the pipeline cache for this to
        // succeed.
        let Some(custom_phase_render_pipeline_id) = maybe_custom_phase_render_pipeline_id else {
            return RenderCommandResult::Failure;
        };
        let Some(custom_phase_render_pipeline) =
            pipeline_cache.get_render_pipeline(custom_phase_render_pipeline_id.pipeline_id)
        else {
            return RenderCommandResult::Failure;
        };

        pass.set_render_pipeline(custom_phase_render_pipeline);
        RenderCommandResult::Success
    }
}

/// A [`RenderCommand`] that binds the vertex and index buffers and issues the
/// draw command for our custom phase item.
struct DrawCustomPhaseItem;

impl<P> RenderCommand<P> for DrawCustomPhaseItem
where
    P: PhaseItem,
{
    type Param = SRes<CustomPhaseItemBuffers>;

    type ViewQuery = ();

    type ItemQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        custom_phase_item_buffers: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // Borrow check workaround.
        let custom_phase_item_buffers = custom_phase_item_buffers.into_inner();

        // Tell the GPU where the vertices are.
        pass.set_vertex_buffer(
            0,
            custom_phase_item_buffers
                .vertices
                .buffer()
                .unwrap()
                .slice(..),
        );

        // Tell the GPU where the indices are.
        pass.set_index_buffer(
            custom_phase_item_buffers
                .indices
                .buffer()
                .unwrap()
                .slice(..),
            0,
            IndexFormat::Uint32,
        );

        // Draw one triangle (3 vertices).
        pass.draw_indexed(0..3, 0, 0..1);

        RenderCommandResult::Success
    }
}

/// The GPU vertex and index buffers for our custom phase item.
///
/// As the custom phase item is a single triangle, these are uploaded once and
/// then left alone.
#[derive(Resource)]
struct CustomPhaseItemBuffers {
    /// The vertices for the single triangle.
    ///
    /// This is a [`RawBufferVec`] because that's the simplest and fastest type
    /// of GPU buffer, and [`Vertex`] objects are simple.
    vertices: RawBufferVec<Vertex>,

    /// The indices of the single triangle.
    ///
    /// As above, this is a [`RawBufferVec`] because `u32` values have trivial
    /// size and alignment.
    indices: RawBufferVec<u32>,
}

/// The CPU-side structure that describes a single vertex of the triangle.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    /// The 3D position of the triangle vertex.
    position: Vec3,
    /// Padding.
    pad0: u32,
    /// The color of the triangle vertex.
    color: Vec3,
    /// Padding.
    pad1: u32,
}

impl Vertex {
    /// Creates a new vertex structure.
    const fn new(position: Vec3, color: Vec3) -> Vertex {
        Vertex {
            position,
            color,
            pad0: 0,
            pad1: 0,
        }
    }
}

/// The custom draw commands that Bevy executes for each entity we enqueue into
/// the render phase.
type DrawCustomPhaseItemCommands = (SetCustomPhaseItemPipeline, DrawCustomPhaseItem);

/// A single triangle's worth of vertices, for demonstration purposes.
static VERTICES: [Vertex; 3] = [
    Vertex::new(vec3(-0.866, -0.5, 0.5), vec3(1.0, 0.0, 0.0)),
    Vertex::new(vec3(0.866, -0.5, 0.5), vec3(0.0, 1.0, 0.0)),
    Vertex::new(vec3(0.0, 1.0, 0.5), vec3(0.0, 0.0, 1.0)),
];

/// The entry point.
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(ExtractComponentPlugin::<CustomRenderedEntity>::default())
        .add_systems(Startup, setup);

    // We make sure to add these to the render app, not the main app.
    app.get_sub_app_mut(RenderApp)
        .unwrap()
        .init_resource::<CustomPhasePipeline>()
        .init_resource::<SpecializedRenderPipelines<CustomPhasePipeline>>()
        .add_render_command::<Opaque3d, DrawCustomPhaseItemCommands>()
        .add_systems(
            Render,
            prepare_custom_phase_item_buffers.in_set(RenderSet::Prepare),
        )
        .add_systems(Render, queue_custom_phase_item.in_set(RenderSet::Queue));

    app.run();
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands) {
    // Spawn a single entity that has custom rendering. It'll be extracted into
    // the render world via [`ExtractComponent`].
    commands.spawn(CustomRenderedEntity);

    // Spawn the camera.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// Creates the [`CustomPhaseItemBuffers`] resource.
///
/// This must be done in a startup system because it needs the [`RenderDevice`]
/// and [`RenderQueue`] to exist, and they don't until [`App::run`] is called.
fn prepare_custom_phase_item_buffers(mut commands: Commands) {
    commands.init_resource::<CustomPhaseItemBuffers>();
}

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
#[allow(clippy::too_many_arguments)]
fn queue_custom_phase_item(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    custom_phase_pipeline: Res<CustomPhasePipeline>,
    msaa: Res<Msaa>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    mut specialized_render_pipelines: ResMut<SpecializedRenderPipelines<CustomPhasePipeline>>,
    views: Query<Entity, With<ExtractedView>>,
    custom_rendered_entities: Query<Entity, With<CustomRenderedEntity>>,
) {
    let draw_custom_phase_item = opaque_draw_functions
        .read()
        .id::<DrawCustomPhaseItemCommands>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for view_entity in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view_entity) else {
            continue;
        };

        // Find all the custom rendered entities.
        for entity in custom_rendered_entities.iter() {
            // Ordinarily, the [`SpecializedRenderPipeline::Key`] would contain
            // some per-view settings, such as whether the view is HDR, but for
            // simplicity's sake we simply hard-code the view's characteristics,
            // with the exception of number of MSAA samples.
            let pipeline_id = specialized_render_pipelines.specialize(
                &pipeline_cache,
                &custom_phase_pipeline,
                *msaa,
            );

            commands
                .entity(entity)
                .insert(CustomPhaseRenderPipeline { pipeline_id });

            // Add the custom render item. We use the
            // [`BinnedRenderPhaseType::NonMesh`] type to skip the special
            // handling that Bevy has for meshes (preprocessing, indirect
            // draws, etc.)
            //
            // The asset ID is arbitrary; we simply use [`AssetId::invalid`],
            // but you can use anything you like. Note that the asset ID need
            // not be the ID of a [`Mesh`].
            opaque_phase.add(
                Opaque3dBinKey {
                    draw_function: draw_custom_phase_item,
                    pipeline: pipeline_id,
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                    material_bind_group_id: None,
                    lightmap_image: None,
                },
                entity,
                BinnedRenderPhaseType::NonMesh,
            );
        }
    }
}

impl SpecializedRenderPipeline for CustomPhasePipeline {
    type Key = Msaa;

    fn specialize(&self, msaa: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("custom render pipeline".into()),
            layout: vec![],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    // This needs to match the layout of [`Vertex`].
                    attributes: vec![
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 16,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    // Ordinarily, you'd want to check whether the view has the
                    // HDR format and substitute the appropriate texture format
                    // here, but we omit that for simplicity.
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            // Note that if your view has no depth buffer this will need to be
            // changed.
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: default(),
                bias: default(),
            }),
            multisample: MultisampleState {
                count: msaa.samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}

impl FromWorld for CustomPhaseItemBuffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        // Create the vertex and index buffers.
        let mut vbo = RawBufferVec::new(BufferUsages::VERTEX);
        let mut ibo = RawBufferVec::new(BufferUsages::INDEX);

        for vertex in &VERTICES {
            vbo.push(*vertex);
        }
        for index in 0..3 {
            ibo.push(index);
        }

        // These two lines are required in order to trigger the upload to GPU.
        vbo.write_buffer(render_device, render_queue);
        ibo.write_buffer(render_device, render_queue);

        CustomPhaseItemBuffers {
            vertices: vbo,
            indices: ibo,
        }
    }
}

impl FromWorld for CustomPhasePipeline {
    fn from_world(world: &mut World) -> Self {
        // Load and compile the shader in the background.
        let asset_server = world.resource::<AssetServer>();

        CustomPhasePipeline {
            shader: asset_server.load("shaders/custom_phase_item.wgsl"),
        }
    }
}
