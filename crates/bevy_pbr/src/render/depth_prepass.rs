use crate::{
    draw_3d_graph, AlphaMode, MeshPipeline, MeshUniform, StandardMaterial, MeshViewBindGroup,
    DEPTH_PREPASS_SHADER_HANDLE,Material, RenderMaterials
};

use bevy_app::Plugin;
use bevy_asset::Handle;
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
};
use bevy_core_pipeline::prelude::Camera3d;
use bevy_render::{
    camera::{ CameraPlugin},
    mesh::Mesh,
    render_asset::RenderAssets,
    extract_component::DynamicUniformIndex,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{
        sort_phase_system, AddRenderCommand, DrawFunctionId, DrawFunctions, EntityPhaseItem,
        PhaseItem, RenderCommand, RenderPhase, TrackedRenderPass,RenderCommandResult
    },
    render_resource::{
        BindGroup, BindGroupLayout, CachedRenderPipelineId, FragmentState, PipelineCache,
        RenderPipelineDescriptor, Shader, SpecializedRenderPipeline, SpecializedRenderPipelines,
        VertexBufferLayout, VertexState,  SamplerBindingType,
    },
    renderer::{RenderContext, RenderDevice},
    texture::Image,
    view::{
        ExtractedView, Msaa, ViewDepthTexture, ViewUniformOffset, ViewUniforms, VisibleEntities,
    },
    RenderApp, RenderStage,
};
use bevy_utils::FloatOrd;
use bevy_utils::HashMap;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferSize, CompareFunction, DepthBiasState,
    DepthStencilState, Face, FrontFace, IndexFormat, LoadOp, MultisampleState, Operations,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, ShaderStages, StencilFaceState, StencilState, TextureFormat,
    TextureSampleType, TextureViewDimension, VertexAttribute, VertexFormat, VertexStepMode,
};
use std::hash::Hash;
use std::marker::PhantomData;


pub struct DepthPrepassPipeline {
    pub view_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
}

impl FromWorld for DepthPrepassPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        // TODO: change this to StandardMaterialUniformData::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                },
                // Base Color Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Base Color Texture Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });

        let mesh_layout = mesh_pipeline.mesh_layout.clone();

        DepthPrepassPipeline {
            view_layout,
            material_layout,
            mesh_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct DepthPrepassPipelineKey: u32 {
        const NONE                        = 0;
        const VERTEX_TANGENTS             = (1 << 0);
        const OPAQUE_DEPTH_PREPASS        = (1 << 1);
        const ALPHA_MASK_DEPTH_PREPASS    = (1 << 2);
        const MSAA_RESERVED_BITS          = DepthPrepassPipelineKey::MSAA_MASK_BITS << DepthPrepassPipelineKey::MSAA_SHIFT_BITS;
    }
}

impl DepthPrepassPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        DepthPrepassPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for DepthPrepassPipeline {
    type Key = DepthPrepassPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut vertex_attributes = vec![
            // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 0,
            },
        ];
        let vertex_array_stride = if key.contains(DepthPrepassPipelineKey::VERTEX_TANGENTS) {
            if key.contains(DepthPrepassPipelineKey::ALPHA_MASK_DEPTH_PREPASS) {
                vertex_attributes.push(
                    // Uv (GOTCHA! uv is no longer third in the buffer due to how Mesh sorts attributes (alphabetically))
                    VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 40,
                        shader_location: 1,
                    },
                );
            }
            48
        } else {
            if key.contains(DepthPrepassPipelineKey::ALPHA_MASK_DEPTH_PREPASS) {
                vertex_attributes.push(
                    // Uv
                    VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 24,
                        shader_location: 1,
                    },
                );
            }
            32
        };
        let mut shader_defs = Vec::new();
        if key.contains(DepthPrepassPipelineKey::VERTEX_TANGENTS) {
            shader_defs.push(String::from("VERTEX_TANGENTS"));
        }

        let (label, entry_point, fragment_state) =
            if key.contains(DepthPrepassPipelineKey::OPAQUE_DEPTH_PREPASS) {
                (
                    Some("opaque_depth_prepass_pipeline".into()),
                    "vertex_opaque".into(),
                    None,
                )
            } else {
                (
                    Some("alpha_mask_depth_prepass_pipeline".into()),
                    "vertex_alpha_mask".into(),
                    Some(FragmentState {
                        shader: DEPTH_PREPASS_SHADER_HANDLE.typed::<Shader>(),
                        shader_defs: shader_defs.clone(),
                        entry_point: "fragment_alpha_mask".into(),
                        targets: vec![],
                    }),
                )
            };

        RenderPipelineDescriptor {
            label,
            vertex: VertexState {
                shader: DEPTH_PREPASS_SHADER_HANDLE.typed::<Shader>(),
                entry_point,
                shader_defs,
                buffers: vec![VertexBufferLayout {
                    array_stride: vertex_array_stride,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vertex_attributes,
                }],
            },
            fragment: fragment_state,
            layout: Some(vec![
                self.view_layout.clone(),
                self.material_layout.clone(),
                self.mesh_layout.clone(),
            ]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
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
        }
    }
}

