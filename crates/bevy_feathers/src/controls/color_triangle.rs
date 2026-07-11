use core::f32::consts::TAU;

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Asset, Assets};
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Has, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    template::FromTemplate,
};
use bevy_math::Vec2;
use bevy_picking::{
    events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press},
    Pickable,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy_render::render_resource::AsBindGroup;
use bevy_scene::prelude::*;
use bevy_shader::{ShaderDefVal, ShaderRef};
use bevy_ui::{
    percent, px, AlignSelf, BorderColor, BorderRadius, ComputedNode, ComputedUiRenderTargetInfo,
    Display, InteractionDisabled, Node, Outline, PositionType, UiGlobalTransform, UiRect, UiScale,
    UiSystems, UiTransform, Val2,
};
use bevy_ui_render::{prelude::UiMaterial, ui_material::MaterialNode, UiMaterialPlugin};
use bevy_ui_widgets::ValueChange;

use crate::{cursor::EntityCursor, palette, theme::ThemeBackgroundColor, tokens};

/// Constants must be the same as in color_triangle.wgsl
const RING_WIDTH: f32 = 12.0;
const SPACING: f32 = 4.0;
const MIN_HEIGHT: f32 = 100.0;
const PADDING: f32 = 4.0;
const MIN_DIAMETER: f32 = MIN_HEIGHT - 2.0 * PADDING;

/// A "color triangle" widget, which is a 2d picker that allows selecting all three components of
/// a HWB color space. It consists of a hue ring surrounding a triangle where the corners are max
/// whiteness, max blackness, and zero whiteness/blackness.
///
/// This is spawnable by inheriting it as a "scene component".
///
/// The control emits a [`ValueChange<ColorTriangleValue>`] containing the hue, whiteness and
/// blackness of the selection.
///
/// The control does not do any color space conversions internally, except when converting for
/// display.
#[derive(
    SceneComponent, FromTemplate, Debug, Reflect, Copy, PartialEq, Eq, Hash, Default, Clone,
)]
#[reflect(Component)]
#[require(ColorTriangleDragState)]
pub enum FeathersColorTriangle {
    /// Use the HWB color space.
    #[default]
    Hwb,
}

/// Component that contains the selected hue on the ring, and the whiteness and blackness
/// selected within the triangle.
///
/// This is also emitted by [`FeathersColorTriangle`] via [`ValueChange`] when the selection
/// changes.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorTriangleValue {
    /// Hue in degrees, in the range [0, 360).
    pub hue: f32,
    /// Whiteness in the range [0, 1].
    pub whiteness: f32,
    /// Blackness in the range [0, 1].
    pub blackness: f32,
}

/// Marker identifying the inner element of the color triangle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorTriangleInner;

/// Marker identifying the thumb element of the color triangle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorTriangleThumb;

/// Marker identifying the thumb element of the ring surrounding the color triangle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorTriangleRingThumb;

/// Component used to manage the state of a color triangle during dragging. A drag that
/// starts in the ring only changes the hue, and a drag that starts in the triangle only
/// changes whiteness/blackness.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct ColorTriangleDragState {
    /// The segment the most recent press landed in, or `None`.
    segment: Option<ColorTriangleSegment>,
    /// Whether a drag is in progress.
    dragging: bool,
}

/// The part of the widget a press or drag interacts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
enum ColorTriangleSegment {
    Ring,
    Triangle,
}

#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct ColorTriangleMaterialKey {
    triangle: FeathersColorTriangle,
}

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
#[bind_group_data(ColorTriangleMaterialKey)]
struct ColorTriangleMaterial {
    triangle: FeathersColorTriangle,

    #[uniform(0)]
    hue: f32,

    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    #[uniform(0)]
    _webgl2_padding_12b: bevy_math::Vec3,
}

impl From<&ColorTriangleMaterial> for ColorTriangleMaterialKey {
    fn from(material: &ColorTriangleMaterial) -> Self {
        Self {
            triangle: material.triangle,
        }
    }
}

impl UiMaterial for ColorTriangleMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_feathers/assets/shaders/color_triangle.wgsl".into()
    }

    fn specialize(
        descriptor: &mut bevy_render::render_resource::RenderPipelineDescriptor,
        key: bevy_ui_render::prelude::UiMaterialKey<Self>,
    ) {
        let triangle_def = match key.bind_group_data.triangle {
            FeathersColorTriangle::Hwb => "TRIANGLE_HWB",
        };
        descriptor.fragment.as_mut().unwrap().shader_defs =
            vec![ShaderDefVal::Bool(triangle_def.into(), true)];
    }
}

