mod camera;
mod pipeline;
mod render_pass;

pub use camera::*;
pub use pipeline::*;
pub use render_pass::*;

use crate::{CalculatedClip, Node, UiColor, UiImage};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetEvent, Assets, Handle, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_math::{const_vec3, Mat4, Vec2, Vec3, Vec4Swizzles};
use bevy_reflect::TypeUuid;
use bevy_render::{
    color::Color,
    render_asset::RenderAssets,
    render_graph::{RenderGraph, SlotInfo, SlotType},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions, RenderPhase},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::{ViewUniforms, Visibility},
    RenderApp, RenderStage, RenderWorld,
};
use bevy_sprite::{Rect, SpriteAssetEvents, TextureAtlas};
use bevy_text::{DefaultTextPipeline, Text};
use bevy_transform::components::GlobalTransform;
use bevy_utils::FloatOrd;
use bevy_utils::HashMap;
use bevy_window::{WindowId, Windows};
use bytemuck::{Pod, Zeroable};
use std::ops::Range;

pub mod node {
    pub const UI_PASS_DRIVER: &str = "ui_pass_driver";
}

pub mod draw_ui_graph {
    pub const NAME: &str = "draw_ui";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const UI_PASS: &str = "ui_pass";
    }
}

pub const UI_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 13012847047162779583);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum RenderUiSystem {
    ExtractNode,
}

pub fn build_ui_render(app: &mut App) {
    load_internal_asset!(app, UI_SHADER_HANDLE, "ui.wgsl", Shader::from_wgsl);

    let render_app = match app.get_sub_app_mut(RenderApp) {
        Ok(render_app) => render_app,
        Err(_) => return,
    };

    render_app
        .init_resource::<UiPipeline>()
        .init_resource::<SpecializedRenderPipelines<UiPipeline>>()
        .init_resource::<UiImageBindGroups>()
        .init_resource::<UiMeta>()
        .init_resource::<ExtractedUiNodes>()
        .init_resource::<DrawFunctions<TransparentUi>>()
        .add_render_command::<TransparentUi, DrawUi>()
        .add_system_to_stage(RenderStage::Extract, extract_ui_camera_phases)
        .add_system_to_stage(
            RenderStage::Extract,
            extract_uinodes.label(RenderUiSystem::ExtractNode),
        )
        .add_system_to_stage(
            RenderStage::Extract,
            extract_text_uinodes.after(RenderUiSystem::ExtractNode),
        )
        .add_system_to_stage(RenderStage::Prepare, prepare_uinodes)
        .add_system_to_stage(RenderStage::Queue, queue_uinodes)
        .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<TransparentUi>);

    // Render graph
    let ui_pass_node = UiPassNode::new(&mut render_app.world);
    let mut graph = render_app.world.resource_mut::<RenderGraph>();

    let mut draw_ui_graph = RenderGraph::default();
    draw_ui_graph.add_node(draw_ui_graph::node::UI_PASS, ui_pass_node);
    let input_node_id = draw_ui_graph.set_input(vec![SlotInfo::new(
        draw_ui_graph::input::VIEW_ENTITY,
        SlotType::Entity,
    )]);
    draw_ui_graph
        .add_slot_edge(
            input_node_id,
            draw_ui_graph::input::VIEW_ENTITY,
            draw_ui_graph::node::UI_PASS,
            UiPassNode::IN_VIEW,
        )
        .unwrap();
    graph.add_sub_graph(draw_ui_graph::NAME, draw_ui_graph);

    graph.add_node(node::UI_PASS_DRIVER, UiPassDriverNode);
    graph
        .add_node_edge(
            bevy_core_pipeline::node::MAIN_PASS_DRIVER,
            node::UI_PASS_DRIVER,
        )
        .unwrap();
}

pub struct ExtractedUiNode {
    pub transform: Mat4,
    pub color: Color,
    pub rect: Rect,
    pub image: Handle<Image>,
    pub atlas_size: Option<Vec2>,
    pub clip: Option<Rect>,
}

