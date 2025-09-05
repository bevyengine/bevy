use crate::ui_material::{MaterialNode, UiMaterial, UiMaterialKey};
use crate::*;
use bevy_asset::*;
use bevy_ecs::{
    prelude::{Component, With},
    query::ROQueryItem,
    system::{
        lifetimeless::{Read, SRes},
        *,
    },
};
use bevy_image::BevyDefault as _;
use bevy_math::{Affine2, FloatOrd, Rect, Vec2};
use bevy_mesh::VertexBufferLayout;
use bevy_render::{
    globals::{GlobalsBuffer, GlobalsUniform},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, TemporaryRenderEntity},
    view::*,
    Extract, ExtractSchedule, Render, RenderSystems,
};
use bevy_render::{RenderApp, RenderStartup};
use bevy_shader::{load_shader_library, Shader, ShaderRef};
use bevy_sprite::BorderRect;
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
use core::{hash::Hash, marker::PhantomData, ops::Range};

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given
/// [`UiMaterial`] asset type (which includes [`UiMaterial`] types).
pub struct UiMaterialPlugin<M: UiMaterial>(PhantomData<M>);

impl<M: UiMaterial> Default for UiMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: UiMaterial> Plugin for UiMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "ui_vertex_output.wgsl");

        embedded_asset!(app, "ui_material.wgsl");

        app.init_asset::<M>()
            //.register_type::<MaterialNode<M>>()
            .add_plugins((
                //ExtractComponentPlugin::<MaterialNode<M>>::extract_visible(),
                RenderAssetPlugin::<PreparedUiMaterial<M>>::default(),
            ));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<ExtractedUiMaterialNodes<M>>()
                .init_resource::<UiMaterialMeta<M>>()
                .init_resource::<SpecializedRenderPipelines<UiMaterialPipeline<M>>>()
                .add_systems(RenderStartup, init_ui_material_pipeline::<M>)
                .add_systems(
                    ExtractSchedule,
                    extract_ui_material_nodes::<M>.in_set(RenderUiSystems::ExtractBackgrounds),
                )
                .add_systems(
                    Render,
                    (
                        queue_ui_material_nodes::<M>.in_set(RenderSystems::Queue),
                        prepare_uimaterial_nodes::<M>.in_set(RenderSystems::PrepareBindGroups),
                    ),
                );
        }
    }
}

#[derive(Resource)]
pub struct UiMaterialMeta<M: UiMaterial> {
    vertices: RawBufferVec<UiMaterialVertex>,
    view_bind_group: Option<BindGroup>,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> Default for UiMaterialMeta<M> {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            view_bind_group: Default::default(),
            marker: PhantomData,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UiMaterialVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub size: [f32; 2],
    pub border: [f32; 4],
    pub radius: [f32; 4],
}

// in this [`UiMaterialPipeline`] there is (currently) no batching going on.
// Therefore the [`UiMaterialBatch`] is more akin to a draw call.
#[derive(Component)]
pub struct UiMaterialBatch<M: UiMaterial> {
    /// The range of vertices inside the [`UiMaterialMeta`]
    pub range: Range<u32>,
    pub material: AssetId<M>,
}

/// Render pipeline data for a given [`UiMaterial`]
#[derive(Resource)]
pub struct UiMaterialPipeline<M: UiMaterial> {
    pub ui_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> SpecializedRenderPipeline for UiMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = UiMaterialKey<M>;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // size
                VertexFormat::Float32x2,
                // border widths
                VertexFormat::Float32x4,
                // border radius
                VertexFormat::Float32x4,
            ],
        );
        let shader_defs = Vec::new();

        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            label: Some("ui_material_pipeline".into()),
            ..default()
        };

        descriptor.layout = vec![self.view_layout.clone(), self.ui_layout.clone()];

        M::specialize(&mut descriptor, key);

        descriptor
    }
}

pub fn init_ui_material_pipeline<M: UiMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    let ui_layout = M::bind_group_layout(&render_device);

    let view_layout = render_device.create_bind_group_layout(
        "ui_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<GlobalsUniform>(false),
            ),
        ),
    );

    let load_default = || load_embedded_asset!(asset_server.as_ref(), "ui_material.wgsl");

    commands.insert_resource(UiMaterialPipeline::<M> {
        ui_layout,
        view_layout,
        vertex_shader: match M::vertex_shader() {
            ShaderRef::Default => load_default(),
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => asset_server.load(path),
        },
        fragment_shader: match M::fragment_shader() {
            ShaderRef::Default => load_default(),
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => asset_server.load(path),
        },
        marker: PhantomData,
    });
}

