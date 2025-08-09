//! Demonstrates how to enqueue custom draw commands in a render phase.
//!
//! This example shows how to use the built-in
//! [`bevy_render::render_phase::BinnedRenderPhase`] functionality with a
//! custom [`RenderCommand`] to allow inserting arbitrary GPU drawing logic
//! into Bevy's pipeline. This is not the only way to add custom rendering code
//! into Bevy—render nodes are another, lower-level method—but it does allow
//! for better reuse of parts of Bevy's built-in mesh rendering logic.

use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{self, VisibilityClass},
    },
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT},
    ecs::{
        component::Tick,
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            BufferUsages, Canonical, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, FragmentState, IndexFormat, PipelineCache, RawBufferVec,
            RenderPipeline, RenderPipelineDescriptor, Specializer, SpecializerKey, TextureFormat,
            Variants, VertexAttribute, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, RenderVisibleEntities},
        Render, RenderApp, RenderSystems,
    },
};
use bytemuck::{Pod, Zeroable};

/// A marker component that represents an entity that is to be rendered using
/// our custom phase item.
///
/// Note the [`ExtractComponent`] trait implementation: this is necessary to
/// tell Bevy that this object should be pulled into the render world. Also note
/// the `on_add` hook, which is needed to tell Bevy's `check_visibility` system
/// that entities with this component need to be examined for visibility.
#[derive(Clone, Component, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<CustomRenderedEntity>)]
struct CustomRenderedEntity;

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
        _: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
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
type DrawCustomPhaseItemCommands = (SetItemPipeline, DrawCustomPhaseItem);

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
    app.sub_app_mut(RenderApp)
        .init_resource::<CustomPhasePipeline>()
        .add_render_command::<Opaque3d, DrawCustomPhaseItemCommands>()
        .add_systems(
            Render,
            prepare_custom_phase_item_buffers.in_set(RenderSystems::Prepare),
        )
        .add_systems(Render, queue_custom_phase_item.in_set(RenderSystems::Queue));

    app.run();
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands) {
    // Spawn a single entity that has custom rendering. It'll be extracted into
    // the render world via [`ExtractComponent`].
    commands.spawn((
        Visibility::default(),
        Transform::default(),
        // This `Aabb` is necessary for the visibility checks to work.
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(0.5),
        },
        CustomRenderedEntity,
    ));

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
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
fn queue_custom_phase_item(
    pipeline_cache: Res<PipelineCache>,
    mut pipeline: ResMut<CustomPhasePipeline>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa)>,
    mut next_tick: Local<Tick>,
) {
    let draw_custom_phase_item = opaque_draw_functions
        .read()
        .id::<DrawCustomPhaseItemCommands>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (view, view_visible_entities, msaa) in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        // Find all the custom rendered entities that are visible from this
        // view.
        for &entity in view_visible_entities.get::<CustomRenderedEntity>().iter() {
            // Ordinarily, the [`SpecializedRenderPipeline::Key`] would contain
            // some per-view settings, such as whether the view is HDR, but for
            // simplicity's sake we simply hard-code the view's characteristics,
            // with the exception of number of MSAA samples.
            let Ok(pipeline_id) = pipeline
                .variants
                .specialize(&pipeline_cache, CustomPhaseKey(*msaa))
            else {
                continue;
            };

            // Bump the change tick in order to force Bevy to rebuild the bin.
            let this_tick = next_tick.get() + 1;
            next_tick.set(this_tick);

            // Add the custom render item. We use the
            // [`BinnedRenderPhaseType::NonMesh`] type to skip the special
            // handling that Bevy has for meshes (preprocessing, indirect
            // draws, etc.)
            //
            // The asset ID is arbitrary; we simply use [`AssetId::invalid`],
            // but you can use anything you like. Note that the asset ID need
            // not be the ID of a [`Mesh`].
            opaque_phase.add(
                Opaque3dBatchSetKey {
                    draw_function: draw_custom_phase_item,
                    pipeline: pipeline_id,
                    material_bind_group_index: None,
                    lightmap_slab: None,
                    vertex_slab: default(),
                    index_slab: None,
                },
                Opaque3dBinKey {
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                },
                entity,
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
                *next_tick,
            );
        }
    }
}

struct CustomPhaseSpecializer;

#[derive(Resource)]
struct CustomPhasePipeline {
    /// the `variants` collection holds onto the shader handle through the base descriptor
    variants: Variants<RenderPipeline, CustomPhaseSpecializer>,
}

impl FromWorld for CustomPhasePipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/custom_phase_item.wgsl");

        let base_descriptor = RenderPipelineDescriptor {
            label: Some("custom render pipeline".into()),
            vertex: VertexState {
                shader: shader.clone(),
                buffers: vec![VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
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
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                targets: vec![Some(ColorTargetState {
                    // Ordinarily, you'd want to check whether the view has the
                    // HDR format and substitute the appropriate texture format
                    // here, but we omit that for simplicity.
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            // Note that if your view has no depth buffer this will need to be
            // changed.
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: default(),
                bias: default(),
            }),
            ..default()
        };

        let variants = Variants::new(CustomPhaseSpecializer, base_descriptor);

        Self { variants }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
struct CustomPhaseKey(Msaa);

impl Specializer<RenderPipeline> for CustomPhaseSpecializer {
    type Key = CustomPhaseKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.multisample.count = key.0.samples();
        Ok(key)
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
