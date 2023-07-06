mod pipeline;
mod render_pass;

use bevy_core_pipeline::{core_2d::Camera2d, core_3d::Camera3d};
use bevy_hierarchy::Parent;
use bevy_render::{ExtractSchedule, Render};
use bevy_window::{PrimaryWindow, Window};
pub use pipeline::*;
pub use render_pass::*;

use crate::{
    prelude::UiCameraConfig, BackgroundColor, BorderColor, CalculatedClip, ContentSize, Node,
    Style, UiImage, UiScale, UiStack, UiTextureAtlasImage, Val,
};

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetEvent, Assets, Handle, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Rect, UVec4, Vec2, Vec3, Vec4Swizzles};
use bevy_reflect::TypeUuid;
use bevy_render::texture::DEFAULT_IMAGE_HANDLE;
use bevy_render::{
    camera::Camera,
    color::Color,
    render_asset::RenderAssets,
    render_graph::{RenderGraph, RunGraphOnViewNode},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions, RenderPhase},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::{ComputedVisibility, ExtractedView, ViewUniforms},
    Extract, RenderApp, RenderSet,
};
use bevy_sprite::SpriteAssetEvents;
use bevy_sprite::TextureAtlas;
#[cfg(feature = "bevy_text")]
use bevy_text::{PositionedGlyph, Text, TextLayoutInfo};
use bevy_transform::components::GlobalTransform;
use bevy_utils::FloatOrd;
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};
use std::ops::Range;

pub mod node {
    pub const UI_PASS_DRIVER: &str = "ui_pass_driver";
}

pub mod draw_ui_graph {
    pub const NAME: &str = "draw_ui";
    pub mod node {
        pub const UI_PASS: &str = "ui_pass";
    }
}

pub const UI_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 13012847047162779583);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
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
        .init_resource::<SpecializedRenderPipelines<UiPipeline>>()
        .init_resource::<UiImageBindGroups>()
        .init_resource::<UiMeta>()
        .init_resource::<ExtractedUiNodes>()
        .init_resource::<DrawFunctions<TransparentUi>>()
        .add_render_command::<TransparentUi, DrawUi>()
        .add_systems(
            ExtractSchedule,
            (
                extract_default_ui_camera_view::<Camera2d>,
                extract_default_ui_camera_view::<Camera3d>,
                extract_uinodes.in_set(RenderUiSystem::ExtractNode),
                extract_atlas_uinodes.after(RenderUiSystem::ExtractNode),
                extract_uinode_borders.after(RenderUiSystem::ExtractNode),
                #[cfg(feature = "bevy_text")]
                extract_text_uinodes.after(RenderUiSystem::ExtractNode),
            ),
        )
        .add_systems(
            Render,
            (
                prepare_uinodes.in_set(RenderSet::Prepare),
                queue_uinodes.in_set(RenderSet::Queue),
                sort_phase_system::<TransparentUi>.in_set(RenderSet::PhaseSort),
            ),
        );

    // Render graph
    let ui_graph_2d = get_ui_graph(render_app);
    let ui_graph_3d = get_ui_graph(render_app);
    let mut graph = render_app.world.resource_mut::<RenderGraph>();

    if let Some(graph_2d) = graph.get_sub_graph_mut(bevy_core_pipeline::core_2d::graph::NAME) {
        graph_2d.add_sub_graph(draw_ui_graph::NAME, ui_graph_2d);
        graph_2d.add_node(
            draw_ui_graph::node::UI_PASS,
            RunGraphOnViewNode::new(draw_ui_graph::NAME),
        );
        graph_2d.add_node_edge(
            bevy_core_pipeline::core_2d::graph::node::MAIN_PASS,
            draw_ui_graph::node::UI_PASS,
        );
        graph_2d.add_node_edge(
            bevy_core_pipeline::core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
            draw_ui_graph::node::UI_PASS,
        );
        graph_2d.add_node_edge(
            draw_ui_graph::node::UI_PASS,
            bevy_core_pipeline::core_2d::graph::node::UPSCALING,
        );
    }

    if let Some(graph_3d) = graph.get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME) {
        graph_3d.add_sub_graph(draw_ui_graph::NAME, ui_graph_3d);
        graph_3d.add_node(
            draw_ui_graph::node::UI_PASS,
            RunGraphOnViewNode::new(draw_ui_graph::NAME),
        );
        graph_3d.add_node_edge(
            bevy_core_pipeline::core_3d::graph::node::END_MAIN_PASS,
            draw_ui_graph::node::UI_PASS,
        );
        graph_3d.add_node_edge(
            bevy_core_pipeline::core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
            draw_ui_graph::node::UI_PASS,
        );
        graph_3d.add_node_edge(
            draw_ui_graph::node::UI_PASS,
            bevy_core_pipeline::core_3d::graph::node::UPSCALING,
        );
    }
}

