pub mod extracted;
pub mod instances;
mod pipeline;
mod render_pass;
mod ui_material_pipeline;

use bevy_core_pipeline::{core_2d::Camera2d, core_3d::Camera3d};
use bevy_hierarchy::Parent;
use bevy_render::render_phase::PhaseItem;
use bevy_render::view::ViewVisibility;
use bevy_render::{render_resource::BindGroupEntries, ExtractSchedule, Render};
use bevy_window::{PrimaryWindow, Window};
pub use pipeline::*;
pub use render_pass::*;
pub use ui_material_pipeline::*;

use crate::extracted::ExtractedUiNodes;
use crate::{
    prelude::UiCameraConfig, BackgroundColor, BorderColor, CalculatedClip, ContentSize, Node,
    Style, UiImage, UiScale, UiTextureAtlasImage, Val,
};
use crate::{Outline, UiColor, resolve_color_stops, OutlineStyle};

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_math::{Vec3Swizzles, vec2};
use bevy_math::{Mat4, Rect, URect, UVec4, Vec2, Vec3, Vec4Swizzles};
use bevy_render::{
    camera::Camera,
    color::Color,
    render_asset::RenderAssets,
    render_graph::{RenderGraph, RunGraphOnViewNode},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions, RenderPhase},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::{ExtractedView, ViewUniforms},
    Extract, RenderApp, RenderSet,
};
use bevy_sprite::{SpriteAssetEvents, TextureAtlas};
#[cfg(feature = "bevy_text")]
use bevy_text::{PositionedGlyph, Text, TextLayoutInfo};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd, HashMap};
use bytemuck::{Pod, Zeroable};
use std::ops::Range;

use self::instances::{BatchType, UiInstanceBuffer, UiInstanceBuffers, ExtractedInstance};

pub mod node {
    pub const UI_PASS_DRIVER: &str = "ui_pass_driver";
}

pub mod draw_ui_graph {
    pub const NAME: &str = "draw_ui";
    pub mod node {
        pub const UI_PASS: &str = "ui_pass";
    }
}

pub const UI_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(13012847047162779583);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderUiSystem {
    ExtractNode,
    ExtractBorder,
    ExtractOutline,
    ExtractAtlasNode,
    ExtractText,
}

