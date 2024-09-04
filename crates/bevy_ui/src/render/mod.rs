mod pipeline;
mod render_pass;
mod ui_material_pipeline;
pub mod ui_texture_slice_pipeline;

use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_core_pipeline::{core_2d::Camera2d, core_3d::Camera3d};
use bevy_hierarchy::Parent;
use bevy_render::render_phase::ViewSortedRenderPhases;
use bevy_render::{
    render_phase::{PhaseItem, PhaseItemExtraIndex},
    texture::GpuImage,
    view::ViewVisibility,
    ExtractSchedule, Render,
};
use bevy_sprite::{ImageScaleMode, SpriteAssetEvents, TextureAtlas};
pub use pipeline::*;
pub use render_pass::*;
pub use ui_material_pipeline::*;
use ui_texture_slice_pipeline::UiTextureSlicerPlugin;

use crate::graph::{NodeUi, SubGraphUi};
use crate::{
    BackgroundColor, BorderColor, CalculatedClip, DefaultUiCamera, Display, Node, Outline, Style,
    TargetCamera, UiImage, UiScale, Val,
};

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::entity::{EntityHashMap, EntityHashSet};
use bevy_ecs::prelude::*;
use bevy_math::{FloatOrd, Mat4, Rect, URect, UVec4, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use bevy_render::{
    camera::Camera,
    render_asset::RenderAssets,
    render_graph::{RenderGraph, RunGraphOnViewNode},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::{ExtractedView, ViewUniforms},
    Extract, RenderApp, RenderSet,
};
use bevy_sprite::TextureAtlasLayout;
#[cfg(feature = "bevy_text")]
use bevy_text::{PositionedGlyph, Text, TextLayoutInfo};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};
use std::ops::Range;

pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct SubGraphUi;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodeUi {
        UiPass,
    }
}

pub const UI_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(13012847047162779583);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderUiSystem {
    ExtractBackgrounds,
    ExtractImages,
    ExtractBorders,
    ExtractText,
}

