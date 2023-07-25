use crate::prelude::UiCameraConfig;
use crate::{
    BackgroundColor, BorderColor, CalculatedClip, ContentSize, Node, Style, TransparentUi, UiImage,
    UiScale, UiStack, UiTextureAtlasImage, Val,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_hierarchy::Parent;
use bevy_math::{Mat4, Rect, URect, UVec4, Vec2};
use bevy_render::texture::DEFAULT_IMAGE_HANDLE;
use bevy_render::Extract;
use bevy_render::{
    camera::Camera,
    color::Color,
    render_phase::RenderPhase,
    texture::Image,
    view::{ComputedVisibility, ExtractedView},
};
use bevy_sprite::TextureAtlas;
#[cfg(feature = "bevy_text")]
use bevy_text::{PositionedGlyph, Text, TextLayoutInfo};
use bevy_transform::components::GlobalTransform;
use bevy_window::{PrimaryWindow, Window};

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
        // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
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