impl FeathersColorTriangle {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                min_height: px(MIN_HEIGHT),
                aspect_ratio: 1.0f32,
                flex_grow: 0.,
                flex_shrink: 1.,
                align_self: AlignSelf::FlexStart,
                padding: UiRect::all(px(PADDING)),
                border_radius: BorderRadius::all(percent(50)),
            }
            ColorTriangleValue
            ThemeBackgroundColor(tokens::COLOR_PLANE_BG)
            EntityCursor::System(bevy_window::SystemCursorIcon::Crosshair)
            Children [(
                Node {
                    align_self: AlignSelf::Stretch,
                    flex_grow: 1.0,
                }
                ColorTriangleInner
                Children [
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(0),
                            top: percent(0),
                            width: px(10),
                            height: px(10),
                            border: px(1),
                            border_radius: BorderRadius::MAX,
                        }
                        ColorTriangleThumb
                        BorderColor::all(palette::WHITE)
                        Outline {
                            width: px(1),
                            offset: px(0),
                            color: palette::BLACK
                        }
                        Pickable::IGNORE
                        UiTransform::from_translation(Val2::percent(-50., -50.),)
                    ),
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(0),
                            top: percent(0),
                            width: px(10),
                            height: px(10),
                            border: px(1),
                            border_radius: BorderRadius::MAX,
                        }
                        ColorTriangleRingThumb
                        BorderColor::all(palette::WHITE)
                        Outline {
                            width: px(1),
                            offset: px(0),
                            color: palette::BLACK
                        }
                        Pickable::IGNORE
                        UiTransform::from_translation(Val2::percent(-50., -50.),)
                    )
                ]
            )]
        }
    }
}

/// Positions of the triangle's corners for a given hue, relative to the center.
fn triangle_corners(hue_angle: f32, triangle_radius: f32) -> (Vec2, Vec2, Vec2) {
    (
        Vec2::from_angle(hue_angle) * triangle_radius,
        Vec2::from_angle(hue_angle + TAU / 3.0) * triangle_radius,
        Vec2::from_angle(hue_angle - TAU / 3.0) * triangle_radius,
    )
}

fn update_triangle_color(
    q_color_triangle: Query<(Entity, Ref<FeathersColorTriangle>, Ref<ColorTriangleValue>)>,
    q_children: Query<&Children>,
    q_material_node: Query<&MaterialNode<ColorTriangleMaterial>>,
    q_computed_node: Query<Ref<ComputedNode>>,
    mut q_node: Query<&mut Node>,
    mut r_materials: ResMut<Assets<ColorTriangleMaterial>>,
    mut commands: Commands,
) {
    for (triangle_ent, triangle, triangle_value) in q_color_triangle.iter() {
        // Find the inner entity
        let Ok(children) = q_children.get(triangle_ent) else {
            continue;
        };
        let Some(inner_ent) = children.first() else {
            continue;
        };

        let value_changed = triangle.is_changed() || triangle_value.is_changed();

        if let Ok(material_node) = q_material_node.get(*inner_ent) {
            // Node component exists, update it
            if value_changed && let Some(mut material) = r_materials.get_mut(material_node.id()) {
                // Update properties
                material.triangle = *triangle;
                material.hue = triangle_value.hue;
            }
        } else {
            // Insert new node component
            let material = r_materials.add(ColorTriangleMaterial {
                triangle: *triangle,
                hue: triangle_value.hue,
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _webgl2_padding_12b: Default::default(),
            });
            commands.entity(*inner_ent).insert(MaterialNode(material));
        }

        // The thumb positions depend on the inner node's layout size, so they also must be
        // refreshed when the node is resized, not just when the value changes.
        let Ok(inner_node) = q_computed_node.get(*inner_ent) else {
            continue;
        };
        if !value_changed && !inner_node.is_changed() {
            continue;
        }

        // Find the triangle thumb.
        let Ok(children_inner) = q_children.get(*inner_ent) else {
            continue;
        };
        let Some(thumb_ent) = children_inner.first() else {
            continue;
        };

        let Ok(mut thumb_node) = q_node.get_mut(*thumb_ent) else {
            continue;
        };

        // Ensure square aspect ratio.
        let min_side = inner_node.size().min_element();
        if min_side <= 0.0 {
            continue;
        }
        let min_side = min_side.max(MIN_DIAMETER);
        let scale = inner_node.inverse_scale_factor();
        let center = inner_node.size() * scale * 0.5;

        // Position the triangle thumb relative to the corners.
        let hue_angle = triangle_value.hue.to_radians();
        let triangle_radius = min_side * 0.5 - (RING_WIDTH + 2.0 * SPACING);
        let (hue_point, white_point, black_point) = triangle_corners(hue_angle, triangle_radius);
        let offset = (hue_point
            + (white_point - hue_point) * triangle_value.whiteness
            + (black_point - hue_point) * triangle_value.blackness)
            * scale;
        let left = px(center.x + offset.x);
        let top = px(center.y + offset.y);
        if thumb_node.left != left || thumb_node.top != top {
            thumb_node.left = left;
            thumb_node.top = top;
        }

        // Find the ring thumb.
        let Some(ring_thumb_ent) = children_inner.get(1) else {
            continue;
        };

        let Ok(mut ring_thumb_node) = q_node.get_mut(*ring_thumb_ent) else {
            continue;
        };

        // Position ring thumb centered in the ring width and at the hue value.
        let ring_offset = Vec2::from_angle(triangle_value.hue.to_radians())
            * (min_side - RING_WIDTH)
            * 0.5
            * scale;
        let ring_left = px(center.x + ring_offset.x);
        let ring_top = px(center.y + ring_offset.y);
        if ring_thumb_node.left != ring_left || ring_thumb_node.top != ring_top {
            ring_thumb_node.left = ring_left;
            ring_thumb_node.top = ring_top;
        }
    }
}

