use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy_app::{App, Plugin};
use bevy_asset::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity, EventReader},
    query::{ROQueryItem, With},
    schedule::IntoSystemConfigs,
    storage::SparseSet,
    system::lifetimeless::{Read, SRes},
    system::*,
    world::{FromWorld, World},
};
use bevy_math::{Mat4, Rect, Vec2, Vec4Swizzles};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    render_asset::RenderAssets,
    render_phase::*,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, FallbackImage, Image},
    view::*,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{FloatOrd, HashMap, HashSet};
use bevy_window::{PrimaryWindow, Window};
use bytemuck::{Pod, Zeroable};

use crate::*;

pub const UI_MATERIAL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10074188772096983955);

const UI_VERTEX_OUTPUT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10123618247720234751);

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
        load_internal_asset!(
            app,
            UI_VERTEX_OUTPUT_SHADER_HANDLE,
            "ui_vertex_output.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            UI_MATERIAL_SHADER_HANDLE,
            "ui_material.wgsl",
            Shader::from_wgsl
        );
        app.init_asset::<M>()
            .add_plugins(ExtractComponentPlugin::<Handle<M>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<ExtractedUiMaterials<M>>()
                .init_resource::<ExtractedUiMaterialNodes<M>>()
                .init_resource::<RenderUiMaterials<M>>()
                .init_resource::<UiMaterialMeta<M>>()
                .init_resource::<SpecializedRenderPipelines<UiMaterialPipeline<M>>>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_ui_materials::<M>,
                        extract_ui_material_nodes::<M>.in_set(RenderUiSystem::ExtractNode),
                    ),
                )
                .add_systems(
                    Render,
                    (
                        prepare_ui_materials::<M>.in_set(RenderSet::PrepareAssets),
                        queue_ui_material_nodes::<M>.in_set(RenderSet::Queue),
                        prepare_uimaterial_nodes::<M>.in_set(RenderSet::PrepareBindGroups),
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
pub struct UiMaterialMeta<M: UiMaterial> {
    vertices: BufferVec<UiMaterialVertex>,
    view_bind_group: Option<BindGroup>,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> Default for UiMaterialMeta<M> {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
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
    pub border_widths: [f32; 4],
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
                // border_widths
                VertexFormat::Float32x4,
            ],
        );
        let shader_defs = Vec::new();

        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: UI_MATERIAL_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: UI_MATERIAL_SHADER_HANDLE,
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
            label: Some("ui_material_pipeline".into()),
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
    DrawUiMaterialNode<M>,
);

pub struct SetMatUiViewBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P> for SetMatUiViewBindGroup<M, I> {
    type Param = SRes<UiMaterialMeta<M>>;
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

pub struct SetUiMaterialBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P>
    for SetUiMaterialBindGroup<M, I>
{
    type Param = SRes<RenderUiMaterials<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<UiMaterialBatch<M>>;

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

pub struct DrawUiMaterialNode<M>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial> RenderCommand<P> for DrawUiMaterialNode<M> {
    type Param = SRes<UiMaterialMeta<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<UiMaterialBatch<M>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'w UiMaterialBatch<M>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_vertex_buffer(0, ui_meta.into_inner().vertices.buffer().unwrap().slice(..));
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}

pub struct ExtractedUiMaterialNode<M: UiMaterial> {
    pub stack_index: usize,
    pub transform: Mat4,
    pub rect: Rect,
    pub border: [f32; 4],
    pub material: AssetId<M>,
    pub clip: Option<Rect>,
}

#[derive(Resource)]
pub struct ExtractedUiMaterialNodes<M: UiMaterial> {
    pub uinodes: SparseSet<Entity, ExtractedUiMaterialNode<M>>,
}

impl<M: UiMaterial> Default for ExtractedUiMaterialNodes<M> {
    fn default() -> Self {
        Self {
            uinodes: Default::default(),
        }
    }
}

pub fn extract_ui_material_nodes<M: UiMaterial>(
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    materials: Extract<Res<Assets<M>>>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &Node,
            &Style,
            &GlobalTransform,
            &Handle<M>,
            &ViewVisibility,
            Option<&CalculatedClip>,
        )>,
    >,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    ui_scale: Extract<Res<UiScale>>,
) {
    let ui_logical_viewport_size = windows
        .get_single()
        .map(|window| Vec2::new(window.resolution.width(), window.resolution.height()))
        .unwrap_or(Vec2::ZERO)
        // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
        // so we have to divide by `UiScale` to get the size of the UI viewport.
        / ui_scale.0 as f32;
    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((entity, uinode, style, transform, handle, view_visibility, clip)) =
            uinode_query.get(*entity)
        {
            // skip invisible nodes
            if !view_visibility.get() {
                continue;
            }

            // Skip loading materials
            if !materials.contains(handle) {
                continue;
            }

            // Both vertical and horizontal percentage border values are calculated based on the width of the parent node
            // <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
            let parent_width = uinode.size().x;
            let left =
                resolve_border_thickness(style.border.left, parent_width, ui_logical_viewport_size)
                    / uinode.size().x;
            let right = resolve_border_thickness(
                style.border.right,
                parent_width,
                ui_logical_viewport_size,
            ) / uinode.size().x;
            let top =
                resolve_border_thickness(style.border.top, parent_width, ui_logical_viewport_size)
                    / uinode.size().y;
            let bottom = resolve_border_thickness(
                style.border.bottom,
                parent_width,
                ui_logical_viewport_size,
            ) / uinode.size().y;

            extracted_uinodes.uinodes.insert(
                entity,
                ExtractedUiMaterialNode {
                    stack_index,
                    transform: transform.compute_matrix(),
                    material: handle.id(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: uinode.calculated_size,
                    },
                    border: [left, right, top, bottom],
                    clip: clip.map(|clip| clip.clip),
                },
            );
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_uimaterial_nodes<M: UiMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMaterialMeta<M>>,
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    view_uniforms: Res<ViewUniforms>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut phases: Query<&mut RenderPhase<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiMaterialBatch<M>)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_material_view_bind_group",
            &ui_material_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));
        let mut index = 0;

        for mut ui_phase in &mut phases {
            let mut batch_item_index = 0;
            let mut batch_shader_handle = AssetId::invalid();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(extracted_uinode) = extracted_uinodes.uinodes.get(item.entity) {
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

                        batches.push((item.entity, new_batch));

                        existing_batch = batches.last_mut();
                    }

                    let uinode_rect = extracted_uinode.rect;

                    let rect_size = uinode_rect.size().extend(1.0);

                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        (extracted_uinode.transform * (pos * rect_size).extend(1.0)).xyz()
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
                        extracted_uinode.transform.transform_vector3(rect_size);

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
                            border_widths: extracted_uinode.border,
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
        commands.insert_or_spawn_batch(batches);
    }
    extracted_uinodes.uinodes.clear();
}