fn get_ui_graph(render_app: &mut App) -> RenderGraph {
    let ui_pass_node = UiPassNode::new(&mut render_app.world);
    let mut ui_graph = RenderGraph::default();
    ui_graph.add_node(draw_ui_graph::node::UI_PASS, ui_pass_node);
    ui_graph
}

pub struct ExtractedUiNode {
    pub stack_index: usize,
    pub transform: Mat4,
    pub color: Color,
    pub rect: Rect,
    pub image: Handle<Image>,
    pub atlas_size: Option<Vec2>,
    pub clip: Option<Rect>,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Resource, Default)]
pub struct ExtractedUiNodes {
    pub uinodes: Vec<ExtractedUiNode>,
}

pub fn extract_atlas_uinodes(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<
            (
                &Node,
                &GlobalTransform,
                &BackgroundColor,
                &ComputedVisibility,
                Option<&CalculatedClip>,
                &Handle<TextureAtlas>,
                &UiTextureAtlasImage,
            ),
            Without<UiImage>,
        >,
    >,
) {
    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((uinode, transform, color, visibility, clip, texture_atlas_handle, atlas_image)) =
            uinode_query.get(*entity)
        {
            // Skip invisible and completely transparent nodes
            if !visibility.is_visible() || color.0.a() == 0.0 {
                continue;
            }

            let (mut atlas_rect, mut atlas_size, image) =
                if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
                    let atlas_rect = *texture_atlas
                        .textures
                        .get(atlas_image.index)
                        .unwrap_or_else(|| {
                            panic!(
                                "Atlas index {:?} does not exist for texture atlas handle {:?}.",
                                atlas_image.index,
                                texture_atlas_handle.id(),
                            )
                        });
                    (
                        atlas_rect,
                        texture_atlas.size,
                        texture_atlas.texture.clone(),
                    )
                } else {
                    // Atlas not present in assets resource (should this warn the user?)
                    continue;
                };

            // Skip loading images
            if !images.contains(&image) {
                continue;
            }

            let scale = uinode.size() / atlas_rect.size();
            atlas_rect.min *= scale;
            atlas_rect.max *= scale;
            atlas_size *= scale;

            extracted_uinodes.uinodes.push(ExtractedUiNode {
                stack_index,
                transform: transform.compute_matrix(),
                color: color.0,
                rect: atlas_rect,
                clip: clip.map(|clip| clip.clip),
                image,
                atlas_size: Some(atlas_size),
                flip_x: atlas_image.flip_x,
                flip_y: atlas_image.flip_y,
            });
        }
    }
}

fn resolve_border_thickness(value: Val, parent_width: f32, viewport_size: Vec2) -> f32 {
    match value {
        Val::Auto => 0.,
        Val::Px(px) => px.max(0.),
        Val::Percent(percent) => (parent_width * percent / 100.).max(0.),
        Val::Vw(percent) => (viewport_size.x * percent / 100.).max(0.),
        Val::Vh(percent) => (viewport_size.y * percent / 100.).max(0.),
        Val::VMin(percent) => (viewport_size.min_element() * percent / 100.).max(0.),
        Val::VMax(percent) => (viewport_size.max_element() * percent / 100.).max(0.),
    }
}

