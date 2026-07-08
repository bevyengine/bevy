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

/// A "color wheel" widget, which is a circular 2d picker that allows selecting two
/// components of a cylindrical color space.
///
/// This is spawnable by inheriting it as a "scene component".
///
/// The control emits a [`ValueChange<ColorWheelValue>`] containing the `hue` and `saturation` of
/// the selected position within the wheel, along with the current fixed channel value. The
/// control accepts a [`ColorWheelValue`] input value, whose `z` component provides the fixed
/// constant channel for the background gradient.
///
/// The control does not do any color space conversions internally, except when converting to
/// orthogonal coordinates for display.
#[derive(
    SceneComponent, FromTemplate, Debug, Reflect, Copy, PartialEq, Eq, Hash, Default, Clone,
)]
#[reflect(Component)]
#[require(ColorWheelDragState)]
pub enum FeathersColorWheel {
    /// Use the HSV color space.
    #[default]
    Hsv,
    /// Use the HSL color space.
    Hsl,
}

/// Component that contains the selected position within the wheel in polar form, as well as
/// the `z` value. The `hue` and `saturation` determine the placement of the thumb element,
/// while the `z` value controls the background gradient. In the cylindrical color space, `z` is
/// the position along the cylinder's axis (e.g. lightness or value).
///
/// This is also emitted by [`FeathersColorWheel`] via [`ValueChange`] when the selection
/// changes.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorWheelValue {
    /// Hue in degrees, in the range [0, 360).
    pub hue: f32,
    /// Saturation in the range [0, 1].
    pub saturation: f32,
    /// The fixed channel value in the range [0, 1].
    pub z: f32,
}

// Sensible default, particularly important for HSL spaces
impl Default for ColorWheelValue {
    fn default() -> ColorWheelValue {
        ColorWheelValue {
            hue: 0.,
            saturation: 0.,
            z: 0.5,
        }
    }
}

/// Marker identifying the inner element of the color wheel.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorWheelInner;

/// Marker identifying the thumb element of the color wheel.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorWheelThumb;

/// Component used to manage the state of a color wheel during dragging.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct ColorWheelDragState(bool);

#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct ColorWheelMaterialKey {
    wheel: FeathersColorWheel,
}

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
#[bind_group_data(ColorWheelMaterialKey)]
struct ColorWheelMaterial {
    wheel: FeathersColorWheel,

    #[uniform(0)]
    fixed_channel: f32,

    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    #[uniform(0)]
    _webgl2_padding_12b: bevy_math::Vec3,
}

impl From<&ColorWheelMaterial> for ColorWheelMaterialKey {
    fn from(material: &ColorWheelMaterial) -> Self {
        Self {
            wheel: material.wheel,
        }
    }
}

impl UiMaterial for ColorWheelMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_feathers/assets/shaders/color_wheel.wgsl".into()
    }

    fn specialize(
        descriptor: &mut bevy_render::render_resource::RenderPipelineDescriptor,
        key: bevy_ui_render::prelude::UiMaterialKey<Self>,
    ) {
        let wheel_def = match key.bind_group_data.wheel {
            FeathersColorWheel::Hsv => "WHEEL_HSV",
            FeathersColorWheel::Hsl => "WHEEL_HSL",
        };
        descriptor.fragment.as_mut().unwrap().shader_defs =
            vec![ShaderDefVal::Bool(wheel_def.into(), true)];
    }
}

