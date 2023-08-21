use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, AssetEvent, AssetServer, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity, EventReader},
    query::ROQueryItem,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Local, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_math::{Mat4, Rect, Vec2, Vec4Swizzles};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    prelude::Color,
    render_asset::{PrepareAssetSet, RenderAssets},
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        AsBindGroupError, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
        BufferBindingType, BufferUsages, BufferVec, ColorTargetState, ColorWrites, FragmentState,
        FrontFace, MultisampleState, OwnedBindingResource, PipelineCache, PolygonMode,
        PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, Shader, ShaderRef,
        ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
        TextureFormat, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, FallbackImage, Image},
    view::{
        ComputedVisibility, ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{FloatOrd, HashMap, HashSet};

use crate::{
    CalculatedClip, Node, RenderUiSystem, TransparentUi, UiMaterial, UiMaterialKey, UiStack,
    UiVertex, QUAD_INDICES, QUAD_VERTEX_POSITIONS,
};

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
    fn build(&self, app: &mut bevy_app::App) {
        app.add_asset::<M>()
            .add_plugins(ExtractComponentPlugin::<Handle<M>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<ExtractedUiMaterials<M>>()
                .init_resource::<ExtractedUiMaterialNodes<M>>()
                .init_resource::<RenderUiMaterials<M>>()
                .init_resource::<UiMatMeta<M>>()
                .init_resource::<SpecializedRenderPipelines<UiMaterialPipeline<M>>>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_ui_materials::<M>,
                        extract_material_uinodes::<M>.in_set(RenderUiSystem::ExtractNode),
                    ),
                )
                .add_systems(
                    Render,
                    (
                        prepare_ui_materials::<M>
                            .in_set(RenderSet::Prepare)
                            .after(PrepareAssetSet::PreAssetPrepare),
                        prepare_uimaterial_nodes::<M>.in_set(RenderSet::Prepare),
                        queue_ui_material_nodes::<M>.in_set(RenderSet::Queue),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiMaterialPipeline<M>>();
        }
    }
}

#[derive(Resource)]
pub struct UiMatMeta<M: UiMaterial> {
    vertices: BufferVec<UiVertex>,
    view_bind_group: Option<BindGroup>,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> Default for UiMatMeta<M> {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: Default::default(),
            marker: PhantomData,
        }
    }
}

#[derive(Component)]
pub struct MatUiBatch<M: UiMaterial> {
    pub range: Range<u32>,
    pub material: Handle<M>,
    pub z: f32,
}

#[derive(Resource)]
pub struct UiMaterialPipeline<M: UiMaterial> {
    pub ui_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
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
                // color
                VertexFormat::Float32x4,
                // mode
                VertexFormat::Uint32,
            ],
        );
        let shader_defs = Vec::new();

        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: super::UI_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: super::UI_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![],
            push_constant_ranges: Vec::new(),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("material_ui_pipeline".into()),
        };
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout = vec![self.view_layout.clone(), self.ui_layout.clone()];

        M::specialize(&mut descriptor, key);

        descriptor
    }
}

impl<M: UiMaterial> FromWorld for UiMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let ui_layout = M::bind_group_layout(render_device);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
            label: Some("ui_view_layout"),
        });

        UiMaterialPipeline {
            ui_layout,
            view_layout,
            vertex_shader: match M::vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: match M::fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            marker: PhantomData,
        }
    }
}

pub type DrawUiMaterial<M> = (
    SetItemPipeline,
    SetMatUiViewBindGroup<M, 0>,
    SetUiMaterialBindGroup<M, 1>,
    DrawUiMatNode<M>,
);