pub struct OpaqueDepth3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for OpaqueDepth3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for OpaqueDepth3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

pub struct AlphaMaskDepth3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for AlphaMaskDepth3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for AlphaMaskDepth3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum Systems {
    ExtractDepthPhases,
    QueueDepthPrepassMeshes,
}

pub struct DepthPrepassPlugin<M: Material>(PhantomData<M>);
impl<M: Material> Plugin for DepthPrepassPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut bevy_app::App) {
        let render_app = app.sub_app(RenderApp);
        render_app
            .add_system_to_stage(
                RenderStage::Extract,
                extract_depth_phases.label(Systems::ExtractDepthPhases),
            )
            .add_system_to_stage(
                RenderStage::Queue,
                queue_depth_prepass_meshes::<M>.label(Systems::QueueDepthPrepassMeshes),
            )
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<OpaqueDepth3d>)
            .add_system_to_stage(
                RenderStage::PhaseSort,
                sort_phase_system::<AlphaMaskDepth3d>,
            )
            .init_resource::<DepthPrepassMaterialBindGroups>()
            .init_resource::<DepthPrepassPipeline>()
            .init_resource::<SpecializedRenderPipelines<DepthPrepassPipeline>>()
            .init_resource::<DrawFunctions<OpaqueDepth3d>>()
            .init_resource::<DrawFunctions<AlphaMaskDepth3d>>();

        let depth_prepass_node = DepthPrepassNode::new(&mut render_app.world);
        render_app.add_render_command::<OpaqueDepth3d, DrawDepth>();
        render_app.add_render_command::<AlphaMaskDepth3d, DrawDepth>();
        let render_world = render_app.world.cell();
        let mut graph = render_world.get_resource_mut::<RenderGraph>().unwrap();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();

        draw_3d_graph.add_node(draw_3d_graph::node::DEPTH_PREPASS, depth_prepass_node);
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::DEPTH_PREPASS,
                bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
            )
            .unwrap();
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
                draw_3d_graph::node::DEPTH_PREPASS,
                DepthPrepassNode::IN_VIEW,
            )
            .unwrap();
    }
}


// TODO: ActiveCameras
// pub fn extract_depth_phases(mut commands: Commands, active_cameras: Res<ActiveCameras>) {
pub fn extract_depth_phases(mut commands: Commands, active_cameras: Res<Camera3d>) {
        if let Some(camera_3d) = active_cameras.get(CameraPlugin::CAMERA_3D) {
        if let Some(entity) = camera_3d.entity {
            commands.get_or_spawn(entity).insert_bundle((
                RenderPhase::<OpaqueDepth3d>::default(),
                RenderPhase::<AlphaMaskDepth3d>::default(),
            ));
        }
    }
}

#[derive(Component)]
pub struct DepthPrepassViewBindGroup {
    pub value: BindGroup,
}

pub type DepthPrepassMaterialBindGroups = HashMap<Handle<StandardMaterial>, BindGroup>;