pub fn extract_uinode_borders(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    ui_scale: Extract<Res<UiScale>>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<
            (
                &Node,
                &GlobalTransform,
                &Style,
                &BorderColor,
                Option<&Parent>,
                &ComputedVisibility,
                Option<&CalculatedClip>,
            ),
            Without<ContentSize>,
        >,
    >,
    parent_node_query: Extract<Query<&Node, With<Parent>>>,
) {
    let image = bevy_render::texture::DEFAULT_IMAGE_HANDLE.typed();

    let ui_logical_viewport_size = windows
        .get_single()
        .map(|window| Vec2::new(window.resolution.width(), window.resolution.height()))
        .unwrap_or(Vec2::ZERO)
        // The logical window resolutin returned by `Window` only takes into account the window scale factor and not `UiScale`,
        // so we have to divide by `UiScale` to get the size of the UI viewport.
        / ui_scale.scale as f32;

    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((node, global_transform, style, border_color, parent, visibility, clip)) =
            uinode_query.get(*entity)
        {
            // Skip invisible borders
            if !visibility.is_visible()
                || border_color.0.a() == 0.0
                || node.size().x <= 0.
                || node.size().y <= 0.
            {
                continue;
            }

            // Both vertical and horizontal percentage border values are calculated based on the width of the parent node
            // <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
            let parent_width = parent
                .and_then(|parent| parent_node_query.get(parent.get()).ok())
                .map(|parent_node| parent_node.size().x)
                .unwrap_or(ui_logical_viewport_size.x);
            let left =
                resolve_border_thickness(style.border.left, parent_width, ui_logical_viewport_size);
            let right = resolve_border_thickness(
                style.border.right,
                parent_width,
                ui_logical_viewport_size,
            );
            let top =
                resolve_border_thickness(style.border.top, parent_width, ui_logical_viewport_size);
            let bottom = resolve_border_thickness(
                style.border.bottom,
                parent_width,
                ui_logical_viewport_size,
            );

            // Calculate the border rects, ensuring no overlap.
            // The border occupies the space between the node's bounding rect and the node's bounding rect inset in each direction by the node's corresponding border value.
            let max = 0.5 * node.size();
            let min = -max;
            let inner_min = min + Vec2::new(left, top);
            let inner_max = (max - Vec2::new(right, bottom)).max(inner_min);
            let border_rects = [
                // Left border
                Rect {
                    min,
                    max: Vec2::new(inner_min.x, max.y),
                },
                // Right border
                Rect {
                    min: Vec2::new(inner_max.x, min.y),
                    max,
                },
                // Top border
                Rect {
                    min: Vec2::new(inner_min.x, min.y),
                    max: Vec2::new(inner_max.x, inner_min.y),
                },
                // Bottom border
                Rect {
                    min: Vec2::new(inner_min.x, inner_max.y),
                    max: Vec2::new(inner_max.x, max.y),
                },
            ];

            let transform = global_transform.compute_matrix();

            for edge in border_rects {
                if edge.min.x < edge.max.x && edge.min.y < edge.max.y {
                    extracted_uinodes.uinodes.push(ExtractedUiNode {
                        stack_index,
                        // This translates the uinode's transform to the center of the current border rectangle
                        transform: transform * Mat4::from_translation(edge.center().extend(0.)),
                        color: border_color.0,
                        rect: Rect {
                            max: edge.size(),
                            ..Default::default()
                        },
                        image: image.clone_weak(),
                        atlas_size: None,
                        clip: clip.map(|clip| clip.clip),
                        flip_x: false,
                        flip_y: false,
                    });
                }
            }
        }
    }
}

pub fn extract_uinodes(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<
            (
                &Node,
                &GlobalTransform,
                &BackgroundColor,
                Option<&UiImage>,
                &ComputedVisibility,
                Option<&CalculatedClip>,
            ),
            Without<UiTextureAtlasImage>,
        >,
    >,
) {
    extracted_uinodes.uinodes.clear();

    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((uinode, transform, color, maybe_image, visibility, clip)) =
            uinode_query.get(*entity)
        {
            // Skip invisible and completely transparent nodes
            if !visibility.is_visible() || color.0.a() == 0.0 {
                continue;
            }

            let (image, flip_x, flip_y) = if let Some(image) = maybe_image {
                // Skip loading images
                if !images.contains(&image.texture) {
                    continue;
                }
                (image.texture.clone_weak(), image.flip_x, image.flip_y)
            } else {
                (DEFAULT_IMAGE_HANDLE.typed(), false, false)
            };

            extracted_uinodes.uinodes.push(ExtractedUiNode {
                stack_index,
                transform: transform.compute_matrix(),
                color: color.0,
                rect: Rect {
                    min: Vec2::ZERO,
                    max: uinode.calculated_size,
                },
                clip: clip.map(|clip| clip.clip),
                image,
                atlas_size: None,
                flip_x,
                flip_y,
            });
        };
    }
}