pub fn build_ui_render(app: &mut App) {
    load_internal_asset!(app, UI_SHADER_HANDLE, "ui.wgsl", Shader::from_wgsl);

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app
        .init_resource::<SpecializedRenderPipelines<UiPipeline>>()
        .init_resource::<UiImageBindGroups>()
        .init_resource::<UiMeta>()
        .init_resource::<ExtractedUiNodes>()
        .allow_ambiguous_resource::<ExtractedUiNodes>()
        .init_resource::<DrawFunctions<TransparentUi>>()
        .init_resource::<ViewSortedRenderPhases<TransparentUi>>()
        .add_render_command::<TransparentUi, DrawUi>()
        .configure_sets(
            ExtractSchedule,
            (
                RenderUiSystem::ExtractBackgrounds,
                RenderUiSystem::ExtractImages,
                RenderUiSystem::ExtractBorders,
                RenderUiSystem::ExtractText,
            )
                .chain(),
        )
        .add_systems(
            ExtractSchedule,
            (
                extract_default_ui_camera_view,
                extract_uinode_background_colors.in_set(RenderUiSystem::ExtractBackgrounds),
                extract_uinode_images.in_set(RenderUiSystem::ExtractImages),
                extract_uinode_borders.in_set(RenderUiSystem::ExtractBorders),
                #[cfg(feature = "bevy_text")]
                extract_uinode_text.in_set(RenderUiSystem::ExtractText),
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
    let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();

    if let Some(graph_2d) = graph.get_sub_graph_mut(Core2d) {
        graph_2d.add_sub_graph(SubGraphUi, ui_graph_2d);
        graph_2d.add_node(NodeUi::UiPass, RunGraphOnViewNode::new(SubGraphUi));
        graph_2d.add_node_edge(Node2d::EndMainPass, NodeUi::UiPass);
        graph_2d.add_node_edge(Node2d::EndMainPassPostProcessing, NodeUi::UiPass);
        graph_2d.add_node_edge(NodeUi::UiPass, Node2d::Upscaling);
    }

    if let Some(graph_3d) = graph.get_sub_graph_mut(Core3d) {
        graph_3d.add_sub_graph(SubGraphUi, ui_graph_3d);
        graph_3d.add_node(NodeUi::UiPass, RunGraphOnViewNode::new(SubGraphUi));
        graph_3d.add_node_edge(Node3d::EndMainPass, NodeUi::UiPass);
        graph_3d.add_node_edge(Node3d::EndMainPassPostProcessing, NodeUi::UiPass);
        graph_3d.add_node_edge(NodeUi::UiPass, Node3d::Upscaling);
    }

    app.add_plugins(UiTextureSlicerPlugin);
}

fn get_ui_graph(render_app: &mut SubApp) -> RenderGraph {
    let ui_pass_node = UiPassNode::new(render_app.world_mut());
    let mut ui_graph = RenderGraph::default();
    ui_graph.add_node(NodeUi::UiPass, ui_pass_node);
    ui_graph
}

/// The type of UI node.
/// This is used to determine how to render the UI node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeType {
    Rect,
    Border,
}

pub struct ExtractedUiNode {
    pub stack_index: u32,
    pub transform: Mat4,
    pub color: LinearRgba,
    pub rect: Rect,
    pub image: AssetId<Image>,
    pub atlas_size: Option<Vec2>,
    pub clip: Option<Rect>,
    pub flip_x: bool,
    pub flip_y: bool,
    // Camera to render this UI node to. By the time it is extracted,
    // it is defaulted to a single camera if only one exists.
    // Nodes with ambiguous camera will be ignored.
    pub camera_entity: Entity,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    pub border_radius: [f32; 4],
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    pub border: [f32; 4],
    pub node_type: NodeType,
}

#[derive(Resource, Default)]
pub struct ExtractedUiNodes {
    pub uinodes: EntityHashMap<ExtractedUiNode>,
}

pub fn extract_uinode_background_colors(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_query: Extract<Query<(Entity, &Camera)>>,
    default_ui_camera: Extract<DefaultUiCamera>,
    ui_scale: Extract<Res<UiScale>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &Node,
            &GlobalTransform,
            &ViewVisibility,
            Option<&CalculatedClip>,
            Option<&TargetCamera>,
            &BackgroundColor,
            &Style,
            Option<&Parent>,
        )>,
    >,
    node_query: Extract<Query<&Node>>,
) {
    for (
        entity,
        uinode,
        transform,
        view_visibility,
        clip,
        camera,
        background_color,
        style,
        parent,
    ) in &uinode_query
    {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };

        // Skip invisible backgrounds
        if !view_visibility.get() || background_color.0.is_fully_transparent() {
            continue;
        }

        let ui_logical_viewport_size = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|(_, c)| c.logical_viewport_size())
            .unwrap_or(Vec2::ZERO)
            // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
            // so we have to divide by `UiScale` to get the size of the UI viewport.
            / ui_scale.0;

        // Both vertical and horizontal percentage border values are calculated based on the width of the parent node
        // <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
        let parent_width = parent
            .and_then(|parent| node_query.get(parent.get()).ok())
            .map(|parent_node| parent_node.size().x)
            .unwrap_or(ui_logical_viewport_size.x);
        let left =
            resolve_border_thickness(style.border.left, parent_width, ui_logical_viewport_size);
        let right =
            resolve_border_thickness(style.border.right, parent_width, ui_logical_viewport_size);
        let top =
            resolve_border_thickness(style.border.top, parent_width, ui_logical_viewport_size);
        let bottom =
            resolve_border_thickness(style.border.bottom, parent_width, ui_logical_viewport_size);

        let border = [left, top, right, bottom];

        let border_radius = [
            uinode.border_radius.top_left,
            uinode.border_radius.top_right,
            uinode.border_radius.bottom_right,
            uinode.border_radius.bottom_left,
        ]
        .map(|r| r * ui_scale.0);

        extracted_uinodes.uinodes.insert(
            entity,
            ExtractedUiNode {
                stack_index: uinode.stack_index,
                transform: transform.compute_matrix(),
                color: background_color.0.into(),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: uinode.calculated_size,
                },
                clip: clip.map(|clip| clip.clip),
                image: AssetId::default(),
                atlas_size: None,
                flip_x: false,
                flip_y: false,
                camera_entity,
                border,
                border_radius,
                node_type: NodeType::Rect,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn extract_uinode_images(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_query: Extract<Query<(Entity, &Camera)>>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    ui_scale: Extract<Res<UiScale>>,
    default_ui_camera: Extract<DefaultUiCamera>,
    uinode_query: Extract<
        Query<
            (
                &Node,
                &GlobalTransform,
                &ViewVisibility,
                Option<&CalculatedClip>,
                Option<&TargetCamera>,
                &UiImage,
                Option<&TextureAtlas>,
                Option<&Parent>,
                &Style,
            ),
            Without<ImageScaleMode>,
        >,
    >,
    node_query: Extract<Query<&Node>>,
) {
    for (uinode, transform, view_visibility, clip, camera, image, atlas, parent, style) in
        &uinode_query
    {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };

        // Skip invisible images
        if !view_visibility.get() || image.color.is_fully_transparent() {
            continue;
        }

        let (rect, atlas_size) = match atlas {
            Some(atlas) => {
                let Some(layout) = texture_atlases.get(&atlas.layout) else {
                    // Atlas not present in assets resource (should this warn the user?)
                    continue;
                };
                let mut atlas_rect = layout.textures[atlas.index].as_rect();
                let mut atlas_size = layout.size.as_vec2();
                let scale = uinode.size() / atlas_rect.size();
                atlas_rect.min *= scale;
                atlas_rect.max *= scale;
                atlas_size *= scale;
                (atlas_rect, Some(atlas_size))
            }
            None => (
                Rect {
                    min: Vec2::ZERO,
                    max: uinode.calculated_size,
                },
                None,
            ),
        };

        let ui_logical_viewport_size = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|(_, c)| c.logical_viewport_size())
            .unwrap_or(Vec2::ZERO)
            // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
            // so we have to divide by `UiScale` to get the size of the UI viewport.
            / ui_scale.0;

        // Both vertical and horizontal percentage border values are calculated based on the width of the parent node
        // <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
        let parent_width = parent
            .and_then(|parent| node_query.get(parent.get()).ok())
            .map(|parent_node| parent_node.size().x)
            .unwrap_or(ui_logical_viewport_size.x);
        let left =
            resolve_border_thickness(style.border.left, parent_width, ui_logical_viewport_size);
        let right =
            resolve_border_thickness(style.border.right, parent_width, ui_logical_viewport_size);
        let top =
            resolve_border_thickness(style.border.top, parent_width, ui_logical_viewport_size);
        let bottom =
            resolve_border_thickness(style.border.bottom, parent_width, ui_logical_viewport_size);

        let border = [left, top, right, bottom];

        let border_radius = [
            uinode.border_radius.top_left,
            uinode.border_radius.top_right,
            uinode.border_radius.bottom_right,
            uinode.border_radius.bottom_left,
        ]
        .map(|r| r * ui_scale.0);

        extracted_uinodes.uinodes.insert(
            commands.spawn_empty().id(),
            ExtractedUiNode {
                stack_index: uinode.stack_index,
                transform: transform.compute_matrix(),
                color: image.color.into(),
                rect,
                clip: clip.map(|clip| clip.clip),
                image: image.texture.id(),
                atlas_size,
                flip_x: image.flip_x,
                flip_y: image.flip_y,
                camera_entity,
                border,
                border_radius,
                node_type: NodeType::Rect,
            },
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

#[inline]
fn clamp_corner(r: f32, size: Vec2, offset: Vec2) -> f32 {
    let s = 0.5 * size + offset;
    let sm = s.x.min(s.y);
    r.min(sm)
}

#[inline]
fn clamp_radius(
    [top_left, top_right, bottom_right, bottom_left]: [f32; 4],
    size: Vec2,
    border: Vec4,
) -> [f32; 4] {
    let s = size - border.xy() - border.zw();
    [
        clamp_corner(top_left, s, border.xy()),
        clamp_corner(top_right, s, border.zy()),
        clamp_corner(bottom_right, s, border.zw()),
        clamp_corner(bottom_left, s, border.xw()),
    ]
}

pub fn extract_uinode_borders(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_query: Extract<Query<(Entity, &Camera)>>,
    default_ui_camera: Extract<DefaultUiCamera>,
    ui_scale: Extract<Res<UiScale>>,
    uinode_query: Extract<
        Query<(
            &Node,
            &GlobalTransform,
            &ViewVisibility,
            Option<&CalculatedClip>,
            Option<&TargetCamera>,
            Option<&Parent>,
            &Style,
            AnyOf<(&BorderColor, &Outline)>,
        )>,
    >,
    node_query: Extract<Query<&Node>>,
) {
    let image = AssetId::<Image>::default();

    for (
        uinode,
        global_transform,
        view_visibility,
        maybe_clip,
        maybe_camera,
        maybe_parent,
        style,
        (maybe_border_color, maybe_outline),
    ) in &uinode_query
    {
        let Some(camera_entity) = maybe_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
        else {
            continue;
        };

        // Skip invisible borders
        if !view_visibility.get()
            || style.display == Display::None
            || maybe_border_color.is_some_and(|border_color| border_color.0.is_fully_transparent())
                && maybe_outline.is_some_and(|outline| outline.color.is_fully_transparent())
        {
            continue;
        }

        let ui_logical_viewport_size = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|(_, c)| c.logical_viewport_size())
            .unwrap_or(Vec2::ZERO)
            // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
            // so we have to divide by `UiScale` to get the size of the UI viewport.
            / ui_scale.0;

        // Both vertical and horizontal percentage border values are calculated based on the width of the parent node
        // <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
        let parent_width = maybe_parent
            .and_then(|parent| node_query.get(parent.get()).ok())
            .map(|parent_node| parent_node.size().x)
            .unwrap_or(ui_logical_viewport_size.x);
        let left =
            resolve_border_thickness(style.border.left, parent_width, ui_logical_viewport_size);
        let right =
            resolve_border_thickness(style.border.right, parent_width, ui_logical_viewport_size);
        let top =
            resolve_border_thickness(style.border.top, parent_width, ui_logical_viewport_size);
        let bottom =
            resolve_border_thickness(style.border.bottom, parent_width, ui_logical_viewport_size);

        let border = [left, top, right, bottom];

        let border_radius = [
            uinode.border_radius.top_left,
            uinode.border_radius.top_right,
            uinode.border_radius.bottom_right,
            uinode.border_radius.bottom_left,
        ]
        .map(|r| r * ui_scale.0);

        let border_radius = clamp_radius(border_radius, uinode.size(), border.into());

        // don't extract border if no border or the node is zero-sized (a zero sized node can still have an outline).
        if uinode.size().x > 0. && uinode.size().y > 0. && border != [0.; 4] {
            if let Some(border_color) = maybe_border_color {
                extracted_uinodes.uinodes.insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        stack_index: uinode.stack_index,
                        transform: global_transform.compute_matrix(),
                        color: border_color.0.into(),
                        rect: Rect {
                            max: uinode.size(),
                            ..Default::default()
                        },
                        image,
                        atlas_size: None,
                        clip: maybe_clip.map(|clip| clip.clip),
                        flip_x: false,
                        flip_y: false,
                        camera_entity,
                        border_radius,
                        border,
                        node_type: NodeType::Border,
                    },
                );
            }
        }

        if let Some(outline) = maybe_outline {
            let outer_distance = uinode.outline_offset() + uinode.outline_width();
            let outline_radius = border_radius.map(|radius| {
                if radius > 0. {
                    radius + outer_distance
                } else {
                    0.
                }
            });
            let outline_size = uinode.size() + 2. * outer_distance;
            extracted_uinodes.uinodes.insert(
                commands.spawn_empty().id(),
                ExtractedUiNode {
                    stack_index: uinode.stack_index,
                    transform: global_transform.compute_matrix(),
                    color: outline.color.into(),
                    rect: Rect {
                        max: outline_size,
                        ..Default::default()
                    },
                    image,
                    atlas_size: None,
                    clip: maybe_clip.map(|clip| clip.clip),
                    flip_x: false,
                    flip_y: false,
                    camera_entity,
                    border: [uinode.outline_width(); 4],
                    border_radius: outline_radius,
                    node_type: NodeType::Border,
                },
            );
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

/// Extracts all UI elements associated with a camera into the render world.
pub fn extract_default_ui_camera_view(
    mut commands: Commands,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    ui_scale: Extract<Res<UiScale>>,
    query: Extract<Query<(Entity, &Camera), Or<(With<Camera2d>, With<Camera3d>)>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();

    let scale = ui_scale.0.recip();
    for (entity, camera) in &query {
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
                    clip_from_view: projection_matrix,
                    world_from_view: GlobalTransform::from_xyz(
                        0.0,
                        0.0,
                        UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
                    ),
                    clip_from_world: None,
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
            commands
                .get_or_spawn(entity)
                .insert(DefaultCameraView(default_camera_view));
            transparent_render_phases.insert_or_clear(entity);

            live_entities.insert(entity);
        }
    }

    transparent_render_phases.retain(|entity, _| live_entities.contains(entity));
}

#[cfg(feature = "bevy_text")]
pub fn extract_uinode_text(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_query: Extract<Query<(Entity, &Camera)>>,
    default_ui_camera: Extract<DefaultUiCamera>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    ui_scale: Extract<Res<UiScale>>,
    uinode_query: Extract<
        Query<(
            &Node,
            &GlobalTransform,
            &ViewVisibility,
            Option<&CalculatedClip>,
            Option<&TargetCamera>,
            &Text,
            &TextLayoutInfo,
        )>,
    >,
) {
    for (uinode, global_transform, view_visibility, clip, camera, text, text_layout_info) in
        &uinode_query
    {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };

        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !view_visibility.get() || uinode.size().x == 0. || uinode.size().y == 0. {
            continue;
        }

        let scale_factor = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|(_, c)| c.target_scaling_factor())
            .unwrap_or(1.0)
            * ui_scale.0;
        let inverse_scale_factor = scale_factor.recip();

        // Align the text to the nearest physical pixel:
        // * Translate by minus the text node's half-size
        //      (The transform translates to the center of the node but the text coordinates are relative to the node's top left corner)
        // * Multiply the logical coordinates by the scale factor to get its position in physical coordinates
        // * Round the physical position to the nearest physical pixel
        // * Multiply by the rounded physical position by the inverse scale factor to return to logical coordinates

        let logical_top_left = -0.5 * uinode.size();

        let mut transform = global_transform.affine()
            * bevy_math::Affine3A::from_translation(logical_top_left.extend(0.));

        transform.translation *= scale_factor;
        transform.translation = transform.translation.round();
        transform.translation *= inverse_scale_factor;

        let mut color = LinearRgba::WHITE;
        let mut current_section = usize::MAX;
        for PositionedGlyph {
            position,
            atlas_info,
            section_index,
            ..
        } in &text_layout_info.glyphs
        {
            if *section_index != current_section {
                color = LinearRgba::from(text.sections[*section_index].style.color);
                current_section = *section_index;
            }
            let atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();

            let mut rect = atlas.textures[atlas_info.glyph_index].as_rect();
            rect.min *= inverse_scale_factor;
            rect.max *= inverse_scale_factor;
            extracted_uinodes.uinodes.insert(
                commands.spawn_empty().id(),
                ExtractedUiNode {
                    stack_index: uinode.stack_index,
                    transform: transform
                        * Mat4::from_translation(position.extend(0.) * inverse_scale_factor),
                    color,
                    rect,
                    image: atlas_info.texture.id(),
                    atlas_size: Some(atlas.size.as_vec2() * inverse_scale_factor),
                    clip: clip.map(|clip| clip.clip),
                    flip_x: false,
                    flip_y: false,
                    camera_entity,
                    border: [0.; 4],
                    border_radius: [0.; 4],
                    node_type: NodeType::Rect,
                },
            );
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// Shader flags to determine how to render the UI node.
    /// See [`shader_flags`] for possible values.
    pub flags: u32,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    pub radius: [f32; 4],
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    pub border: [f32; 4],
    /// Size of the UI node.
    pub size: [f32; 2],
}

#[derive(Resource)]
pub struct UiMeta {
    vertices: RawBufferVec<UiVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
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
    pub range: Range<u32>,
    pub image: AssetId<Image>,
    pub camera: Entity,
}

/// The values here should match the values for the constants in `ui.wgsl`
pub mod shader_flags {
    pub const UNTEXTURED: u32 = 0;
    pub const TEXTURED: u32 = 1;
    /// Ordering: top left, top right, bottom right, bottom left.
    pub const CORNERS: [u32; 4] = [0, 2, 2 | 4, 4];
    pub const BORDER: u32 = 8;
}

#[allow(clippy::too_many_arguments)]
pub fn queue_uinodes(
    extracted_uinodes: Res<ExtractedUiNodes>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<(Entity, &ExtractedView)>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();
    for (entity, extracted_uinode) in extracted_uinodes.uinodes.iter() {
        let Ok((view_entity, view)) = views.get_mut(extracted_uinode.camera_entity) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_pipeline,
            UiPipelineKey { hdr: view.hdr },
        );
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
            extra_index: PhaseItemExtraIndex::NONE,
        });
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
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
    mut previous_len: Local<usize>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            AssetEvent::Unused { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_view_bind_group",
            &ui_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Buffer indexes
        let mut vertices_index = 0;
        let mut indices_index = 0;

        for ui_phase in phases.values_mut() {
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
                        || existing_batch.as_ref().map(|(_, b)| b.camera)
                            != Some(extracted_uinode.camera_entity)
                    {
                        if let Some(gpu_image) = gpu_images.get(extracted_uinode.image) {
                            batch_item_index = item_index;
                            batch_image_handle = extracted_uinode.image;

                            let new_batch = UiBatch {
                                range: vertices_index..vertices_index,
                                image: extracted_uinode.image,
                                camera: extracted_uinode.camera_entity,
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

                    let mut flags = if extracted_uinode.image != AssetId::default() {
                        shader_flags::TEXTURED
                    } else {
                        shader_flags::UNTEXTURED
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
                    let uvs = if flags == shader_flags::UNTEXTURED {
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

                    let color = extracted_uinode.color.to_f32_array();
                    if extracted_uinode.node_type == NodeType::Border {
                        flags |= shader_flags::BORDER;
                    }

                    for i in 0..4 {
                        ui_meta.vertices.push(UiVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            color,
                            flags: flags | shader_flags::CORNERS[i],
                            radius: extracted_uinode.border_radius,
                            border: extracted_uinode.border,
                            size: rect_size.xy().into(),
                        });
                    }

                    for &i in &QUAD_INDICES {
                        ui_meta.indices.push(indices_index + i as u32);
                    }

                    vertices_index += 6;
                    indices_index += 4;

                    existing_batch.unwrap().1.range.end = vertices_index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else {
                    batch_image_handle = AssetId::invalid();
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        ui_meta.indices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
    extracted_uinodes.uinodes.clear();
}