/// Determine which segment of the widget a pointer position hits, and the value the widget
/// would take at that position.
///
/// When `drag_segment` is `Some`, the value is computed for that segment regardless of where
/// the pointer is.
fn get_segment_value(
    current: &ColorTriangleValue,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    pointer_position: Vec2,
    ui_scale: f32,
    drag_segment: Option<ColorTriangleSegment>,
) -> Option<(ColorTriangleSegment, ColorTriangleValue)> {
    let inverse_transform = transform.try_inverse()?;
    let local = inverse_transform
        .transform_point2(pointer_position * node_target.scale_factor() / ui_scale);
    let min_side = node.size().min_element();
    if min_side <= 0.0 {
        return None;
    }
    let min_side = min_side.max(MIN_DIAMETER);
    let pos = local / min_side;
    let radial = pos.length();

    let triangle_radius = 0.5 - (RING_WIDTH + 2.0 * SPACING) / min_side;

    // Select segment based on distance from center, with SPACING pixels of wiggle room.
    let spacing = SPACING / min_side;
    let segment = if let Some(segment) = drag_segment {
        segment
    } else if radial <= triangle_radius + spacing {
        ColorTriangleSegment::Triangle
    } else if radial <= 0.5 + spacing {
        ColorTriangleSegment::Ring
    } else {
        return None;
    };

    let value = match segment {
        ColorTriangleSegment::Ring => {
            // Keep the current hue when radial is 0.
            if radial > 0.0 {
                ColorTriangleValue {
                    hue: pos.to_angle().rem_euclid(TAU).to_degrees(),
                    ..*current
                }
            } else {
                *current
            }
        }
        ColorTriangleSegment::Triangle => {
            let hue_angle = current.hue.to_radians();
            let (hue_point, white_point, black_point) =
                triangle_corners(hue_angle, triangle_radius);

            let area = (white_point - hue_point).perp_dot(black_point - hue_point);
            let mut whiteness = (pos - hue_point).perp_dot(black_point - hue_point) / area;
            let mut blackness = (white_point - hue_point).perp_dot(pos - hue_point) / area;

            // Clamp positions outside the triangle
            whiteness = whiteness.clamp(0.0, 1.0);
            blackness = blackness.clamp(0.0, 1.0);
            let wb = whiteness + blackness;
            if wb > 1.0 {
                whiteness /= wb;
                blackness /= wb;
            }
            ColorTriangleValue {
                hue: current.hue,
                whiteness,
                blackness,
            }
        }
    };

    Some((segment, value))
}

