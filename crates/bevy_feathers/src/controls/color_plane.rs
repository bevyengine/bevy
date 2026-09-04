use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Asset, Assets};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::{Vec2, Vec3};
use bevy_picking::{
    cursor::EntityCursor,
    events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress},
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

use crate::{palette, theme::ThemeBackgroundColor, tokens};

const COLOR_PLANE_THUMB_SIZE: f32 = 10.0;

/// A "color plane" widget, which is a 2d picker that allows selecting two
/// components of a color space.
///
/// This is spawnable by inheriting it as a "scene component".
///
/// The control emits a [`ValueChange<Vec2>`] representing the current x and y values, ranging
/// from 0 to 1. The control accepts a [`Vec3`] input value, where the third component ('z')
/// is used to provide the fixed constant channel for the background gradient. Note that
/// the Y component is inverted, so that upward movement increases the value.
///
/// The control does not do any color space conversions internally, other than the shader code
/// for displaying gradients. Avoiding excess conversions helps avoid gimble-lock problems when
/// implementing a color picker for cylindrical color spaces such as HSL.
///
/// **Note:** For information on how widget state is managed
/// and how to respond to state changes, see the [`bevy_ui_widgets` documentation](bevy_ui_widgets).
#[derive(SceneComponent, Debug, Reflect, Copy, PartialEq, Eq, Hash, Default, Clone)]
#[reflect(Component)]
#[require(ColorPlaneDragState)]
pub enum FeathersColorPlane {
    /// Show red on the horizontal axis and green on the vertical.
    RedGreen,
    /// Show red on the horizontal axis and blue on the vertical.
    RedBlue,
    /// Show green on the horizontal axis and blue on the vertical.
    GreenBlue,
    /// Show hue on the horizontal axis and saturation on the vertical.
    HueSaturation,
    /// Show hue on the horizontal axis and lightness on the vertical.
    #[default]
    HueLightness,
    /// Show OKHSL hue on horizontal axis and saturation on vertical.
    OkhslHueSaturation,
    /// Show OKHSL hue on horizontal axis and lightness on vertical.
    OkhslHueLightness,
}

/// Component that contains the two components of the selected color, as well as the "z" value.
/// The x and y values determine the placement of the thumb element, while the z value controls
/// the background gradient.
#[derive(Component, Default, Clone, PartialEq, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorPlaneValue(pub Vec3);

/// Marker identifying the inner element of the color plane.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPlaneInner;

/// Marker identifying the thumb element of the color plane.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPlaneThumb;

/// Component used to manage the state of a slider during dragging.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct ColorPlaneDragState(bool);

#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct ColorPlaneMaterialKey {
    plane: FeathersColorPlane,
}

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
#[bind_group_data(ColorPlaneMaterialKey)]
struct ColorPlaneMaterial {
    plane: FeathersColorPlane,

    #[uniform(0)]
    fixed_channel: f32,

    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    #[uniform(0)]
    _webgl2_padding_12b: Vec3,
}

impl From<&ColorPlaneMaterial> for ColorPlaneMaterialKey {
    fn from(material: &ColorPlaneMaterial) -> Self {
        Self {
            plane: material.plane,
        }
    }
}

impl UiMaterial for ColorPlaneMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_feathers/assets/shaders/color_plane.wesl".into()
    }

    fn specialize(
        descriptor: &mut bevy_render::render_resource::RenderPipelineDescriptor,
        key: bevy_ui_render::prelude::UiMaterialKey<Self>,
    ) {
        let plane_def = match key.bind_group_data.plane {
            FeathersColorPlane::RedGreen => "PLANE_RG",
            FeathersColorPlane::RedBlue => "PLANE_RB",
            FeathersColorPlane::GreenBlue => "PLANE_GB",
            FeathersColorPlane::HueSaturation => "PLANE_HS",
            FeathersColorPlane::HueLightness => "PLANE_HL",
            FeathersColorPlane::OkhslHueSaturation => "PLANE_OKHS",
            FeathersColorPlane::OkhslHueLightness => "PLANE_OKHL",
        };
        descriptor.fragment.as_mut().unwrap().shader_defs =
            vec![ShaderDefVal::Bool(plane_def.into(), true)];
    }
}