/// The UI camera is "moved back" by this many units (plus the [`UI_CAMERA_TRANSFORM_OFFSET`]) and also has a view
/// distance of this many units. This ensures that with a left-handed projection,
/// as ui elements are "stacked on top of each other", they are within the camera's view
/// and have room to grow.
// TODO: Consider computing this value at runtime based on the maximum z-value.
const UI_CAMERA_FAR: f32 = 1000.0;

// This value is subtracted from the far distance for the camera's z-position to ensure nodes at z == 0.0 are rendered
// TODO: Evaluate if we still need this.
const UI_CAMERA_TRANSFORM_OFFSET: f32 = -0.1;

#[derive(Component)]
pub struct DefaultCameraView(pub Entity);

pub fn extract_default_ui_camera_view<T: Component>(
    mut commands: Commands,
    ui_scale: Extract<Res<UiScale>>,
    query: Extract<Query<(Entity, &Camera, Option<&UiCameraConfig>), With<T>>>,
) {
    let scale = (ui_scale.scale as f32).recip();
    for (entity, camera, camera_ui) in &query {
        // ignore cameras with disabled ui
        if matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. })) {
            continue;
        }
        if let (Some(logical_size), Some((physical_origin, _)), Some(physical_size)) = (
            camera.logical_viewport_size(),
            camera.physical_viewport_rect(),
            camera.physical_viewport_size(),
        ) {
            // use a projection matrix with the origin in the top left instead of the bottom left that comes with OrthographicProjection
            let projection_matrix = Mat4::orthographic_rh(
                0.0,
                logical_size.x * scale,
                logical_size.y * scale,
                0.0,
                0.0,
                UI_CAMERA_FAR,
            );
            let default_camera_view = commands
                .spawn(ExtractedView {
                    projection: projection_matrix,
                    transform: GlobalTransform::from_xyz(
                        0.0,
                        0.0,
                        UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
                    ),
                    view_projection: None,
                    hdr: camera.hdr,
                    viewport: UVec4::new(
                        physical_origin.x,
                        physical_origin.y,
                        physical_size.x,
                        physical_size.y,
                    ),
                    color_grading: Default::default(),
                })
                .id();
            commands.get_or_spawn(entity).insert((
                DefaultCameraView(default_camera_view),
                RenderPhase::<TransparentUi>::default(),
            ));
        }
    }
}

#[cfg(feature = "bevy_text")]
pub fn extract_text_uinodes(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    ui_stack: Extract<Res<UiStack>>,
    ui_scale: Extract<Res<UiScale>>,
    uinode_query: Extract<
        Query<(
            &Node,
            &GlobalTransform,
            &Text,
            &TextLayoutInfo,
            &ComputedVisibility,
            Option<&CalculatedClip>,
        )>,
    >,
) {
    // TODO: Support window-independent UI scale: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0)
        * ui_scale.scale;

    let inverse_scale_factor = (scale_factor as f32).recip();

    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((uinode, global_transform, text, text_layout_info, visibility, clip)) =
            uinode_query.get(*entity)
        {
            // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
            if !visibility.is_visible() || uinode.size().x == 0. || uinode.size().y == 0. {
                continue;
            }
            let transform = global_transform.compute_matrix()
                * Mat4::from_translation(-0.5 * uinode.size().extend(0.));

            let mut color = Color::WHITE;
            let mut current_section = usize::MAX;
            for PositionedGlyph {
                position,
                atlas_info,
                section_index,
                ..
            } in &text_layout_info.glyphs
            {
                if *section_index != current_section {
                    color = text.sections[*section_index].style.color.as_rgba_linear();
                    current_section = *section_index;
                }
                let atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();

                let mut rect = atlas.textures[atlas_info.glyph_index];
                rect.min *= inverse_scale_factor;
                rect.max *= inverse_scale_factor;
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    stack_index,
                    transform: transform
                        * Mat4::from_translation(position.extend(0.) * inverse_scale_factor),
                    color,
                    rect,
                    image: atlas.texture.clone_weak(),
                    atlas_size: Some(atlas.size * inverse_scale_factor),
                    clip: clip.map(|clip| clip.clip),
                    flip_x: false,
                    flip_y: false,
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
    pub mode: u32,
}

#[derive(Resource)]
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
    Vec3::new(-0.5, -0.5, 0.0),
    Vec3::new(0.5, -0.5, 0.0),
    Vec3::new(0.5, 0.5, 0.0),
    Vec3::new(-0.5, 0.5, 0.0),
];

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub image: Handle<Image>,
    pub z: f32,
}

