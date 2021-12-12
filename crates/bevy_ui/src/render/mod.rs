mod camera;
mod pipeline;
mod render_pass;

pub use camera::*;
pub use pipeline::*;
pub use render_pass::*;

use std::ops::Range;

use bevy_app::prelude::*;
use bevy_asset::{AssetEvent, Assets, Handle, HandleUntyped};
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_math::{const_vec3, Mat4, Vec2, Vec3, Vec4Swizzles};
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::ActiveCameras,
    color::Color,
    render_asset::RenderAssets,
    render_graph::{RenderGraph, SlotInfo, SlotType},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions, RenderPhase},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::ViewUniforms,
    RenderApp, RenderStage, RenderWorld,
};
use bevy_sprite::{SpriteAssetEvents, TextureAtlas};
use bevy_text::{DefaultTextPipeline, Text};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bevy_window::Windows;

use bytemuck::{Pod, Zeroable};

use crate::{Node, UiColor, UiImage};

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
pub enum UiSystem {
    ExtractNode,
}

pub fn build_ui_render(app: &mut App) {
    let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
    let ui_shader = Shader::from_wgsl(include_str!("ui.wgsl"));
    shaders.set_untracked(UI_SHADER_HANDLE, ui_shader);

    let mut active_cameras = app.world.get_resource_mut::<ActiveCameras>().unwrap();
    active_cameras.add(CAMERA_UI);

    let render_app = app.sub_app(RenderApp);
    render_app
        .init_resource::<UiPipeline>()
        .init_resource::<SpecializedPipelines<UiPipeline>>()
        .init_resource::<UiImageBindGroups>()
        .init_resource::<UiMeta>()
        .init_resource::<ExtractedUiNodes>()
        .init_resource::<DrawFunctions<TransparentUi>>()
        .add_render_command::<TransparentUi, DrawUi>()
        .add_system_to_stage(RenderStage::Extract, extract_ui_camera_phases)
        .add_system_to_stage(
            RenderStage::Extract,
            extract_uinodes.label(UiSystem::ExtractNode),
        )
        .add_system_to_stage(
            RenderStage::Extract,
            extract_text_uinodes.after(UiSystem::ExtractNode),
        )
        .add_system_to_stage(RenderStage::Prepare, prepare_uinodes)
        .add_system_to_stage(RenderStage::Queue, queue_uinodes)
        .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<TransparentUi>);

    // Render graph
    let ui_pass_node = UiPassNode::new(&mut render_app.world);
    let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

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
    pub rect: bevy_sprite::Rect,
    pub image: Handle<Image>,
    pub atlas_size: Option<Vec2>,
}

#[derive(Default)]
pub struct ExtractedUiNodes {
    pub uinodes: Vec<ExtractedUiNode>,
}

pub fn extract_uinodes(
    mut render_world: ResMut<RenderWorld>,
    images: Res<Assets<Image>>,
    uinode_query: Query<(&Node, &GlobalTransform, &UiColor, &UiImage)>,
) {
    let mut extracted_uinodes = render_world.get_resource_mut::<ExtractedUiNodes>().unwrap();
    extracted_uinodes.uinodes.clear();
    for (uinode, transform, color, image) in uinode_query.iter() {
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
        });
    }
}

pub fn extract_text_uinodes(
    mut render_world: ResMut<RenderWorld>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    text_pipeline: Res<DefaultTextPipeline>,
    windows: Res<Windows>,
    uinode_query: Query<(Entity, &Node, &GlobalTransform, &Text)>,
) {
    let mut extracted_uinodes = render_world.get_resource_mut::<ExtractedUiNodes>().unwrap();

    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor() as f32
    } else {
        1.
    };

    for (entity, uinode, transform, text) in uinode_query.iter() {
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
    pub color: u32,
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

const QUAD_VERTEX_POSITIONS: &[Vec3] = &[
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
    const_vec3!([-0.5, 0.5, 0.0]),
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
];

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
    for extracted_uinode in extracted_uinodes.uinodes.iter() {
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

        // Specify the corners of the node
        let mut bottom_left = Vec2::new(uinode_rect.min.x, uinode_rect.max.y);
        let mut top_left = uinode_rect.min;
        let mut top_right = Vec2::new(uinode_rect.max.x, uinode_rect.min.y);
        let mut bottom_right = uinode_rect.max;

        let atlas_extent = extracted_uinode.atlas_size.unwrap_or(uinode_rect.max);
        bottom_left /= atlas_extent;
        bottom_right /= atlas_extent;
        top_left /= atlas_extent;
        top_right /= atlas_extent;

        let uvs: [[f32; 2]; 6] = [
            bottom_left.into(),
            top_right.into(),
            top_left.into(),
            bottom_left.into(),
            bottom_right.into(),
            top_right.into(),
        ];

        let rect_size = extracted_uinode.rect.size().extend(1.0);
        let color = extracted_uinode.color.as_linear_rgba_f32();
        // encode color as a single u32 to save space
        let color = (color[0] * 255.0) as u32
            | ((color[1] * 255.0) as u32) << 8
            | ((color[2] * 255.0) as u32) << 16
            | ((color[3] * 255.0) as u32) << 24;
        for (index, vertex_position) in QUAD_VERTEX_POSITIONS.iter().enumerate() {
            let mut final_position = *vertex_position * rect_size;
            final_position = (extracted_uinode.transform * final_position.extend(1.0)).xyz();
            ui_meta.vertices.push(UiVertex {
                position: final_position.into(),
                uv: uvs[index],
                color,
            });
        }

        last_z = extracted_uinode.transform.w_axis[2];
        end += QUAD_VERTEX_POSITIONS.len() as u32;
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
    mut pipelines: ResMut<SpecializedPipelines<UiPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut image_bind_groups: ResMut<UiImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    mut ui_batches: Query<(Entity, &UiBatch)>,
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
            for (entity, batch) in ui_batches.iter_mut() {
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
