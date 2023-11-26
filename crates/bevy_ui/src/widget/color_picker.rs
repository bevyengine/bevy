use std::f32::consts::PI;

use crate::{Interaction, RelativeCursorPosition};
use crate::{UiMaterial, UiMaterialPlugin};
use bevy_app::{Plugin, Update};
use bevy_asset::{load_internal_asset, Asset, Assets, Handle};
use bevy_derive::Deref;
use bevy_ecs::entity::Entity;
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::Component;
use bevy_ecs::query::{Added, Changed, With};
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::system::{Commands, Query};
use bevy_ecs::system::{Res, ResMut};
use bevy_hierarchy::Parent;
use bevy_log::warn;
use bevy_math::Vec3;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_reflect::TypePath;
use bevy_render::color::Color;
use bevy_render::render_resource::{AsBindGroup, Shader, ShaderRef};

pub const COLOR_PICKER_HUE_WHEEL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(864139486189741938413);
pub const COLOR_PICKER_SATURATION_VALUE_BOX_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(821381985132985160161);

#[derive(Debug, Default)]
pub struct ColorPickerPlugin;

impl Plugin for ColorPickerPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            COLOR_PICKER_HUE_WHEEL_SHADER_HANDLE,
            "color_picker/hue_wheel.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            COLOR_PICKER_SATURATION_VALUE_BOX_SHADER_HANDLE,
            "color_picker/saturation_value_box.wgsl",
            Shader::from_wgsl
        );

        app.add_event::<HueWheelEvent>()
            .add_event::<SaturationValueBoxEvent>()
            .add_systems(
                Update,
                (
                    add_sibling_component,
                    hue_wheel_events,
                    update_saturation_value_box_hue,
                    saturation_value_box_events,
                )
                    .chain(),
            )
            .add_plugins((
                UiMaterialPlugin::<HueWheelMaterial>::default(),
                UiMaterialPlugin::<SaturationValueBoxMaterial>::default(),
            ));
    }
}

/// When the hue wheel is pressed, this event is generated
#[derive(Debug, Event)]
pub struct HueWheelEvent {
    /// The [`HueWheel`] entity which produced the event
    pub entity: Entity,

    /// The color pressed on the wheel
    pub color: Color,

    /// The (0., 1.) range hue value pressed
    pub hue: f32,
}

/// When the saturation-value box is pressed, this event is generated
#[derive(Debug, Event)]
pub struct SaturationValueBoxEvent {
    /// The [`SaturationValueBox`] entity which produced the event
    pub entity: Entity,

    /// The color pressed within the box
    pub color: Color,
}

/// The entity which has this component is a sibling to the wrapped hue wheel entity.
/// A [`SaturationValueBox`] can have this component in order to store which hue wheel it is related to.
#[derive(Debug, Component, Deref)]
struct HueWheelSibling(Entity);

fn hue_wheel_events(
    interaction_query: Query<
        (
            Entity,
            &Handle<HueWheelMaterial>,
            &Interaction,
            &RelativeCursorPosition,
        ),
        (Changed<Interaction>, With<HueWheel>),
    >,
    wheels: Res<Assets<HueWheelMaterial>>,
    mut event_writer: EventWriter<HueWheelEvent>,
) {
    for (entity, material_handle, interaction, relative_position) in &interaction_query {
        if *interaction == Interaction::Pressed {
            if let Some(uv) = relative_position.normalized {
                // NOTE: The UV and hue calculations must sync with similar calculations in `hue_wheel.wgsl`
                let uv = (uv * 2.) - 1.;

                let Some(HueWheelMaterial { inner_radius }) = wheels.get(material_handle) else {
                    warn!("unexpected: a saturation-value box was pressed but found no asset containing its material");
                    continue;
                };
                let length = uv.length();
                if length < *inner_radius || length > 1.0 {
                    // the wheel is cut-out if below the radius in the shader so don't
                    // generate events either.
                    // similarly, don't send events from interactions outside the outer radius
                    continue;
                }

                let hue = (uv.y.atan2(uv.x) + PI) / (2. * PI);

                event_writer.send(HueWheelEvent {
                    entity,
                    color: hsv_to_rgb(hue, 1., 1.),
                    hue,
                });
            }
        }
    }
}