#[derive(Resource, Deref, DerefMut)]
pub struct RenderUiMaterials<T: UiMaterial>(HashMap<AssetId<T>, PreparedUiMaterial<T>>);

impl<T: UiMaterial> Default for RenderUiMaterials<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct PreparedUiMaterial<T: UiMaterial> {
    pub bindings: Vec<(u32, OwnedBindingResource)>,
    pub bind_group: BindGroup,
    pub key: T::Data,
}

#[derive(Resource)]
pub struct ExtractedUiMaterials<M: UiMaterial> {
    extracted: Vec<(AssetId<M>, M)>,
    removed: Vec<AssetId<M>>,
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
    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.push(*id);
            }
            AssetEvent::LoadedWithDependencies { .. } => {
                // not implemented
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            extracted_assets.push((id, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedUiMaterials {
        extracted: extracted_assets,
        removed,
    });
}

pub struct PrepareNextFrameMaterials<M: UiMaterial> {
    assets: Vec<(AssetId<M>, M)>,
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
    for (id, material) in queued_assets {
        match prepare_ui_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(id, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((id, material));
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

#[allow(clippy::too_many_arguments)]
pub fn queue_ui_material_nodes<M: UiMaterial>(
    extracted_uinodes: Res<ExtractedUiMaterialNodes<M>>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_materials: Res<RenderUiMaterials<M>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<TransparentUi>)>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_function = draw_functions.read().id::<DrawUiMaterial<M>>();

    for (entity, extracted_uinode) in extracted_uinodes.uinodes.iter() {
        let material = render_materials.get(&extracted_uinode.material).unwrap();
        for (view, mut transparent_phase) in &mut views {
            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &ui_material_pipeline,
                UiMaterialKey {
                    hdr: view.hdr,
                    bind_group_data: material.key.clone(),
                },
            );
            transparent_phase
                .items
                .reserve(extracted_uinodes.uinodes.len());
            transparent_phase.add(TransparentUi {
                draw_function,
                pipeline,
                entity: *entity,
                sort_key: (
                    FloatOrd(extracted_uinode.stack_index as f32),
                    entity.index(),
                ),
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}