fn emit_color_triangle_value_change(
    commands: &mut Commands,
    source: Entity,
    value: ColorTriangleValue,
    is_final: bool,
) {
    commands.trigger(ValueChange {
        source,
        value,
        is_final,
    });
}

fn on_pointer_press(
    mut press: On<Pointer<Press>>,
    mut q_color_triangles: Query<
        (
            &ColorTriangleValue,
            &mut ColorTriangleDragState,
            Has<InteractionDisabled>,
        ),
        With<FeathersColorTriangle>,
    >,
    q_color_triangle_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorTriangleInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_triangle_inner.get(press.entity)
        && let Ok((value, mut state, disabled)) = q_color_triangles.get_mut(parent.0)
    {
        press.propagate(false);
        if !disabled {
            let segment_value = get_segment_value(
                value,
                node,
                node_target,
                transform,
                press.pointer_location.position,
                ui_scale.0,
                None,
            );
            state.segment = segment_value.map(|(segment, _)| segment);
            if let Some((_, new_value)) = segment_value {
                emit_color_triangle_value_change(&mut commands, parent.0, new_value, false);
            }
        }
    }
}

fn on_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_color_triangles: Query<
        (&mut ColorTriangleDragState, Has<InteractionDisabled>),
        With<FeathersColorTriangle>,
    >,
    q_color_triangle_inner: Query<&ChildOf, With<ColorTriangleInner>>,
) {
    if let Ok(parent) = q_color_triangle_inner.get(drag_start.entity)
        && let Ok((mut state, disabled)) = q_color_triangles.get_mut(parent.0)
    {
        drag_start.propagate(false);
        if !disabled {
            state.dragging = true;
        }
    }
}

fn on_drag(
    mut drag: On<Pointer<Drag>>,
    q_color_triangles: Query<
        (
            &ColorTriangleValue,
            &ColorTriangleDragState,
            Has<InteractionDisabled>,
        ),
        With<FeathersColorTriangle>,
    >,
    q_color_triangle_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorTriangleInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_triangle_inner.get(drag.entity)
        && let Ok((value, state, disabled)) = q_color_triangles.get(parent.0)
    {
        drag.propagate(false);
        if state.dragging
            && state.segment.is_some()
            && !disabled
            && let Some((_, new_value)) = get_segment_value(
                value,
                node,
                node_target,
                transform,
                drag.pointer_location.position,
                ui_scale.0,
                state.segment,
            )
        {
            emit_color_triangle_value_change(&mut commands, parent.0, new_value, false);
        }
    }
}

fn on_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_color_triangles: Query<
        (
            &ColorTriangleValue,
            &mut ColorTriangleDragState,
            Has<InteractionDisabled>,
        ),
        With<FeathersColorTriangle>,
    >,
    q_color_triangle_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorTriangleInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_triangle_inner.get(drag_end.entity)
        && let Ok((value, mut state, disabled)) = q_color_triangles.get_mut(parent.0)
    {
        drag_end.propagate(false);
        if state.dragging
            && state.segment.is_some()
            && !disabled
            && let Some((_, new_value)) = get_segment_value(
                value,
                node,
                node_target,
                transform,
                drag_end.pointer_location.position,
                ui_scale.0,
                state.segment,
            )
        {
            emit_color_triangle_value_change(&mut commands, parent.0, new_value, true);
        }
        state.segment = None;
        state.dragging = false;
    }
}

fn on_drag_cancel(
    drag_cancel: On<Pointer<Cancel>>,
    mut q_color_triangles: Query<&mut ColorTriangleDragState, With<FeathersColorTriangle>>,
    q_color_triangle_inner: Query<&ChildOf, With<ColorTriangleInner>>,
) {
    if let Ok(parent) = q_color_triangle_inner.get(drag_cancel.entity)
        && let Ok(mut state) = q_color_triangles.get_mut(parent.0)
    {
        state.segment = None;
        state.dragging = false;
    }
}

/// Plugin which registers the observers for updating the triangle color.
pub struct ColorTrianglePlugin;

impl Plugin for ColorTrianglePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(UiMaterialPlugin::<ColorTriangleMaterial>::default());
        // Ensure thumbs stay inside ring/triangle on next frame when layout changes
        app.add_systems(PostUpdate, update_triangle_color.before(UiSystems::Layout));
        app.add_observer(on_pointer_press)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drag_cancel);
    }
}