pub type DrawUiMaterial<M> = (
    SetItemPipeline,
    SetMatUiViewBindGroup<M, 0>,
    SetUiMaterialBindGroup<M, 1>,
    DrawUiMaterialNode<M>,
);

pub struct SetMatUiViewBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P> for SetMatUiViewBindGroup<M, I> {
    type Param = SRes<UiMaterialMeta<M>>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: Option<()>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            ui_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetUiMaterialBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P>
    for SetUiMaterialBindGroup<M, I>
{
    type Param = SRes<RenderAssets<PreparedUiMaterial<M>>>;
    type ViewQuery = ();
    type ItemQuery = Read<UiMaterialBatch<M>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: Option<ROQueryItem<'_, '_, Self::ItemQuery>>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(material_handle) = material_handle else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials.into_inner().get(material_handle.material) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawUiMaterialNode<M>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial> RenderCommand<P> for DrawUiMaterialNode<M> {
    type Param = SRes<UiMaterialMeta<M>>;
    type ViewQuery = ();
    type ItemQuery = Read<UiMaterialBatch<M>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiMaterialBatch<M>>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, ui_meta.into_inner().vertices.buffer().unwrap().slice(..));
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}

pub struct ExtractedUiMaterialNode<M: UiMaterial> {
    pub stack_index: u32,
    pub transform: Affine2,
    pub rect: Rect,
    pub border: BorderRect,
    pub border_radius: [f32; 4],
    pub material: AssetId<M>,
    pub clip: Option<Rect>,
    // Camera to render this UI node to. By the time it is extracted,
    // it is defaulted to a single camera if only one exists.
    // Nodes with ambiguous camera will be ignored.
    pub extracted_camera_entity: Entity,
    pub main_entity: MainEntity,
    pub render_entity: Entity,
}

#[derive(Resource)]
pub struct ExtractedUiMaterialNodes<M: UiMaterial> {
    pub uinodes: Vec<ExtractedUiMaterialNode<M>>,
}

impl<M: UiMaterial> Default for ExtractedUiMaterialNodes<M> {
    fn default() -> Self {
        Self {
            uinodes: Default::default(),
        }
    }
}

pub fn extract_ui_material_nodes<M: UiMaterial>(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    materials: Extract<Res<Assets<M>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &MaterialNode<M>,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    for (entity, computed_node, transform, handle, inherited_visibility, clip, camera) in
        uinode_query.iter()
    {
        // skip invisible nodes
        if !inherited_visibility.get() || computed_node.is_empty() {
            continue;
        }

        // Skip loading materials
        if !materials.contains(handle) {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        extracted_uinodes.uinodes.push(ExtractedUiMaterialNode {
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            stack_index: computed_node.stack_index,
            transform: transform.into(),
            material: handle.id(),
            rect: Rect {
                min: Vec2::ZERO,
                max: computed_node.size(),
            },
            border: computed_node.border(),
            border_radius: computed_node.border_radius().into(),
            clip: clip.map(|clip| clip.clip),
            extracted_camera_entity,
            main_entity: entity.into(),
        });
    }
}

pub fn prepare_uimaterial_nodes<M: UiMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMaterialMeta<M>>,
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    if let (Some(view_binding), Some(globals_binding)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        let mut batches: Vec<(Entity, UiMaterialBatch<M>)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_material_view_bind_group",
            &ui_material_pipeline.view_layout,
            &BindGroupEntries::sequential((view_binding, globals_binding)),
        ));
        let mut index = 0;