#[allow(clippy::too_many_arguments)]
pub fn queue_depth_prepass_meshes<M: Material>(
    mut commands: Commands,
    msaa: Res<Msaa>,
    opaque_depth_draw_functions: Res<DrawFunctions<OpaqueDepth3d>>,
    alpha_mask_depth_draw_functions: Res<DrawFunctions<AlphaMaskDepth3d>>,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    mut depth_prepass_material_bind_groups: ResMut<DepthPrepassMaterialBindGroups>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DepthPrepassPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    gpu_images: Res<RenderAssets<Image>>,
    render_materials: Res<RenderMaterials<M>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    standard_material_meshes: Query<(&Handle<StandardMaterial>, &Handle<Mesh>, &MeshUniform)>,
    depth_prepass_pipeline: Res<DepthPrepassPipeline>,
    mesh_pipeline: Res<MeshPipeline>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<OpaqueDepth3d>,
        &mut RenderPhase<AlphaMaskDepth3d>,
    )>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        for (entity, view, visible_entities, mut opaque_depth_phase, mut alpha_mask_depth_phase) in
            views.iter_mut()
        {
            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_binding.clone(),
                }],
                label: None,
                layout: &depth_prepass_pipeline.view_layout,
            });

            commands.entity(entity).insert(DepthPrepassViewBindGroup {
                value: view_bind_group,
            });

            let draw_opaque_depth = opaque_depth_draw_functions
                .read()
                .get_id::<DrawDepth>()
                .unwrap();
            let draw_alpha_mask_depth = alpha_mask_depth_draw_functions
                .read()
                .get_id::<DrawDepth>()
                .unwrap();

            let inverse_view_matrix = view.transform.compute_matrix().inverse();
            let inverse_view_row_2 = inverse_view_matrix.row(2);

            for visible_entity in &visible_entities.entities {
                if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                    standard_material_meshes.get(visible_entity.entity)
                {
                    if let Some(material) = render_materials.get(material_handle) {
                        if material.alpha_mode == AlphaMode::Blend {
                            continue;
                        }
                        if !depth_prepass_material_bind_groups.contains_key(material_handle) {
                            if let Some((base_color_texture_view, base_color_sampler)) =
                                mesh_pipeline.image_handle_to_texture(
                                    &*gpu_images,
                                    &material.base_color_texture,
                                )
                            {
                                let bind_group =
                                    render_device.create_bind_group(&BindGroupDescriptor {
                                        entries: &[
                                            BindGroupEntry {
                                                binding: 0,
                                                resource: material.buffer.as_entire_binding(),
                                            },
                                            BindGroupEntry {
                                                binding: 1,
                                                resource: BindingResource::TextureView(
                                                    base_color_texture_view,
                                                ),
                                            },
                                            BindGroupEntry {
                                                binding: 2,
                                                resource: BindingResource::Sampler(
                                                    base_color_sampler,
                                                ),
                                            },
                                        ],
                                        label: None,
                                        layout: &depth_prepass_pipeline.material_layout,
                                    });
                                depth_prepass_material_bind_groups
                                    .insert(material_handle.clone(), bind_group);
                            }
                        }

                        let mut key = DepthPrepassPipelineKey::from_msaa_samples(msaa.samples);
                        if let Some(mesh) = render_meshes.get(mesh_handle) {
                            if mesh.has_tangents {
                                key |= DepthPrepassPipelineKey::VERTEX_TANGENTS;
                            }
                        }
                        key |= match material.alpha_mode {
                            AlphaMode::Opaque => DepthPrepassPipelineKey::OPAQUE_DEPTH_PREPASS,
                            AlphaMode::Mask(_) => DepthPrepassPipelineKey::ALPHA_MASK_DEPTH_PREPASS,
                            _ => panic!("No depth prepass for alpha blend mode"),
                        };
                        let pipeline_id =
                            pipelines.specialize(&mut pipeline_cache, &depth_prepass_pipeline, key);

                        // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                        //       gives the z component of translation of the mesh in view space
                        let mesh_z = inverse_view_row_2.dot(mesh_uniform.transform.col(3));

                        // NOTE: Front-to-back ordering for opaque and alpha mask with ascending sort means near should have the
                        //       lowest sort key and getting further away should increase. As we have
                        //       -z in front fo the camera, values in view space decrease away from the
                        //       camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                        let distance = -mesh_z;
                        match material.alpha_mode {
                            AlphaMode::Opaque => {
                                opaque_depth_phase.add(OpaqueDepth3d {
                                    entity: visible_entity.entity,
                                    draw_function: draw_opaque_depth,
                                    pipeline: pipeline_id,
                                    distance,
                                });
                            }
                            AlphaMode::Mask(_) => {
                                alpha_mask_depth_phase.add(AlphaMaskDepth3d {
                                    entity: visible_entity.entity,
                                    draw_function: draw_alpha_mask_depth,
                                    pipeline: pipeline_id,
                                    distance,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

pub struct DepthPrepassNode {
    query: QueryState<
        (
            &'static RenderPhase<OpaqueDepth3d>,
            &'static RenderPhase<AlphaMaskDepth3d>,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl DepthPrepassNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for DepthPrepassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(DepthPrepassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;

        let (opaque_depth_phase, alpha_mask_depth_phase, depth) = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");

        {
            // Run the opaque pass, sorted front-to-back
            // NOTE: Scoped to drop the mutable borrow of render_context

            let draw_functions = world
                .get_resource::<DrawFunctions<OpaqueDepth3d>>()
                .unwrap();
            let mut draw_functions = draw_functions.write();

            let pass_descriptor = RenderPassDescriptor {
                label: Some("opaque_depth_prepass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The opaque depth prepass clears and writes to the depth buffer.
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };
            let pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut tracked_pass = TrackedRenderPass::new(pass);
            for item in opaque_depth_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        {
            // Run the alpha_mask depth prepass, sorted front-to-back
            // NOTE: Scoped to drop the mutable borrow of render_context

            let draw_functions = world
                .get_resource::<DrawFunctions<AlphaMaskDepth3d>>()
                .unwrap();
            let mut draw_functions = draw_functions.write();

            let pass_descriptor = RenderPassDescriptor {
                label: Some("alpha_mask_depth_prepass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The alpha_mask pass loads and writes to the depth buffer.
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };
            let pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut tracked_pass = TrackedRenderPass::new(pass);
            for item in alpha_mask_depth_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        Ok(())
    }
}

pub type DrawDepth = (
    SetDepthPrepassPipeline,
    SetMeshViewBindGroup<0>,
    SetDepthPrepassMaterialBindGroup<1>,
    SetTransformBindGroup<2>,
    DrawMesh,
);

pub struct SetDepthPrepassPipeline;
impl RenderCommand<OpaqueDepth3d> for SetDepthPrepassPipeline {
    type Param = SRes<PipelineCache>;
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &OpaqueDepth3d,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let pipeline = pipeline_cache
            .into_inner()
            .get_state(item.pipeline)
            .unwrap();
        pass.set_render_pipeline(pipeline);
        RenderCommandResult::Success
    }
}
impl RenderCommand<AlphaMaskDepth3d> for SetDepthPrepassPipeline {
    type Param = SRes<PipelineCache>;
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &AlphaMaskDepth3d,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) ->         RenderCommandResult    {
        let pipeline = pipeline_cache
            .into_inner()
            .get_state(item.pipeline)
            .unwrap();
        pass.set_render_pipeline(pipeline);
        RenderCommandResult::Success

    }
}

pub struct SetMeshViewBindGroup<const I: usize>;
impl<T: PhaseItem, const I: usize> RenderCommand<T> for SetMeshViewBindGroup<I> {
    type Param = SQuery<(Read<ViewUniformOffset>, Read<DepthPrepassViewBindGroup>)>;
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: &T,
        view_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (view_uniform, depth_prepass_view_bind_group) = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            &depth_prepass_view_bind_group.value,
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetTransformBindGroup<const I: usize>;
impl<T: EntityPhaseItem + PhaseItem, const I: usize> RenderCommand<T> for SetTransformBindGroup<I> {
    type Param = (
        SRes<MeshViewBindGroup>,
        SQuery<Read<DynamicUniformIndex<MeshUniform>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &T,
        (transform_bind_group, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let transform_index = mesh_query.get(item.entity()).unwrap();
        pass.set_bind_group(
            I,
            &transform_bind_group.into_inner().value,
            &[transform_index.index()],
        );
    }
}

pub struct SetDepthPrepassMaterialBindGroup<const I: usize>;
impl<T: EntityPhaseItem + PhaseItem, const I: usize> RenderCommand<T>
    for SetDepthPrepassMaterialBindGroup<I>
{
    type Param = (
        SRes<DepthPrepassMaterialBindGroups>,
        SQuery<Read<Handle<StandardMaterial>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &T,
        (material_bind_groups, handle_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let handle = handle_query.get(item.entity()).unwrap();
        let material_bind_groups = material_bind_groups.into_inner();
        let material_bind_group = material_bind_groups.get(handle).unwrap();
        pass.set_bind_group(I, material_bind_group, &[]);
    }
}

pub struct DrawMesh;
impl<T: EntityPhaseItem + PhaseItem> RenderCommand<T> for DrawMesh {
    type Param = (SRes<RenderAssets<Mesh>>, SQuery<Read<Handle<Mesh>>>);
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &T,
        (meshes, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item.entity()).unwrap();
        let gpu_mesh = meshes.into_inner().get(mesh_handle).unwrap();
        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        if let Some(index_info) = &gpu_mesh.index_info {
            pass.set_index_buffer(index_info.buffer.slice(..), 0, IndexFormat::Uint32);
            pass.draw_indexed(0..index_info.count, 0, 0..1);
        } else {
            panic!("non-indexed drawing not supported yet")
        }
        RenderCommandResult::Success
    }
}