impl FeathersColorWheel {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                min_height: px(100.0),
                aspect_ratio: 1.0f32,
                flex_grow: 0.,
                flex_shrink: 1.,
                align_self: AlignSelf::FlexStart,
                padding: UiRect::all(px(4)),
                border_radius: BorderRadius::all(percent(50)),
            }
            ColorWheelValue
            ThemeBackgroundColor(tokens::COLOR_PLANE_BG)
            EntityCursor::System(bevy_window::SystemCursorIcon::Crosshair)
            Children [(
                Node {
                    align_self: AlignSelf::Stretch,
                    flex_grow: 1.0,
                }
                ColorWheelInner
                Children [(
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(0),
                        top: percent(0),
                        width: px(10),
                        height: px(10),
                        border: px(1),
                        border_radius: BorderRadius::MAX,
                    }
                    ColorWheelThumb
                    BorderColor::all(palette::WHITE)
                    Outline {
                        width: px(1),
                        offset: px(0),
                        color: palette::BLACK
                    }
                    Pickable::IGNORE
                    UiTransform::from_translation(Val2::percent(-50., -50.),)
                )]
            )]
        }
    }
}

fn update_wheel_color(
    q_color_wheel: Query<(Entity, Ref<FeathersColorWheel>, Ref<ColorWheelValue>)>,
    q_children: Query<&Children>,
    q_material_node: Query<&MaterialNode<ColorWheelMaterial>>,
    q_computed_node: Query<Ref<ComputedNode>>,
    mut q_node: Query<&mut Node>,
    mut r_materials: ResMut<Assets<ColorWheelMaterial>>,
    mut commands: Commands,
) {
    for (wheel_ent, wheel, wheel_value) in q_color_wheel.iter() {
        // Find the inner entity
        let Ok(children) = q_children.get(wheel_ent) else {
            continue;
        };
        let Some(inner_ent) = children.first() else {
            continue;
        };

        let value_changed = wheel.is_changed() || wheel_value.is_changed();

        if let Ok(material_node) = q_material_node.get(*inner_ent) {
            // Node component exists, update it
            if value_changed && let Some(mut material) = r_materials.get_mut(material_node.id()) {
                // Update properties
                material.wheel = *wheel;
                material.fixed_channel = wheel_value.z;
            }
        } else {
            // Insert new node component
            let material = r_materials.add(ColorWheelMaterial {
                wheel: *wheel,
                fixed_channel: wheel_value.z,
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _webgl2_padding_12b: Default::default(),
            });
            commands.entity(*inner_ent).insert(MaterialNode(material));
        }

        // The thumb position depends on the inner node's layout size, so it also must be
        // refreshed when the node is resized, not just when the value changes.
        let Ok(inner_node) = q_computed_node.get(*inner_ent) else {
            continue;
        };
        if !value_changed && !inner_node.is_changed() {
            continue;
        }

        // Find the thumb.
        let Ok(children_inner) = q_children.get(*inner_ent) else {
            continue;
        };
        let Some(thumb_ent) = children_inner.first() else {
            continue;
        };

        let Ok(mut thumb_node) = q_node.get_mut(*thumb_ent) else {
            continue;
        };

        // Ensure thumb is in the wheel even when aspect_ratio fails
        let size = inner_node.size() * inner_node.inverse_scale_factor();
        let min_side = size.min_element();
        let offset = Vec2::from_angle(wheel_value.hue.to_radians())
            * (wheel_value.saturation.clamp(0.0, 1.0) * 0.5 * min_side);
        let left = px(size.x * 0.5 + offset.x);
        let top = px(size.y * 0.5 + offset.y);
        if thumb_node.left != left || thumb_node.top != top {
            thumb_node.left = left;
            thumb_node.top = top;
        }
    }
}

fn emit_color_wheel_value_change(
    commands: &mut Commands,
    source: Entity,
    current: &ColorWheelValue,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    pointer_position: Vec2,
    ui_scale: f32,
    is_final: bool,
    dragging: bool,
) {
    // Ensure press is in the wheel even when aspect_ratio fails
    let Some(inverse_transform) = transform.try_inverse() else {
        return;
    };
    let local = inverse_transform
        .transform_point2(pointer_position * node_target.scale_factor() / ui_scale);
    let min_side = node.size().min_element();
    if min_side <= 0.0 {
        return;
    }
    let pos = local / min_side;
    let radial = pos.length();

    // Ignore presses outside wheel
    if radial > 0.5 && !dragging {
        return;
    }
    let saturation = (radial * 2.0).clamp(0.0, 1.0);

    // Keep the current hue when radial is 0
    let hue = if radial > 0.0 {
        pos.to_angle().rem_euclid(TAU).to_degrees()
    } else {
        current.hue
    };

    commands.trigger(ValueChange {
        source,
        value: ColorWheelValue {
            hue,
            saturation,
            z: current.z,
        },
        is_final,
    });
}

