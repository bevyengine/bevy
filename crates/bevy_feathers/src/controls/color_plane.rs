use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Asset, Assets};
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, Has, Or, With},
    reflect::ReflectComponent,
    spawn::SpawnRelated,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::{Vec2, Vec3};
use bevy_picking::{
    events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press},
    Pickable,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy_render::render_resource::AsBindGroup;
use bevy_shader::{ShaderDefVal, ShaderRef};
use bevy_ui::{
    px, AlignSelf, BorderColor, BorderRadius, ComputedNode, ComputedUiRenderTargetInfo, Display,
    InteractionDisabled, Node, Outline, PositionType, UiGlobalTransform, UiRect, UiScale,
    UiTransform, Val, Val2,
};
use bevy_ui_render::{prelude::UiMaterial, ui_material::MaterialNode, UiMaterialPlugin};
use bevy_ui_widgets::ValueChange;

use crate::{cursor::EntityCursor, palette, theme::ThemeBackgroundColor, tokens};

/// Marker identifying a color plane widget.
///
/// The variant selects which view of the color pane is shown.
#[derive(Component, Default, Debug, Clone, Reflect, Copy, PartialEq, Eq, Hash)]
#[reflect(Component, Clone, Default)]
#[require(ColorPlaneDragState)]
pub enum ColorPlane {
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
}

/// Component that contains the two components of the selected color, as well as the "z" value.
/// The x and y values determine the placement of the thumb element, while the z value controls
/// the background gradient.
#[derive(Component, Default, Clone, Reflect)]
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
    plane: ColorPlane,
}

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
#[bind_group_data(ColorPlaneMaterialKey)]
struct ColorPlaneMaterial {
    plane: ColorPlane,

    #[uniform(0)]
    fixed_channel: f32,
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
        "embedded://bevy_feathers/assets/shaders/color_plane.wgsl".into()
    }

    fn specialize(
        descriptor: &mut bevy_render::render_resource::RenderPipelineDescriptor,
        key: bevy_ui_render::prelude::UiMaterialKey<Self>,
    ) {
        let plane_def = match key.bind_group_data.plane {
            ColorPlane::RedGreen => "PLANE_RG",
            ColorPlane::RedBlue => "PLANE_RB",
            ColorPlane::GreenBlue => "PLANE_GB",
            ColorPlane::HueSaturation => "PLANE_HS",
            ColorPlane::HueLightness => "PLANE_HL",
        };
        descriptor.fragment.as_mut().unwrap().shader_defs =
            vec![ShaderDefVal::Bool(plane_def.into(), true)];
    }
}

/// Template function to spawn a "color plane", which is a 2d picker that allows selecting two
/// components of a color space.
///
/// The control emits a [`ValueChange<Vec2>`] representing the current x and y values, ranging
/// from 0 to 1. The control accepts a [`Vec3`] input value, where the third component ('z')
/// is used to provide the fixed constant channel for the background gradient.
///
/// The control does not do any color space conversions internally, other than the shader code
/// for displaying gradients. Avoiding excess conversions helps avoid gimble-lock problems when
/// implementing a color picker for cylindrical color spaces such as HSL.
///
/// # Arguments
/// * `overrides` - a bundle of components that are merged in with the normal swatch components.
pub fn color_plane<B: Bundle>(plane: ColorPlane, overrides: B) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            min_height: px(100.0),
            align_self: AlignSelf::Stretch,
            padding: UiRect::all(px(4)),
            border_radius: BorderRadius::all(px(5)),
            ..Default::default()
        },
        plane,
        ColorPlaneValue::default(),
        ThemeBackgroundColor(tokens::COLOR_PLANE_BG),
        EntityCursor::System(bevy_window::SystemCursorIcon::Crosshair),
        overrides,
        children![(
            Node {
                align_self: AlignSelf::Stretch,
                flex_grow: 1.0,
                ..Default::default()
            },
            ColorPlaneInner,
            children![(
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(0.),
                    top: Val::Percent(0.),
                    width: px(10),
                    height: px(10),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::MAX,
                    ..Default::default()
                },
                ColorPlaneThumb,
                BorderColor::all(palette::WHITE),
                Outline {
                    width: Val::Px(1.),
                    offset: Val::Px(0.),
                    color: palette::BLACK
                },
                Pickable::IGNORE,
                UiTransform::from_translation(Val2::new(Val::Percent(-50.0), Val::Percent(-50.0),))
            )],
        ),],
    )
}

fn update_plane_color(
    q_color_plane: Query<
        (Entity, &ColorPlane, &ColorPlaneValue),
        Or<(Changed<ColorPlane>, Changed<ColorPlaneValue>)>,
    >,
    q_children: Query<&Children>,
    q_material_node: Query<&MaterialNode<ColorPlaneMaterial>>,
    mut q_node: Query<&mut Node>,
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
            if let Some(material) = r_materials.get_mut(material_node.id()) {
                // Update properties
                material.plane = *plane;
                material.fixed_channel = plane_value.0.z;
            }
        } else {
            // Insert new node component
            let material = r_materials.add(ColorPlaneMaterial {
                plane: *plane,
                fixed_channel: plane_value.0.z,
            });
            commands.entity(*inner_ent).insert(MaterialNode(material));
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

        thumb_node.left = Val::Percent(plane_value.0.x * 100.0);
        thumb_node.top = Val::Percent(plane_value.0.y * 100.0);
    }
}

fn on_pointer_press(
    mut press: On<Pointer<Press>>,
    q_color_planes: Query<Has<InteractionDisabled>, With<ColorPlane>>,
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
            let local_pos = transform.try_inverse().unwrap().transform_point2(
                press.pointer_location.position * node_target.scale_factor() / ui_scale.0,
            );
            let pos = local_pos / node.size() + Vec2::splat(0.5);
            let new_value = pos.clamp(Vec2::ZERO, Vec2::ONE);
            commands.trigger(ValueChange {
                source: parent.0,
                value: new_value,
            });
        }
    }
}

fn on_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_color_planes: Query<
        (&mut ColorPlaneDragState, Has<InteractionDisabled>),
        With<ColorPlane>,
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
    mut drag: On<Pointer<Drag>>,
    q_color_planes: Query<(&ColorPlaneDragState, Has<InteractionDisabled>), With<ColorPlane>>,
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
            let local_pos = transform.try_inverse().unwrap().transform_point2(
                drag.pointer_location.position * node_target.scale_factor() / ui_scale.0,
            );
            let pos = local_pos / node.size() + Vec2::splat(0.5);
            let new_value = pos.clamp(Vec2::ZERO, Vec2::ONE);
            commands.trigger(ValueChange {
                source: parent.0,
                value: new_value,
            });
        }
    }
}

fn on_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_color_planes: Query<&mut ColorPlaneDragState, With<ColorPlane>>,
    q_color_plane_inner: Query<&ChildOf, With<ColorPlaneInner>>,
) {
    if let Ok(parent) = q_color_plane_inner.get(drag_end.entity)
        && let Ok(mut state) = q_color_planes.get_mut(parent.0)
    {
        drag_end.propagate(false);
        state.0 = false;
    }
}

fn on_drag_cancel(
    drag_cancel: On<Pointer<Cancel>>,
    mut q_color_planes: Query<&mut ColorPlaneDragState, With<ColorPlane>>,
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
        app.add_systems(PostUpdate, update_plane_color);
        app.add_observer(on_pointer_press)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drag_cancel);
    }
}