pub struct SetMatUiViewBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P> for SetMatUiViewBindGroup<M, I> {
    type Param = SRes<UiMatMeta<M>>;
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: (),
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

pub struct DrawUiMatNode<M>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial> RenderCommand<P> for DrawUiMatNode<M> {
    type Param = SRes<UiMatMeta<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<MatUiBatch<M>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'w MatUiBatch<M>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_vertex_buffer(0, ui_meta.into_inner().vertices.buffer().unwrap().slice(..));
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}

pub struct SetUiMaterialBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P>
    for SetUiMaterialBindGroup<M, I>
{
    type Param = SRes<RenderUiMaterials<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<MatUiBatch<M>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: ROQueryItem<'_, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = materials
            .into_inner()
            .get(&material_handle.material)
            .unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct ExtractedUiMaterialNode<M: UiMaterial> {
    pub stack_index: usize,
    pub transform: Mat4,
    pub rect: Rect,
    pub material: Handle<M>,
    pub clip: Option<Rect>,
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

pub fn extract_material_uinodes<M: UiMaterial>(
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<(
            &Node,
            &GlobalTransform,
            &Handle<M>,
            &ComputedVisibility,
            Option<&CalculatedClip>,
        )>,
    >,
) {
    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((uinode, transform, handle, visibility, clip)) = uinode_query.get(*entity) {
            if !visibility.is_visible() {
                continue;
            }
            // Skip invisible and completely transparent nodes
            extracted_uinodes.uinodes.push(ExtractedUiMaterialNode {
                stack_index,
                transform: transform.compute_matrix(),
                material: handle.clone_weak(),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: uinode.calculated_size,
                },
                clip: clip.map(|clip| clip.clip),
            });
        };
    }
}

pub fn prepare_uimaterial_nodes<M: UiMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMatMeta<M>>,
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
) {
    ui_meta.vertices.clear();

    extracted_uinodes
        .uinodes
        .sort_by_key(|node| node.stack_index);
    for extracted_uinode in extracted_uinodes.uinodes.drain(..) {
        let uinode_rect = extracted_uinode.rect;

        let rect_size = uinode_rect.size().extend(1.0);

        let positions = QUAD_VERTEX_POSITIONS
            .map(|pos| (extracted_uinode.transform * (pos * rect_size).extend(1.)).xyz());

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

        let transformed_rect_size = extracted_uinode.transform.transform_vector3(rect_size);

        if extracted_uinode.transform.x_axis[1] == 0.0
            && positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
        {
            continue;
        }
        let extent = uinode_rect.max;
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
        .map(|pos| pos / extent);

        for i in QUAD_INDICES {
            ui_meta.vertices.push(crate::UiVertex {
                position: positions_clipped[i].into(),
                uv: uvs[i].into(),
                color: Color::WHITE.into(),
                mode: 1,
            });
        }
        commands.spawn(MatUiBatch {
            range: 0..QUAD_INDICES.len() as u32,
            material: extracted_uinode.material,
            z: extracted_uinode.transform.w_axis[2],
        });
    }
    ui_meta.vertices.write_buffer(&render_device, &render_queue);
}

#[allow(clippy::too_many_arguments)]
pub fn queue_ui_material_nodes<M: UiMaterial>(
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    render_device: Res<RenderDevice>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut ui_meta: ResMut<UiMatMeta<M>>,
    view_uniforms: Res<ViewUniforms>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_materials: Res<RenderUiMaterials<M>>,
    ui_batches: Query<(Entity, &MatUiBatch<M>)>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<TransparentUi>)>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        ui_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("ui_view_bind_group"),
            layout: &ui_material_pipeline.view_layout,
        }));
        let draw_ui_function = draw_functions.read().id::<DrawUiMaterial<M>>();
        for (view, mut transparent_phase) in &mut views {
            for (entity, batch) in &ui_batches {
                if let Some(material) = render_materials.get(&batch.material) {
                    let pipeline = pipelines.specialize(
                        &pipeline_cache,
                        &ui_material_pipeline,
                        UiMaterialKey {
                            hdr: view.hdr,
                            bind_group_data: material.key.clone(),
                        },
                    );
                    transparent_phase.add(TransparentUi {
                        sort_key: FloatOrd(batch.z),
                        entity,
                        pipeline,
                        draw_function: draw_ui_function,
                    });
                }
            }
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct RenderUiMaterials<T: UiMaterial>(HashMap<Handle<T>, PreparedUiMaterial<T>>);

impl<T: UiMaterial> Default for RenderUiMaterials<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct PreparedUiMaterial<T: UiMaterial> {
    pub bindings: Vec<OwnedBindingResource>,
    pub bind_group: BindGroup,
    pub key: T::Data,
}

#[derive(Resource)]
pub struct ExtractedUiMaterials<M: UiMaterial> {
    extracted: Vec<(Handle<M>, M)>,
    removed: Vec<Handle<M>>,
}

impl<M: UiMaterial> Default for ExtractedUiMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

pub fn extract_ui_materials<M: UiMaterial>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(&handle) {
            extracted_assets.push((handle, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedUiMaterials {
        extracted: extracted_assets,
        removed,
    });
}

pub struct PrepareNextFrameMaterials<M: UiMaterial> {
    assets: Vec<(Handle<M>, M)>,
}

impl<M: UiMaterial> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

pub fn prepare_ui_materials<M: UiMaterial>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedUiMaterials<M>>,
    mut render_materials: ResMut<RenderUiMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<UiMaterialPipeline<M>>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, material) in queued_assets {
        match prepare_ui_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.remove(&removed);
    }

    for (handle, material) in std::mem::take(&mut extracted_assets.extracted) {
        match prepare_ui_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }
}

fn prepare_ui_material<M: UiMaterial>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &Res<FallbackImage>,
    pipeline: &UiMaterialPipeline<M>,
) -> Result<PreparedUiMaterial<M>, AsBindGroupError> {
    let prepared =
        material.as_bind_group(&pipeline.ui_layout, render_device, images, fallback_image)?;
    Ok(PreparedUiMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        key: prepared.data,
    })
}
