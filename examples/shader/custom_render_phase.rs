//! An example showcasing how to set up a custom draw command and render phase
//!
//! The example shader is a very basic shader that just outputs a color at the correct position.
//!
//! This is a fairly low level example and assumes some familiarity with rendering concepts and wgpu.

use bevy::{
    core_pipeline::core_3d::{self, CORE_3D},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
        },
        render_phase::{
            sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
            DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
            CachedRenderPipelineId, LoadOp, Operations, PipelineCache,
            RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
            ShaderStages, ShaderType, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines,
        },
        renderer::{RenderContext, RenderDevice},
        view::{ExtractedView, ViewDepthTexture, ViewTarget, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::FloatOrd,
};

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins((DefaultPlugins, CustomRenderPhasePlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.0, 5.0),
        ..default()
    });

    // Spawn 3 cubes that use the custom draw command
    // Each cube is at a different depth to show that the sorting works correctly

    commands.spawn(CustomMaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: CustomMaterial {
            base_color: Color::RED,
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(CustomMaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: CustomMaterial {
            base_color: Color::GREEN,
        },
        transform: Transform::from_xyz(0.5, 0.5, -1.0),
        ..default()
    });
    commands.spawn(CustomMaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: CustomMaterial {
            base_color: Color::BLUE,
        },
        transform: Transform::from_xyz(1.0, 1.0, -2.0),
        ..default()
    });
}

// Initializing various parts of the render pipeline can be quite complex so it's easier to do it in a separate plugin
pub struct CustomRenderPhasePlugin;
impl Plugin for CustomRenderPhasePlugin {
    fn build(&self, app: &mut App) {
        // The UniformComponentPlugin will set up the necessary system to
        // automatically extract and prepare the given uniform component
        app.add_plugins(UniformComponentPlugin::<CustomMaterialUniform>::default());

        // We need to get the render app from the main app
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            // The CustomPipeline is a specialized mesh pipeline so we need to initialize it
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            // When making a custom phase you need to initialize the DrawFunctions resource for that phase
            .init_resource::<DrawFunctions<CustomPhaseItem>>()
            // You also need to add the custom render command to that phase
            .add_render_command::<CustomPhaseItem, DrawCustom>();

        // The render world
        render_app
            // The extract schedule is the only sync point between the main world and the render world
            // When you need to send data to the render world you need to extract
            // that data and you can only do it in the ExtractSchedule.
            // Some common extract scenarios have plugins that will do this automatically.
            // For the purpose of the example we will do it manually.
            .add_systems(
                ExtractSchedule,
                (extract_render_phase, extract_custom_material_uniform),
            )
            .add_systems(
                Render,
                (
                    // This will automatically sort all items in the phase based on the [`PhaseItem::sort_key()`]
                    sort_phase_system::<CustomPhaseItem>.in_set(RenderSet::PhaseSort),
                    queue_mesh_custom_phase.in_set(RenderSet::Queue),
                    queue_custom_bind_group.in_set(RenderSet::Queue),
                ),
            );

        // Bevy's renderer uses a render graph which is a collection of nodes in a directed acyclic graph.
        // It currently runs on each view/camera and executes each node in the specified order.
        // It will make sure that any node that needs a dependency from another node
        // only runs when that dependency is done.
        //
        // Each node can execute arbitrary work, but it generally runs at least one render pass.
        // A node only has access to the render world, so if you need data from the main world
        // you need to extract it manually or with the plugin like above.
        render_app
            // Add the node that will render the custom phase
            .add_render_graph_node::<ViewNodeRunner<CustomNode>>(CORE_3D, CustomNode::NAME)
            // This will schedule the custom node to run after the main opaque pass
            .add_render_graph_edge(
                CORE_3D,
                core_3d::graph::node::MAIN_OPAQUE_PASS,
                CustomNode::NAME,
            );
    }

    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        // This needs to be after the initial build because it needs a reference to the RenderDevice
        // but it doesn't exist in the build() step
        render_app.init_resource::<CustomPipeline>();
    }
}

// The render node that will render the custom phase
#[derive(Default)]
pub struct CustomNode;
impl CustomNode {
    const NAME: &str = "custom_node";
}

impl ViewNode for CustomNode {
    type ViewQuery = (
        &'static RenderPhase<CustomPhaseItem>,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (custom_phase, target, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        if custom_phase.items.is_empty() {
            return Ok(());
        }

        // The render pass that will be used for by the draw command
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("custom_pass"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        // This will automatically call the draw command on each items in the render phase
        custom_phase.render(&mut render_pass, world, view_entity);

        Ok(())
    }
}

// The bind group of the custom material
#[derive(Resource, Deref)]
pub struct CustomMaterialBindGroup(BindGroup);

// TODO document render command
pub struct SetMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMaterialBindGroup<I> {
    type Param = SRes<CustomMaterialBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<CustomMaterialUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        mesh_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, bind_group.into_inner(), &[mesh_index.index()]);
        RenderCommandResult::Success
    }
}