const TEXTURED_QUAD: u32 = 0;
const UNTEXTURED_QUAD: u32 = 1;

pub fn prepare_uinodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMeta>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
) {
    ui_meta.vertices.clear();

    // sort by ui stack index, starting from the deepest node
    extracted_uinodes
        .uinodes
        .sort_by_key(|node| node.stack_index);

    let mut start = 0;
    let mut end = 0;
    let mut current_batch_image = DEFAULT_IMAGE_HANDLE.typed();
    let mut last_z = 0.0;

    #[inline]
    fn is_textured(image: &Handle<Image>) -> bool {
        image.id() != DEFAULT_IMAGE_HANDLE.id()
    }

    for extracted_uinode in &extracted_uinodes.uinodes {
        let mode = if is_textured(&extracted_uinode.image) {
            if current_batch_image.id() != extracted_uinode.image.id() {
                if is_textured(&current_batch_image) && start != end {
                    commands.spawn(UiBatch {
                        range: start..end,
                        image: current_batch_image,
                        z: last_z,
                    });
                    start = end;
                }
                current_batch_image = extracted_uinode.image.clone_weak();
            }
            TEXTURED_QUAD
        } else {
            // Untextured `UiBatch`es are never spawned within the loop.
            // If all the `extracted_uinodes` are untextured a single untextured UiBatch will be spawned after the loop terminates.
            UNTEXTURED_QUAD
        };

        let mut uinode_rect = extracted_uinode.rect;

        let rect_size = uinode_rect.size().extend(1.0);

        // Specify the corners of the node
        let positions = QUAD_VERTEX_POSITIONS
            .map(|pos| (extracted_uinode.transform * (pos * rect_size).extend(1.)).xyz());

        // Calculate the effect of clipping
        // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
        let mut positions_diff = if let Some(clip) = extracted_uinode.clip {
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
        let uvs = if mode == UNTEXTURED_QUAD {
            [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
        } else {
            let atlas_extent = extracted_uinode.atlas_size.unwrap_or(uinode_rect.max);
            if extracted_uinode.flip_x {
                std::mem::swap(&mut uinode_rect.max.x, &mut uinode_rect.min.x);
                positions_diff[0].x *= -1.;
                positions_diff[1].x *= -1.;
                positions_diff[2].x *= -1.;
                positions_diff[3].x *= -1.;
            }
            if extracted_uinode.flip_y {
                std::mem::swap(&mut uinode_rect.max.y, &mut uinode_rect.min.y);
                positions_diff[0].y *= -1.;
                positions_diff[1].y *= -1.;
                positions_diff[2].y *= -1.;
                positions_diff[3].y *= -1.;
            }
            [
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
            .map(|pos| pos / atlas_extent)
        };

        let color = extracted_uinode.color.as_linear_rgba_f32();
        for i in QUAD_INDICES {
            ui_meta.vertices.push(UiVertex {
                position: positions_clipped[i].into(),
                uv: uvs[i].into(),
                color,
                mode,
            });
        }

        last_z = extracted_uinode.transform.w_axis[2];
        end += QUAD_INDICES.len() as u32;
    }

    // if start != end, there is one last batch to process
    if start != end {
        commands.spawn(UiBatch {
            range: start..end,
            image: current_batch_image,
            z: last_z,
        });
    }

    ui_meta.vertices.write_buffer(&render_device, &render_queue);
}

#[derive(Resource, Default)]
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
    pipeline_cache: Res<PipelineCache>,
    mut image_bind_groups: ResMut<UiImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    ui_batches: Query<(Entity, &UiBatch)>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<TransparentUi>)>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } | AssetEvent::Removed { handle } => {
                image_bind_groups.values.remove(handle)
            }
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
        let draw_ui_function = draw_functions.read().id::<DrawUi>();
        for (view, mut transparent_phase) in &mut views {
            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &ui_pipeline,
                UiPipelineKey { hdr: view.hdr },
            );
            for (entity, batch) in &ui_batches {
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