fn on_pointer_press(
    mut press: On<Pointer<Press>>,
    q_color_wheels: Query<(&ColorWheelValue, Has<InteractionDisabled>), With<FeathersColorWheel>>,
    q_color_wheel_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorWheelInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_wheel_inner.get(press.entity)
        && let Ok((value, disabled)) = q_color_wheels.get(parent.0)
    {
        press.propagate(false);
        if !disabled {
            emit_color_wheel_value_change(
                &mut commands,
                parent.0,
                value,
                node,
                node_target,
                transform,
                press.pointer_location.position,
                ui_scale.0,
                false,
                false,
            );
        }
    }
}

fn on_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_color_wheels: Query<
        (&mut ColorWheelDragState, Has<InteractionDisabled>),
        With<FeathersColorWheel>,
    >,
    q_color_wheel_inner: Query<&ChildOf, With<ColorWheelInner>>,
) {
    if let Ok(parent) = q_color_wheel_inner.get(drag_start.entity)
        && let Ok((mut state, disabled)) = q_color_wheels.get_mut(parent.0)
    {
        drag_start.propagate(false);
        if !disabled {
            state.0 = true;
        }
    }
}

fn on_drag(
    mut drag: On<Pointer<Drag>>,
    q_color_wheels: Query<
        (
            &ColorWheelValue,
            &ColorWheelDragState,
            Has<InteractionDisabled>,
        ),
        With<FeathersColorWheel>,
    >,
    q_color_wheel_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorWheelInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_wheel_inner.get(drag.entity)
        && let Ok((value, state, disabled)) = q_color_wheels.get(parent.0)
    {
        drag.propagate(false);
        if state.0 && !disabled {
            emit_color_wheel_value_change(
                &mut commands,
                parent.0,
                value,
                node,
                node_target,
                transform,
                drag.pointer_location.position,
                ui_scale.0,
                false,
                true,
            );
        }
    }
}

fn on_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_color_wheels: Query<
        (
            &ColorWheelValue,
            &mut ColorWheelDragState,
            Has<InteractionDisabled>,
        ),
        With<FeathersColorWheel>,
    >,
    q_color_wheel_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorWheelInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_wheel_inner.get(drag_end.entity)
        && let Ok((value, mut state, disabled)) = q_color_wheels.get_mut(parent.0)
    {
        drag_end.propagate(false);
        if state.0 && !disabled {
            emit_color_wheel_value_change(
                &mut commands,
                parent.0,
                value,
                node,
                node_target,
                transform,
                drag_end.pointer_location.position,
                ui_scale.0,
                true,
                true,
            );
        }
        state.0 = false;
    }
}

fn on_drag_cancel(
    drag_cancel: On<Pointer<Cancel>>,
    mut q_color_wheels: Query<&mut ColorWheelDragState, With<FeathersColorWheel>>,
    q_color_wheel_inner: Query<&ChildOf, With<ColorWheelInner>>,
) {
    if let Ok(parent) = q_color_wheel_inner.get(drag_cancel.entity)
        && let Ok(mut state) = q_color_wheels.get_mut(parent.0)
    {
        state.0 = false;
    }
}

/// Plugin which registers the observers for updating the wheel color.
pub struct ColorWheelPlugin;

impl Plugin for ColorWheelPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(UiMaterialPlugin::<ColorWheelMaterial>::default());
        // Ensure thumb stays inside wheel on next frame when layout changes
        app.add_systems(PostUpdate, update_wheel_color.before(UiSystems::Layout));
        app.add_observer(on_pointer_press)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drag_cancel);
    }
}