pub fn build_ui_render(app: &mut App) {
    load_internal_asset!(app, UI_SHADER_HANDLE, "ui.wgsl", Shader::from_wgsl);

    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
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
                extract_atlas_uinodes
                    .in_set(RenderUiSystem::ExtractAtlasNode)
                    .after(RenderUiSystem::ExtractNode),
                #[cfg(feature = "bevy_text")]
                extract_text_uinodes
                    .in_set(RenderUiSystem::ExtractText)
                    .after(RenderUiSystem::ExtractAtlasNode),
                extract_uinode_borders
                    .in_set(RenderUiSystem::ExtractBorder)
                    .after(RenderUiSystem::ExtractAtlasNode)
                    .after(RenderUiSystem::ExtractText),
                extract_uinode_outlines
                    .in_set(RenderUiSystem::ExtractOutline)
                    .after(RenderUiSystem::ExtractBorder),
            ),
        )
        .add_systems(
            Render,
            (
                queue_uinodes.in_set(RenderSet::Queue),
                sort_phase_system::<TransparentUi>.in_set(RenderSet::PhaseSort),
                prepare_uinodes.in_set(RenderSet::PrepareBindGroups),
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

pub fn extract_atlas_uinodes(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    uinode_query: Extract<
        Query<
            (
                Entity,
                &Node,
                &GlobalTransform,
                &BackgroundColor,
                &ViewVisibility,
                Option<&CalculatedClip>,
                &Handle<TextureAtlas>,
                &UiTextureAtlasImage,
            ),
            Without<UiImage>,
        >,
    >,
) {
    for (
        entity,
        uinode,
        transform,
        color,
        view_visibility,
        clip,
        texture_atlas_handle,
        atlas_image,
    ) in uinode_query.iter()
    {
        // Skip invisible and completely transparent nodes
        if !view_visibility.get() || color.0.is_fully_transparent() {
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

        atlas_rect.min /= atlas_size;
        atlas_rect.max /= atlas_size;

        let color = match &color.0 {
            UiColor::Color(color) => *color,
            _ => Color::NONE,
        };

        extracted_uinodes.push_node(
            entity,
            uinode.stack_index as usize,
            uinode.position.into(),
            uinode.size().into(),
            Some(image.id()),
            atlas_rect,
            color,
            uinode.border,
            uinode.border_radius,
            clip.map(|clip| clip.clip),
        );
    }
}

pub(crate) fn resolve_border_thickness(value: Val, parent_width: f32, viewport_size: Vec2) -> f32 {
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
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    ui_scale: Extract<Res<UiScale>>,
    uinode_query: Extract<
        Query<
            (
                Entity,
                &Node,
                &GlobalTransform,
                &Style,
                &BorderColor,
                Option<&Parent>,
                &ViewVisibility,
                Option<&CalculatedClip>,
            ),
            Without<ContentSize>,
        >,
    >,
    node_query: Extract<Query<&Node>>,
) {
    let image = AssetId::<Image>::default();

    let viewport_size = windows
        .get_single()
        .map(|window| Vec2::new(window.resolution.width(), window.resolution.height()))
        .unwrap_or(Vec2::ZERO)
        // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
        // so we have to divide by `UiScale` to get the size of the UI viewport.
        / ui_scale.0;

    for (entity, uinode, global_transform, style, border_color, parent, view_visibility, clip) in
        uinode_query.iter()
    {
        // Skip invisible borders
        if !view_visibility.get()
            || border_color.0.is_fully_transparent()
            || uinode.size().x <= 0.
            || uinode.size().y <= 0.
        {
            continue;
        }

        let size = uinode.size();
        let position = uinode.position();
        let border = uinode.border;

        let transform = global_transform.compute_matrix();

        
        match &border_color.0 {
            UiColor::Color(color) => {
                extracted_uinodes.push_border(
                    entity, uinode.stack_index as usize,
                    position,
                    size,
                    *color,
                    uinode.border,
                    uinode.border_radius,
                    clip.map(|clip| clip.clip),
                );
            }
            UiColor::LinearGradient(l) => {
                let (start_point, length) = l.resolve_geometry(uinode.rect());
                let stops = resolve_color_stops(&l.stops, length, viewport_size);
                extracted_uinodes.push_border_with_linear_gradient(
                    entity, uinode.stack_index as usize,
                    position,
                    size,
                    border,
                    uinode.border_radius,
                    start_point,
                    l.angle,
                    &stops,
                    clip.map(|clip| clip.clip),
                );
            }
            UiColor::RadialGradient(r) => {
                let ellipse = r.resolve_geometry(uinode.rect(), viewport_size);
                let stops = resolve_color_stops(&r.stops, ellipse.extents.x, viewport_size);
                extracted_uinodes.push_border_with_radial_gradient(
                    entity, uinode.stack_index as usize,
                    position,
                    size,
                    border,
                    uinode.border_radius,
                    ellipse,
                    &stops,
                    clip.map(|clip| clip.clip),
                );
            }
        }
    }
}

pub fn extract_uinode_outlines(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &Node,
            &GlobalTransform,
            &Outline,
            Option<&OutlineStyle>,
            &ViewVisibility,
            Option<&CalculatedClip>,
        )>,
    >,
) {
    let image = AssetId::<Image>::default();
    for (entity, uinode, global_transform, outline, maybe_outline_style, view_visibility, maybe_clip) in uinode_query.iter() {
        // Skip invisible outlines
        if !view_visibility.get()
            || outline.color.is_fully_transparent()
            || uinode.outline_width == 0.
        {
            continue;
        }

        match maybe_outline_style.unwrap_or(&OutlineStyle::Solid) {
            OutlineStyle::Solid => {
                extracted_uinodes.push_border(
                    entity,
                    uinode.stack_index as usize,
                    uinode.position()
                        - Vec2::splat(uinode.outline_offset + uinode.outline_width),
                    uinode.size() + 2. * (uinode.outline_width + uinode.outline_offset),
                    outline.color,
                    [uinode.outline_width; 4],
                    uinode.border_radius.map(|r| {
                        if r <= 0. {
                            0.
                        } else {
                            r + uinode.outline_offset + uinode.outline_width
                        }
                    }),
                    maybe_clip.map(|clip| clip.clip),
                );
            }
            OutlineStyle::Dashed {
                dash_length,
                break_length,
            } => {
                let dl = if let Val::Px(dl) = *dash_length {
                    dl
                } else {
                    10.
                };
                let bl = if let Val::Px(bl) = *break_length {
                    bl
                } else {
                    dl
                };
                extracted_uinodes.push_dashed_border(
                    entity,
                    uinode.stack_index as usize,
                    uinode.position()
                        - Vec2::splat(uinode.outline_offset + uinode.outline_width),
                    uinode.size() + 2. * (uinode.outline_width + uinode.outline_offset),
                    outline.color,
                    uinode.outline_width,
                    dl,
                    bl,
                    uinode.border_radius.map(|r| {
                        if r <= 0. {
                            0.
                        } else {
                            r + uinode.outline_offset + uinode.outline_width
                        }
                    }),
                    maybe_clip.map(|clip| clip.clip),
                )
            }
        }
    }
}

pub fn extract_uinodes(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    ui_scale: Extract<Res<UiScale>>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
        uinode_query: Extract<
        Query<
            (
                Entity,
                &Node,
                &GlobalTransform,
                &BackgroundColor,
                Option<&UiImage>,
                &ViewVisibility,
                Option<&CalculatedClip>,
            ),
            Without<UiTextureAtlasImage>,
        >,
    >,
) {
    let viewport_size = windows
    .get_single()
    .map(|window| vec2(window.resolution.width(), window.resolution.height()))
    .unwrap_or(Vec2::ZERO)
    / ui_scale.0 as f32;

    for (entity, uinode, transform, color, maybe_image, view_visibility, clip) in
        uinode_query.iter()
    {
        // Skip invisible and completely transparent nodes
        if !view_visibility.get()
            || color.0.is_fully_transparent()
            || uinode.size().x <= 0.
            || uinode.size().y <= 0.
        {
            continue;
        }

        let (image, flip_x, flip_y) = if let Some(image) = maybe_image {
            // Skip loading images
            if !images.contains(&image.texture) {
                continue;
            }
            (Some(image.texture.id()), image.flip_x, image.flip_y)
        } else {
            (None, false, false)
        };

        match &color.0 {
            UiColor::Color(color) => {
                extracted_uinodes.push_node(
                    entity, uinode.stack_index as usize,
                    uinode.position,
                    uinode.size(),
                    image,
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                    *color,
                    uinode.border,
                    uinode.border_radius,
                    clip.map(|clip| clip.clip),
                );
            }
            UiColor::LinearGradient(l) => {
                let (start_point, length) = l.resolve_geometry(uinode.rect());
                let stops = resolve_color_stops(&l.stops, length, viewport_size);
                extracted_uinodes.push_node_with_linear_gradient(
                    entity, uinode.stack_index as usize,
                    uinode.position,
                    uinode.size(),
                    uinode.border,
                    uinode.border_radius,
                    start_point,
                    l.angle,
                    &stops,
                    clip.map(|clip| clip.clip),
                );
            }
            UiColor::RadialGradient(r) => {
                let ellipse = r.resolve_geometry(uinode.rect(), viewport_size);
                let stops = resolve_color_stops(&r.stops, ellipse.extents.x, viewport_size);
                extracted_uinodes.push_node_with_radial_gradient(
                    entity, uinode.stack_index as usize,
                    uinode.position,
                    uinode.size(),
                    uinode.border(),
                    uinode.border_radius,
                    ellipse,
                    &stops,
                    clip.map(|clip| clip.clip),
                );
            }
        }
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
    let scale = ui_scale.0.recip();
    for (entity, camera, camera_ui) in &query {
        // ignore cameras with disabled ui
        if matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. })) {
            continue;
        }
        // ignore inactive cameras
        if !camera.is_active {
            continue;
        }

        if let (
            Some(logical_size),
            Some(URect {
                min: physical_origin,
                ..
            }),
            Some(physical_size),
        ) = (
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
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    ui_scale: Extract<Res<UiScale>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &Node,
            &GlobalTransform,
            &Text,
            &TextLayoutInfo,
            &ViewVisibility,
            Option<&CalculatedClip>,
        )>,
    >,
) {
    // TODO: Support window-independent UI scale: https://github.com/bevyengine/bevy/issues/5621

    let scale_factor = windows
        .get_single()
        .map(|window| window.scale_factor())
        .unwrap_or(1.)
        * ui_scale.0;

    let inverse_scale_factor = scale_factor.recip();

    for (entity, uinode, global_transform, text, text_layout_info, view_visibility, clip) in
        uinode_query.iter()
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !view_visibility.get() || uinode.size().x == 0. || uinode.size().y == 0. {
            continue;
        }

        let node_position =
        (uinode.position() * scale_factor as f32).round() / scale_factor as f32;


        let mut color = Color::WHITE;
        let mut current_section = usize::MAX;
        for PositionedGlyph {
            position: glyph_position,
            atlas_info,
            section_index,
            ..
        } in &text_layout_info.glyphs
        {
            if *section_index != current_section {
                color = text.sections[*section_index].style.color.as_rgba_linear();
                current_section = *section_index;
            }
            if let Some(atlas) = texture_atlases.get(&atlas_info.texture_atlas) {

            let mut uv_rect = atlas.textures[atlas_info.glyph_index];
            let scaled_glyph_size = uv_rect.size() * inverse_scale_factor;
            let scaled_glyph_position = *glyph_position * inverse_scale_factor;
            uv_rect.min /= atlas.size;
            uv_rect.max /= atlas.size;

            let position = node_position + scaled_glyph_position - 0.5 * scaled_glyph_size;


            extracted_uinodes.push_glyph(
                entity,
                uinode.stack_index as usize,
                position,
                scaled_glyph_size,
                atlas.texture.id(),
                color,
                clip.map(|clip| clip.clip),
                uv_rect,
            );

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
    pub view_bind_group: Option<BindGroup>,
    pub index_buffer: BufferVec<u32>,
    pub instance_buffers: UiInstanceBuffers,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            index_buffer: BufferVec::<u32>::new(BufferUsages::INDEX),
            instance_buffers: Default::default(),
        }
    }
}

impl UiMeta {
    fn clear_instance_buffers(&mut self) {
        self.instance_buffers.clear_all();
    }

    fn write_instance_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        self.instance_buffers.write_all(render_device, render_queue);
    }

    fn push(&mut self, item: &ExtractedInstance) {
        item.push(&mut self.instance_buffers);
    }
}