// The custom draw command
// TODO explain a bit more why it's a type alias
type DrawCustom = (
    // Sets the render pipeline for the draw
    SetItemPipeline,
    // Sets the mesh view bind group at index 0
    SetMeshViewBindGroup<0>,
    // Sets the custom material bind group at index 1
    SetMaterialBindGroup<1>,
    // Sets the mesh bind group at index 2
    SetMeshBindGroup<2>,
    // Draws the mesh with the specified pipeline and bind groups
    DrawMesh,
);

// TODO explain what a PhaseItem is
pub struct CustomPhaseItem {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for CustomPhaseItem {
    type SortKey = FloatOrd;

    fn entity(&self) -> Entity {
        self.entity
    }

    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl CachedRenderPipelinePhaseItem for CustomPhaseItem {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

#[derive(Component, ShaderType, Clone, Copy)]
pub struct CustomMaterialUniform {
    base_color: Color,
}

#[derive(Resource)]
pub struct CustomPipeline {
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("custom_bind_group_layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(CustomMaterialUniform::min_size()),
                    },
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                }],
            });

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/custom_draw.wgsl");

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();
        CustomPipeline {
            shader,
            mesh_pipeline,
            bind_group_layout,
        }
    }
}

// TODO explain what SpecializedMeshPipeline are
impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;
    fn specialize(
        &self,
        key: Self::Key,
        layout: &bevy_internal::render::mesh::MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut desc = self.mesh_pipeline.specialize(key, layout)?;

        desc.label = Some("mesh_custom_pipeline".into());

        // The layout of the pipeline
        // It's important that it matches the order specified in the `DrawCustom`
        desc.layout = vec![
            self.mesh_pipeline.view_layout.clone(),
            self.bind_group_layout.clone(),
            self.mesh_pipeline.mesh_layouts.model_only.clone(),
        ];
        desc.vertex.shader = self.shader.clone();
        desc.fragment.as_mut().unwrap().shader = self.shader.clone();

        Ok(desc)
    }
}

/// Make sure all 3d cameras have a [`CustomPhase`] [`RenderPhase`]
fn extract_render_phase(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<CustomPhaseItem>::default());
        }
    }
}

/// Create the [`CustomMaterialUniform`] for each mesh with an Outline component
fn extract_custom_material_uniform(
    mut commands: Commands,
    custom_materials: Extract<Query<(Entity, &CustomMaterial)>>,
) {
    for (entity, custom_material) in &custom_materials {
        commands.get_or_spawn(entity).insert(CustomMaterialUniform {
            base_color: custom_material.base_color,
        });
    }
}

/// Queues the creation of the bind group
fn queue_custom_bind_group(
    mut commands: Commands,
    custom_pipeline: Res<CustomPipeline>,
    render_device: Res<RenderDevice>,
    custom_material_uniforms: Res<ComponentUniforms<CustomMaterialUniform>>,
) {
    let Some(uniform) = custom_material_uniforms.binding() else {
        return;
    };
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("custom_material_bind_group"),
        layout: &custom_pipeline.bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniform.clone(),
        }],
    });
    commands.insert_resource(CustomMaterialBindGroup(bind_group));
}

#[derive(Component, Clone, Copy, Default)]
pub struct CustomMaterial {
    pub base_color: Color,
}

// Bundle used to spanw a mesh rendered with the `CustomMaterial`
// It's essentially the `MaterialMeshBundle` but with the `CustomMaterial` instead of and `Handle<Material>`
#[derive(Bundle, Clone, Default)]
struct CustomMaterialMeshBundle {
    mesh: Handle<Mesh>,
    material: CustomMaterial,
    transform: Transform,
    global_transform: GlobalTransform,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
}

#[allow(clippy::too_many_arguments)]
fn queue_mesh_custom_phase(
    draw_functions: Res<DrawFunctions<CustomPhaseItem>>,
    pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    meshes: Query<(Entity, &Handle<Mesh>, &MeshUniform), With<CustomMaterialUniform>>,
    mut views: Query<(
        &ExtractedView,
        &mut VisibleEntities,
        &mut RenderPhase<CustomPhaseItem>,
    )>,
    msaa: Res<Msaa>,
) {
    let draw_function = draw_functions.read().id::<DrawCustom>();

    for (view, visible_entities, mut custom_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let inv_view_row_2 = view_matrix.inverse().row(2);

        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        for visible_entity in visible_entities.entities.iter().copied() {
            let Ok((entity, mesh_handle, mesh_uniform)) = meshes.get(visible_entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_handle) else {
                continue;
            };

            let key = MeshPipelineKey::from_primitive_topology(mesh.primitive_topology) | view_key;

            let Ok(pipeline) = pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout) else {
                continue;
            };

            // Add the draw command for the mesh to the custom phase
            custom_phase.add(CustomPhaseItem {
                entity,
                pipeline,
                draw_function,
                // Computes the distance to the view
                // This will be used to sort the draw command
                distance: inv_view_row_2.dot(mesh_uniform.transform.col(3)),
            });
        }
    }
}