impl FeathersColorPlane {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                min_height: px(100.0),
                align_self: AlignSelf::Stretch,
                padding: UiRect::all(px(4)),
                border_radius: BorderRadius::all(px(5)),
            }
            ColorPlaneValue
            ThemeBackgroundColor(tokens::COLOR_PLANE_BG)
            EntityCursor::System(bevy_window::SystemCursorIcon::Crosshair)
            Children [(
                Node {
                    align_self: AlignSelf::Stretch,
                    flex_grow: 1.0,
                }
                ColorPlaneInner
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
                    ColorPlaneThumb
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

fn update_plane_color(
    q_color_plane: Query<
        (Entity, &FeathersColorPlane, &ColorPlaneValue),
        Or<(Changed<FeathersColorPlane>, Changed<ColorPlaneValue>)>,
    >,
    q_children: Query<&Children>,
    q_material_node: Query<&MaterialNode<ColorPlaneMaterial>>,
    mut r_materials: ResMut<Assets<ColorPlaneMaterial>>,
    mut commands: Commands,
) {
    for (plane_ent, plane, plane_value) in q_color_plane.iter() {
        // Find the inner entity
        let Ok(children) = q_children.get(plane_ent) else {
            continue;
        };
        let Some(inner_ent) = children.first() else {
            continue;
        };

        if let Ok(material_node) = q_material_node.get(*inner_ent) {
            // Node component exists, update it
            if let Some(mut material) = r_materials.get_mut(material_node.id()) {
                // Update properties
                material.plane = *plane;
                material.fixed_channel = plane_value.0.z;
            }
        } else {
            // Insert new node component
            let material = r_materials.add(ColorPlaneMaterial {
                plane: *plane,
                fixed_channel: plane_value.0.z,
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _webgl2_padding_12b: Default::default(),
            });
            commands.entity(*inner_ent).insert(MaterialNode(material));
        }
    }
}

fn update_plane_thumb_position(
    q_color_plane: Query<(Entity, &ColorPlaneValue), With<FeathersColorPlane>>,
    q_children: Query<&Children>,
    q_computed_node: Query<&ComputedNode>,
    mut q_transform: Query<&mut UiTransform>,
) {
    for (plane_ent, plane_value) in &q_color_plane {
        let Ok(children) = q_children.get(plane_ent) else {
            continue;
        };
        let Some(inner_ent) = children.first() else {
            continue;
        };
        let Ok(children_inner) = q_children.get(*inner_ent) else {
            continue;
        };
        let Some(thumb_ent) = children_inner.first() else {
            continue;
        };
        let Ok(inner_node) = q_computed_node.get(*inner_ent) else {
            continue;
        };
        let Ok(mut thumb_transform) = q_transform.get_mut(*thumb_ent) else {
            continue;
        };
        let inner_size = inner_node.size() * inner_node.inverse_scale_factor;
        if inner_size.x > 0.0 && inner_size.y > 0.0 {
            let mut updated_transform = *thumb_transform;
            // `ColorPlaneValue` is in channel space, where y increases upward, while
            // the transform is in screen space (+y down), so the y component is inverted.
            updated_transform.translation = Val2::new(
                px(plane_value.0.x * inner_size.x - COLOR_PLANE_THUMB_SIZE * 0.5),
                px((1.0 - plane_value.0.y) * inner_size.y - COLOR_PLANE_THUMB_SIZE * 0.5),
            );
            thumb_transform.set_if_neq(updated_transform);
        }
    }
}

fn emit_color_plane_value_change(
    commands: &mut Commands,
    source: Entity,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    pointer_position: Vec2,
    ui_scale: f32,
    is_final: bool,
) {
    let Some(pos) = node.normalize_point(
        *transform,
        pointer_position * node_target.scale_factor() / ui_scale,
    ) else {
        return;
    };

    let value = (pos + Vec2::splat(0.5)).clamp(Vec2::ZERO, Vec2::ONE);

    commands.trigger(ValueChange {
        source,
        // `normalize_point` is in screen space (+y down), while the plane's value is in
        // channel space, where y increases upward.
        value: Vec2::new(value.x, 1.0 - value.y),
        is_final,
    });
}

fn on_pointer_press(
    mut press: On<PointerPress>,
    q_color_planes: Query<Has<InteractionDisabled>, With<FeathersColorPlane>>,
    q_color_plane_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorPlaneInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_plane_inner.get(press.entity)
        && let Ok(disabled) = q_color_planes.get(parent.0)
    {
        press.propagate(false);
        if !disabled {
            emit_color_plane_value_change(
                &mut commands,
                parent.0,
                node,
                node_target,
                transform,
                press.pointer.position,
                ui_scale.0,
                false,
            );
        }
    }
}

fn on_drag_start(
    mut drag_start: On<PointerDragStart>,
    mut q_color_planes: Query<
        (&mut ColorPlaneDragState, Has<InteractionDisabled>),
        With<FeathersColorPlane>,
    >,
    q_color_plane_inner: Query<&ChildOf, With<ColorPlaneInner>>,
) {
    if let Ok(parent) = q_color_plane_inner.get(drag_start.entity)
        && let Ok((mut state, disabled)) = q_color_planes.get_mut(parent.0)
    {
        drag_start.propagate(false);
        if !disabled {
            state.0 = true;
        }
    }
}

fn on_drag(
    mut drag: On<PointerDrag>,
    q_color_planes: Query<
        (&ColorPlaneDragState, Has<InteractionDisabled>),
        With<FeathersColorPlane>,
    >,
    q_color_plane_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorPlaneInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_plane_inner.get(drag.entity)
        && let Ok((state, disabled)) = q_color_planes.get(parent.0)
    {
        drag.propagate(false);
        if state.0 && !disabled {
            emit_color_plane_value_change(
                &mut commands,
                parent.0,
                node,
                node_target,
                transform,
                drag.pointer.position,
                ui_scale.0,
                false,
            );
        }
    }
}

fn on_drag_end(
    mut drag_end: On<PointerDragEnd>,
    mut q_color_planes: Query<
        (&mut ColorPlaneDragState, Has<InteractionDisabled>),
        With<FeathersColorPlane>,
    >,
    q_color_plane_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorPlaneInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_color_plane_inner.get(drag_end.entity)
        && let Ok((mut state, disabled)) = q_color_planes.get_mut(parent.0)
    {
        drag_end.propagate(false);
        if state.0 && !disabled {
            emit_color_plane_value_change(
                &mut commands,
                parent.0,
                node,
                node_target,
                transform,
                drag_end.pointer.position,
                ui_scale.0,
                true,
            );
        }
        state.0 = false;
    }
}

fn on_drag_cancel(
    drag_cancel: On<PointerCancel>,
    mut q_color_planes: Query<&mut ColorPlaneDragState, With<FeathersColorPlane>>,
    q_color_plane_inner: Query<&ChildOf, With<ColorPlaneInner>>,
) {
    if let Ok(parent) = q_color_plane_inner.get(drag_cancel.entity)
        && let Ok(mut state) = q_color_planes.get_mut(parent.0)
    {
        state.0 = false;
    }
}

/// Plugin which registers the observers for updating the swatch color.
pub struct ColorPlanePlugin;

impl Plugin for ColorPlanePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(UiMaterialPlugin::<ColorPlaneMaterial>::default());
        app.add_systems(
            PostUpdate,
            (update_plane_color, update_plane_thumb_position).before(UiSystems::Layout),
        );
        app.add_observer(on_pointer_press)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drag_cancel);
    }
}