pub(crate) const QUAD_VERTEX_POSITIONS: [Vec3; 4] = [
    Vec3::new(-0.5, -0.5, 0.0),
    Vec3::new(0.5, -0.5, 0.0),
    Vec3::new(0.5, 0.5, 0.0),
    Vec3::new(-0.5, 0.5, 0.0),
];

pub(crate) const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component)]
pub struct UiBatch {
    pub batch_type: BatchType,
    pub range: Range<u32>,
    pub image: AssetId<Image>,
    pub stack_index: u32,
}

const UNTEXTURED_QUAD: u32 = 0;
const TEXTURED_QUAD: u32 = 1;
const BORDERED: u32 = 32;
const FILL_START: u32 = 64;
const FILL_END: u32 = 128;

#[allow(clippy::too_many_arguments)]
pub fn queue_uinodes(
    extracted_uinodes: Res<ExtractedUiNodes>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<TransparentUi>)>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();
    for (view, mut transparent_phase) in &mut views {
        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_pipeline,
            UiPipelineKey {
                hdr: view.hdr,
                clip: false,
                specialization: UiPipelineSpecialization::Node,
            },
        );
        transparent_phase
            .items
            .reserve(extracted_uinodes.uinodes.len());

        for (entity, extracted_uinode) in extracted_uinodes.uinodes.iter() {
            transparent_phase.add(TransparentUi {
                draw_function,
                pipeline,
                entity: *entity,
                sort_key: (
                    FloatOrd(extracted_uinode.stack_index as f32),
                    entity.index(),
                ),
                // batch_range will be calculated in prepare_uinodes
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}

#[derive(Resource, Default)]
pub struct UiImageBindGroups {
    pub values: HashMap<AssetId<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_uinodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMeta>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    view_uniforms: Res<ViewUniforms>,
    ui_pipeline: Res<UiPipeline>,
    mut image_bind_groups: ResMut<UiImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    mut phases: Query<&mut RenderPhase<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
    mut previous_len: Local<usize>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.clear_instance_buffers();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_view_bind_group",
            &ui_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Vertex buffer index
        let mut index = 0;
        for mut ui_phase in &mut phases {
            let mut batch_item_index = 0;
            let mut batch_image_handle = AssetId::invalid();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(extracted_uinode) = extracted_uinodes.uinodes.get(&item.entity) {
                    let mut existing_batch = batches.last_mut();

                    if batch_image_handle == AssetId::invalid()
                        || existing_batch.is_none()
                        || (batch_image_handle != AssetId::default()
                            && extracted_uinode.image != AssetId::default()
                            && batch_image_handle != extracted_uinode.image)
                    {
                        if let Some(gpu_image) = gpu_images.get(extracted_uinode.image) {
                            batch_item_index = item_index;
                            batch_image_handle = extracted_uinode.image;

                            let new_batch = UiBatch {
                                batch_type: extracted_uinode.instance.get_type(),
                                image: extracted_uinode.image.clone(),
                                stack_index: extracted_uinode.stack_index,
                                range: index - 1..index,
                            };

                            batches.push((item.entity, new_batch));

                            image_bind_groups
                                .values
                                .entry(batch_image_handle)
                                .or_insert_with(|| {
                                    render_device.create_bind_group(
                                        "ui_material_bind_group",
                                        &ui_pipeline.image_layout,
                                        &BindGroupEntries::sequential((
                                            &gpu_image.texture_view,
                                            &gpu_image.sampler,
                                        )),
                                    )
                                });

                            existing_batch = batches.last_mut();
                        } else {
                            continue;
                        }
                    } else if batch_image_handle == AssetId::default()
                        && extracted_uinode.image != AssetId::default()
                    {
                        if let Some(gpu_image) = gpu_images.get(extracted_uinode.image) {
                            batch_image_handle = extracted_uinode.image;
                            existing_batch.as_mut().unwrap().1.image = extracted_uinode.image;

                            image_bind_groups
                                .values
                                .entry(batch_image_handle)
                                .or_insert_with(|| {
                                    render_device.create_bind_group(
                                        "ui_material_bind_group",
                                        &ui_pipeline.image_layout,
                                        &BindGroupEntries::sequential((
                                            &gpu_image.texture_view,
                                            &gpu_image.sampler,
                                        )),
                                    )
                                });
                        } else {
                            continue;
                        }
                    }

                    let mode = if extracted_uinode.image != AssetId::default() {
                        TEXTURED_QUAD
                    } else {
                        UNTEXTURED_QUAD
                    };

                    let mut uinode_rect = extracted_uinode.rect;

                    let rect_size = uinode_rect.size().extend(1.0);

                    // Specify the corners of the node
                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        (extracted_uinode.transform * (pos * rect_size).extend(1.)).xyz()
                    });

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
                    index += QUAD_INDICES.len() as u32;
                    existing_batch.unwrap().1.range.end = index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else {
                    batch_image_handle = AssetId::invalid();
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
    extracted_uinodes.uinodes.clear();
}

pub(crate) fn rect_to_f32_4(r: Rect) -> [f32; 4] {
    [r.min.x, r.min.y, r.max.x, r.max.y]
}