/// It's expected that a [`crate::node_bundles::SaturationValueBoxBundle`] and [`crate::node_bundles::HueWheelBundle`] is added to a common parent.
/// To help other systems, add the hue wheel as a sibling to the saturation value box.
fn add_sibling_component(
    sat_val_boxes: Query<(Entity, &Parent), Added<SaturationValueBox>>,
    hue_wheels: Query<(Entity, &Parent), Added<HueWheel>>,
    mut commands: Commands,
) {
    for (sv_e, sv_parent) in &sat_val_boxes {
        if let Some(hue_e) =
            hue_wheels
                .iter()
                .find_map(|(e, parent)| if sv_parent == parent { Some(e) } else { None })
        {
            commands.entity(sv_e).insert(HueWheelSibling(hue_e));
        } else {
            warn!("Found no box wheel sibling");
        }
    }
}

/// When hue wheels produce events the selected hue has changed.
/// The box UI is updated to match here.
fn update_saturation_value_box_hue(
    box_query: Query<
        (
            Entity,
            &Handle<SaturationValueBoxMaterial>,
            &HueWheelSibling,
        ),
        With<SaturationValueBox>,
    >,
    mut hue: EventReader<HueWheelEvent>,
    mut boxes: ResMut<Assets<SaturationValueBoxMaterial>>,
) {
    for event in hue.read() {
        let HueWheelEvent {
            entity,
            color: _,
            hue,
        } = event;

        if let Some((_, handle, _)) = box_query.iter().find(|(.., sibling)| *entity == ***sibling) {
            let Some(material) = boxes.get_mut(handle) else {
                continue;
            };

            material.hue = *hue;
        } else {
            continue;
        }
    }
}

fn saturation_value_box_events(
    interaction_query: Query<
        (
            Entity,
            &Interaction,
            &RelativeCursorPosition,
            &Handle<SaturationValueBoxMaterial>,
        ),
        (Changed<Interaction>, With<SaturationValueBox>),
    >,
    boxes: Res<Assets<SaturationValueBoxMaterial>>,
    mut event_writer: EventWriter<SaturationValueBoxEvent>,
) {
    for (entity, interaction, relative_position, material_handle) in &interaction_query {
        if *interaction == Interaction::Pressed {
            if let Some(uv) = relative_position.normalized {
                let Some(SaturationValueBoxMaterial { hue }) = boxes.get(material_handle) else {
                    warn!("unexpected: a saturation-value box was pressed but found no asset containing its material");
                    continue;
                };

                // NOTE: We want "value" to increase vertically which looks most natural hence the flip
                let color = hsv_to_rgb(*hue, uv.x, 1.0 - uv.y);
                event_writer.send(SaturationValueBoxEvent { entity, color });
            }
        }
    }
}

/// As ported from utils.wgsl:
///
/// ```wgsl
/// fn hsv2rgb(hue: f32, saturation: f32, value: f32) -> vec3<f32> {
///     let rgb = clamp(
///         abs(
///             ((hue * 6.0 + vec3<f32>(0.0, 4.0, 2.0)) % 6.0) - 3.0
///         ) - 1.0,
///         vec3<f32>(0.0),
///         vec3<f32>(1.0)
///     );
///
///     return value * mix(vec3<f32>(1.0), rgb, vec3<f32>(saturation));
/// }
/// ```
///
/// All inputs in range (0., 1.)
///
fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> Color {
    let rgb = ((((Vec3::splat(hue * 6.0) + Vec3::new(0.0, 4.0, 2.0)) % 6.0) - 3.0).abs() - 1.0)
        .clamp(Vec3::ZERO, Vec3::ONE);

    let result = value * Vec3::ONE.lerp(rgb, saturation);

    Color::rgb_linear(result.x, result.y, result.z)
}

/// Marker struct for hue wheels
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct HueWheel;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct HueWheelMaterial {
    /// The ratio of the inner radius (when the hue wheel cuts off) compared to the
    /// outer radius.
    #[uniform(0)]
    pub inner_radius: f32,
}

impl Default for HueWheelMaterial {
    fn default() -> Self {
        Self { inner_radius: 0.85 }
    }
}

/// Marker struct for saturation-value boxes
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct SaturationValueBox;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, Default)]
pub struct SaturationValueBoxMaterial {
    #[uniform(0)]
    hue: f32,
}

impl UiMaterial for HueWheelMaterial {
    fn fragment_shader() -> ShaderRef {
        COLOR_PICKER_HUE_WHEEL_SHADER_HANDLE.into()
    }
}

impl UiMaterial for SaturationValueBoxMaterial {
    fn fragment_shader() -> ShaderRef {
        COLOR_PICKER_SATURATION_VALUE_BOX_SHADER_HANDLE.into()
    }
}