        for ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_shader_handle = AssetId::invalid();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(extracted_uinode) = extracted_uinodes
                    .uinodes
                    .get(item.index)
                    .filter(|n| item.entity() == n.render_entity)
                {
                    let mut existing_batch = batches
                        .last_mut()
                        .filter(|_| batch_shader_handle == extracted_uinode.material);

                    if existing_batch.is_none() {
                        batch_item_index = item_index;
                        batch_shader_handle = extracted_uinode.material;

                        let new_batch = UiMaterialBatch {
                            range: index..index,
                            material: extracted_uinode.material,
                        };

                        batches.push((item.entity(), new_batch));

                        existing_batch = batches.last_mut();
                    }

                    let uinode_rect = extracted_uinode.rect;

                    let rect_size = uinode_rect.size();

                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        extracted_uinode
                            .transform
                            .transform_point2(pos * rect_size)
                            .extend(1.0)
                    });

                    let positions_diff = if let Some(clip) = extracted_uinode.clip {
                        [
                            Vec2::new(
                                f32::max(clip.min.x - positions[0].x, 0.),
                                f32::max(clip.min.y - positions[0].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[1].x, 0.),
                                f32::max(clip.min.y - positions[1].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[2].x, 0.),
                                f32::min(clip.max.y - positions[2].y, 0.),
                            ),
                            Vec2::new(
                                f32::max(clip.min.x - positions[3].x, 0.),
                                f32::min(clip.max.y - positions[3].y, 0.),
                            ),
                        ]
                    } else {
                        [Vec2::ZERO; 4]
                    };

                    let positions_clipped = [
                        positions[0] + positions_diff[0].extend(0.),
                        positions[1] + positions_diff[1].extend(0.),
                        positions[2] + positions_diff[2].extend(0.),
                        positions[3] + positions_diff[3].extend(0.),
                    ];

                    let transformed_rect_size =
                        extracted_uinode.transform.transform_vector2(rect_size);

                    // Don't try to cull nodes that have a rotation
                    // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                    // In those two cases, the culling check can proceed normally as corners will be on
                    // horizontal / vertical lines
                    // For all other angles, bypass the culling check
                    // This does not properly handles all rotations on all axis
                    if extracted_uinode.transform.x_axis[1] == 0.0 {
                        // Cull nodes that are completely clipped
                        if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
                        {
                            continue;
                        }
                    }
                    let uvs = [
                        Vec2::new(
                            uinode_rect.min.x + positions_diff[0].x,
                            uinode_rect.min.y + positions_diff[0].y,
                        ),
                        Vec2::new(
                            uinode_rect.max.x + positions_diff[1].x,
                            uinode_rect.min.y + positions_diff[1].y,
                        ),
                        Vec2::new(
                            uinode_rect.max.x + positions_diff[2].x,
                            uinode_rect.max.y + positions_diff[2].y,
                        ),
                        Vec2::new(
                            uinode_rect.min.x + positions_diff[3].x,
                            uinode_rect.max.y + positions_diff[3].y,
                        ),
                    ]
                    .map(|pos| pos / uinode_rect.max);

                    for i in QUAD_INDICES {
                        ui_meta.vertices.push(UiMaterialVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            size: extracted_uinode.rect.size().into(),
                            radius: extracted_uinode.border_radius,
                            border: [
                                extracted_uinode.border.left,
                                extracted_uinode.border.top,
                                extracted_uinode.border.right,
                                extracted_uinode.border.bottom,
                            ],
                        });
                    }

                    index += QUAD_INDICES.len() as u32;
                    existing_batch.unwrap().1.range.end = index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else {
                    batch_shader_handle = AssetId::invalid();
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.try_insert_batch(batches);
    }
    extracted_uinodes.uinodes.clear();
}

pub struct PreparedUiMaterial<T: UiMaterial> {
    pub bindings: BindingResources,
    pub bind_group: BindGroup,
    pub key: T::Data,
}

impl<M: UiMaterial> RenderAsset for PreparedUiMaterial<M> {
    type SourceAsset = M;

    type Param = (SRes<RenderDevice>, SRes<UiMaterialPipeline<M>>, M::Param);

    fn prepare_asset(
        material: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_device, pipeline, material_param): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let bind_group_data = material.bind_group_data();
        match material.as_bind_group(&pipeline.ui_layout, render_device, material_param) {
            Ok(prepared) => Ok(PreparedUiMaterial {
                bindings: prepared.bindings,
                bind_group: prepared.bind_group,
                key: bind_group_data,
            }),
            Err(AsBindGroupError::RetryNextUpdate) => {
                Err(PrepareAssetError::RetryNextUpdate(material))
            }
            Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
        }
    }
}

pub fn queue_ui_material_nodes<M: UiMaterial>(
    extracted_uinodes: Res<ExtractedUiMaterialNodes<M>>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_materials: Res<RenderAssets<PreparedUiMaterial<M>>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut render_views: Query<&UiCameraView, With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_function = draw_functions.read().id::<DrawUiMaterial<M>>();

    for (index, extracted_uinode) in extracted_uinodes.uinodes.iter().enumerate() {
        let Some(material) = render_materials.get(extracted_uinode.material) else {
            continue;
        };

        let Ok(default_camera_view) =
            render_views.get_mut(extracted_uinode.extracted_camera_entity)
        else {
            continue;
        };

        let Ok(view) = camera_views.get(default_camera_view.0) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_material_pipeline,
            UiMaterialKey {
                hdr: view.hdr,
                bind_group_data: material.key.clone(),
            },
        );
        if transparent_phase.items.capacity() < extracted_uinodes.uinodes.len() {
            transparent_phase.items.reserve_exact(
                extracted_uinodes.uinodes.len() - transparent_phase.items.capacity(),
            );
        }
        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline,
            entity: (extracted_uinode.render_entity, extracted_uinode.main_entity),
            sort_key: FloatOrd(extracted_uinode.stack_index as f32 + stack_z_offsets::MATERIAL),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            index,
            indexed: false,
        });
    }
}