#[derive(Default)]
pub struct ExtractedUiNodes {
    pub uinodes: Vec<ExtractedUiNode>,
}

pub fn extract_uinodes(
    mut render_world: ResMut<RenderWorld>,
    images: Res<Assets<Image>>,
    uinode_query: Query<(
        &Node,
        &GlobalTransform,
        &UiColor,
        &UiImage,
        &Visibility,
        Option<&CalculatedClip>,
    )>,
) {
    let mut extracted_uinodes = render_world.resource_mut::<ExtractedUiNodes>();
    extracted_uinodes.uinodes.clear();
    for (uinode, transform, color, image, visibility, clip) in uinode_query.iter() {
        if !visibility.is_visible {
            continue;
        }
        let image = image.0.clone_weak();
        // Skip loading images
        if !images.contains(image.clone_weak()) {
            continue;
        }
        extracted_uinodes.uinodes.push(ExtractedUiNode {
            transform: transform.compute_matrix(),
            color: color.0,
            rect: bevy_sprite::Rect {
                min: Vec2::ZERO,
                max: uinode.size,
            },
            image,
            atlas_size: None,
            clip: clip.map(|clip| clip.clip),
        });
    }
}

pub fn extract_text_uinodes(
    mut render_world: ResMut<RenderWorld>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    text_pipeline: Res<DefaultTextPipeline>,
    windows: Res<Windows>,
    uinode_query: Query<(
        Entity,
        &Node,
        &GlobalTransform,
        &Text,
        &Visibility,
        Option<&CalculatedClip>,
    )>,
) {
    let mut extracted_uinodes = render_world.resource_mut::<ExtractedUiNodes>();

    let scale_factor = windows.scale_factor(WindowId::primary()) as f32;

    for (entity, uinode, transform, text, visibility, clip) in uinode_query.iter() {
        if !visibility.is_visible {
            continue;
        }
        // Skip if size is set to zero (e.g. when a parent is set to `Display::None`)
        if uinode.size == Vec2::ZERO {
            continue;
        }
        if let Some(text_layout) = text_pipeline.get_glyphs(&entity) {
            let text_glyphs = &text_layout.glyphs;
            let alignment_offset = (uinode.size / -2.0).extend(0.0);

            for text_glyph in text_glyphs {
                let color = text.sections[text_glyph.section_index].style.color;
                let atlas = texture_atlases
                    .get(text_glyph.atlas_info.texture_atlas.clone_weak())
                    .unwrap();
                let texture = atlas.texture.clone_weak();
                let index = text_glyph.atlas_info.glyph_index as usize;
                let rect = atlas.textures[index];
                let atlas_size = Some(atlas.size);

                let transform =
                    Mat4::from_rotation_translation(transform.rotation, transform.translation)
                        * Mat4::from_scale(transform.scale / scale_factor)
                        * Mat4::from_translation(
                            alignment_offset * scale_factor + text_glyph.position.extend(0.),
                        );

                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    transform,
                    color,
                    rect,
                    image: texture,
                    atlas_size,
                    clip: clip.map(|clip| clip.clip),
                });
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub struct UiMeta {
    vertices: BufferVec<UiVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

const QUAD_VERTEX_POSITIONS: [Vec3; 4] = [
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
    const_vec3!([-0.5, 0.5, 0.0]),
];

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub image: Handle<Image>,
    pub z: f32,
}

pub fn prepare_uinodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMeta>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
) {
    ui_meta.vertices.clear();

    // sort by increasing z for correct transparency
    extracted_uinodes
        .uinodes
        .sort_by(|a, b| FloatOrd(a.transform.w_axis[2]).cmp(&FloatOrd(b.transform.w_axis[2])));

    let mut start = 0;
    let mut end = 0;
    let mut current_batch_handle = Default::default();
    let mut last_z = 0.0;
    for extracted_uinode in &extracted_uinodes.uinodes {
        if current_batch_handle != extracted_uinode.image {
            if start != end {
                commands.spawn_bundle((UiBatch {
                    range: start..end,
                    image: current_batch_handle,
                    z: last_z,
                },));
                start = end;
            }
            current_batch_handle = extracted_uinode.image.clone_weak();
        }

        let uinode_rect = extracted_uinode.rect;
        let rect_size = uinode_rect.size().extend(1.0);

        // Specify the corners of the node
        let positions = QUAD_VERTEX_POSITIONS
            .map(|pos| (extracted_uinode.transform * (pos * rect_size).extend(1.)).xyz());

        // Calculate the effect of clipping
        // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
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

        // Cull nodes that are completely clipped
        if positions_diff[0].x - positions_diff[1].x >= rect_size.x
            || positions_diff[1].y - positions_diff[2].y >= rect_size.y
        {
            continue;
        }

        // Clip UVs (Note: y is reversed in UV space)
        let atlas_extent = extracted_uinode.atlas_size.unwrap_or(uinode_rect.max);
        let uvs = [
            Vec2::new(
                uinode_rect.min.x + positions_diff[0].x,
                uinode_rect.max.y - positions_diff[0].y,
            ),
            Vec2::new(
                uinode_rect.max.x + positions_diff[1].x,
                uinode_rect.max.y - positions_diff[1].y,
            ),
            Vec2::new(
                uinode_rect.max.x + positions_diff[2].x,
                uinode_rect.min.y - positions_diff[2].y,
            ),
            Vec2::new(
                uinode_rect.min.x + positions_diff[3].x,
                uinode_rect.min.y - positions_diff[3].y,
            ),
        ]
        .map(|pos| pos / atlas_extent);

        for i in QUAD_INDICES {
            ui_meta.vertices.push(UiVertex {
                position: positions_clipped[i].into(),
                uv: uvs[i].into(),
                color: extracted_uinode.color.as_linear_rgba_f32(),
            });
        }

        last_z = extracted_uinode.transform.w_axis[2];
        end += QUAD_INDICES.len() as u32;
    }

    // if start != end, there is one last batch to process
    if start != end {
        commands.spawn_bundle((UiBatch {
            range: start..end,
            image: current_batch_handle,
            z: last_z,
        },));
    }

    ui_meta.vertices.write_buffer(&render_device, &render_queue);
}

#[derive(Default)]
pub struct UiImageBindGroups {
    pub values: HashMap<Handle<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_uinodes(
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    render_device: Res<RenderDevice>,
    mut ui_meta: ResMut<UiMeta>,
    view_uniforms: Res<ViewUniforms>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut image_bind_groups: ResMut<UiImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    ui_batches: Query<(Entity, &UiBatch)>,
    mut views: Query<&mut RenderPhase<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } => image_bind_groups.values.remove(handle),
            AssetEvent::Removed { handle } => image_bind_groups.values.remove(handle),
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        ui_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("ui_view_bind_group"),
            layout: &ui_pipeline.view_layout,
        }));
        let draw_ui_function = draw_functions.read().get_id::<DrawUi>().unwrap();
        let pipeline = pipelines.specialize(&mut pipeline_cache, &ui_pipeline, UiPipelineKey {});
        for mut transparent_phase in views.iter_mut() {
            for (entity, batch) in ui_batches.iter() {
                image_bind_groups
                    .values
                    .entry(batch.image.clone_weak())
                    .or_insert_with(|| {
                        let gpu_image = gpu_images.get(&batch.image).unwrap();
                        render_device.create_bind_group(&BindGroupDescriptor {
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(&gpu_image.texture_view),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&gpu_image.sampler),
                                },
                            ],
                            label: Some("ui_material_bind_group"),
                            layout: &ui_pipeline.image_layout,
                        })
                    });

                transparent_phase.add(TransparentUi {
                    draw_function: draw_ui_function,
                    pipeline,
                    entity,
                    sort_key: FloatOrd(batch.z),
                });
            }
        }
    }
}
