use bevy_app::{Plugin, PreUpdate};
use bevy_color::{Alpha, Color, Hsla, Okhsla};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Changed, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::Query,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_log::warn_once;
use bevy_picking::{cursor::EntityCursor, PickingSystems};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_scene::prelude::*;
use bevy_ui::{
    percent, px, BackgroundGradient, BorderColor, BorderRadius, ColorStop, ComputedNode, Display,
    Gradient, GridPlacement, InterpolationColorSpace, LinearGradient, Node, Outline, PositionType,
    UiRect, UiSystems, UiTransform, Val2,
};
use bevy_ui_render::ui_material::MaterialNode;
use bevy_ui_widgets::{
    Slider, SliderOrientation, SliderRange, SliderThumb, SliderValue, TrackClick,
};

use crate::{
    alpha_pattern::{AlphaPattern, AlphaPatternMaterial},
    controls::{FeathersSlider, FeathersTextInput, ToggleSwitchSlide},
    focus::FocusIndicator,
    palette,
    rounded_corners::RoundedCorners,
};

const SLIDER_HEIGHT: f32 = 16.0;
const TRACK_PADDING: f32 = 3.0;
const TRACK_RADIUS: f32 = SLIDER_HEIGHT * 0.5 - TRACK_PADDING;
const THUMB_SIZE: f32 = SLIDER_HEIGHT - 2.0;

/// Indicates which color channel we want to edit.
#[derive(Component, Default, Copy, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub enum ColorChannel {
    /// Editing the RGB red channel (0..=1)
    #[default]
    Red,
    /// Editing the RGB green channel (0..=1)
    Green,
    /// Editing the RGB blue channel (0..=1)
    Blue,
    /// Editing the hue channel (0..=360)
    HslHue,
    /// Editing the chroma / saturation channel (0..=1)
    HslSaturation,
    /// Editing the luminance channel (0..=1)
    HslLightness,
    /// Editing the alpha channel (0..=1)
    Alpha,
    /// Editing the Okhsl Hue channel (0..=360)
    OkhslHue,
    /// Editing the Okhsl Saturation channel (0..=1)
    OkhslSaturation,
    /// Editing the Okhsl Lightness channel (0..=1)
    OkhslLightness,
}

impl ColorChannel {
    /// Return the range of this color channel.
    pub fn range(&self) -> SliderRange {
        match self {
            ColorChannel::Red
            | ColorChannel::Green
            | ColorChannel::Blue
            | ColorChannel::Alpha
            | ColorChannel::HslSaturation
            | ColorChannel::HslLightness
            | ColorChannel::OkhslSaturation
            | ColorChannel::OkhslLightness => SliderRange::new(0., 1.),
            ColorChannel::HslHue | ColorChannel::OkhslHue => SliderRange::new(0., 360.),
        }
    }

    /// Return the color endpoints and midpoint of the gradient. This is determined by both the
    /// channel being edited and the base color.
    pub fn gradient_ends(&self, base_color: Color) -> (Color, Color, Color) {
        match self {
            ColorChannel::Red => {
                let base_rgb = base_color.to_srgba();
                (
                    Color::srgb(0.0, base_rgb.green, base_rgb.blue),
                    Color::srgb(0.5, base_rgb.green, base_rgb.blue),
                    Color::srgb(1.0, base_rgb.green, base_rgb.blue),
                )
            }

            ColorChannel::Green => {
                let base_rgb = base_color.to_srgba();
                (
                    Color::srgb(base_rgb.red, 0.0, base_rgb.blue),
                    Color::srgb(base_rgb.red, 0.5, base_rgb.blue),
                    Color::srgb(base_rgb.red, 1.0, base_rgb.blue),
                )
            }

            ColorChannel::Blue => {
                let base_rgb = base_color.to_srgba();
                (
                    Color::srgb(base_rgb.red, base_rgb.green, 0.0),
                    Color::srgb(base_rgb.red, base_rgb.green, 0.5),
                    Color::srgb(base_rgb.red, base_rgb.green, 1.0),
                )
            }

            ColorChannel::HslHue => (
                Color::hsl(0.0 + 0.0001, 1.0, 0.5),
                Color::hsl(180.0, 1.0, 0.5),
                Color::hsl(360.0 - 0.0001, 1.0, 0.5),
            ),

            ColorChannel::HslSaturation => {
                let base_hsla: Hsla = base_color.into();
                (
                    Color::hsl(base_hsla.hue, 0.0, base_hsla.lightness),
                    Color::hsl(base_hsla.hue, 0.5, base_hsla.lightness),
                    Color::hsl(base_hsla.hue, 1.0, base_hsla.lightness),
                )
            }

            ColorChannel::HslLightness => {
                let base_hsla: Hsla = base_color.into();
                (
                    Color::hsl(base_hsla.hue, base_hsla.saturation, 0.0),
                    Color::hsl(base_hsla.hue, base_hsla.saturation, 0.5),
                    Color::hsl(base_hsla.hue, base_hsla.saturation, 1.0),
                )
            }

            ColorChannel::OkhslHue => (
                Color::okhsl(0.0 + 0.0001, 1.0, 0.5),
                Color::okhsl(180.0, 1.0, 0.5),
                Color::okhsl(360.0 - 0.0001, 1.0, 0.5),
            ),

            ColorChannel::OkhslSaturation => {
                let base_okhsla: Okhsla = base_color.into();
                (
                    Color::okhsl(base_okhsla.hue, 0.0, base_okhsla.lightness),
                    Color::okhsl(base_okhsla.hue, 0.5, base_okhsla.lightness),
                    Color::okhsl(base_okhsla.hue, 1.0, base_okhsla.lightness),
                )
            }

            ColorChannel::OkhslLightness => {
                let base_okhsla: Okhsla = base_color.into();
                (
                    Color::okhsl(base_okhsla.hue, base_okhsla.saturation, 0.0),
                    Color::okhsl(base_okhsla.hue, base_okhsla.saturation, 0.5),
                    Color::okhsl(base_okhsla.hue, base_okhsla.saturation, 1.0),
                )
            }
            ColorChannel::Alpha => (
                base_color.with_alpha(0.),
                base_color.with_alpha(0.5),
                base_color.with_alpha(1.),
            ),
        }
    }
}

/// Used to store the color channels that we are not editing: the components of the color
/// that are constant for this slider.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct SliderBaseColor(pub Color);

/// A color slider widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`FeathersColorSliderProps`].
///
/// # Emitted events
///
/// * [`bevy_ui_widgets::ValueChange<f32>`] when the slider value is changed.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
#[derive(SceneComponent, Default, Clone)]
#[scene(FeathersColorSliderProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersColorSlider;

/// Props used to construct a [`FeathersColorSlider`] scene.
#[derive(Clone)]
pub struct FeathersColorSliderProps {
    /// Slider current value
    pub value: f32,
    /// Which color component we're editing
    pub channel: ColorChannel,
}

impl Default for FeathersColorSliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            channel: ColorChannel::Alpha,
        }
    }
}

/// A color slider widget.
#[derive(Component, Default, Clone)]
#[require(Slider, SliderBaseColor(Color::WHITE))]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct ColorSlider {
    /// Which channel is being edited by this slider.
    pub channel: ColorChannel,
}

/// Marker for the track
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
struct ColorSliderTrack;

/// Marker for the thumb
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub(crate) struct ColorSliderThumb;

impl FeathersColorSlider {
    fn scene(props: FeathersColorSliderProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Grid,
                height: px(SLIDER_HEIGHT),
                flex_grow: 1.0,
            }
            Slider {
                track_click: TrackClick::Snap,
                orientation: SliderOrientation::Horizontal,
            }
            ColorSlider {
                channel: {props.channel},
            }
            SliderValue({props.value})
            props.channel.range()
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            TabIndex(0)
            FocusIndicator
            Children [
                // track
                (
                    Node {
                        grid_row: GridPlacement::start(1),
                        grid_column: GridPlacement::start(1),
                        margin: {UiRect::vertical(px(TRACK_PADDING))},
                        border_radius: {RoundedCorners::All.to_border_radius(TRACK_RADIUS)},
                    }
                    ColorSliderTrack
                    AlphaPattern
                    MaterialNode::<AlphaPatternMaterial>
                ),
                // gradient
                (
                    Node {
                        grid_row: GridPlacement::start(1),
                        grid_column: GridPlacement::start(1),
                        margin: {UiRect::vertical(px(TRACK_PADDING))},
                        border_radius: {RoundedCorners::All.to_border_radius(TRACK_RADIUS)},
                    }
                    BackgroundGradient(vec![
                        Gradient::Linear(LinearGradient {
                            angle: LinearGradient::TO_RIGHT,
                            stops: vec![
                                ColorStop::px(Color::NONE, 0),
                                ColorStop::px(Color::NONE, THUMB_SIZE * 0.5),
                                ColorStop::percent(Color::NONE, 50),
                                ColorStop::percent(Color::NONE, 50),
                                ColorStop::percent(Color::NONE, 100),
                            ],
                            color_space: InterpolationColorSpace::Srgba,
                        }),
                        Gradient::Linear(LinearGradient {
                            angle: LinearGradient::TO_LEFT,
                            stops: vec![
                                ColorStop::px(Color::NONE, 0),
                                ColorStop::px(Color::NONE, THUMB_SIZE * 0.5),
                                ColorStop::percent(Color::NONE, 50),
                                ColorStop::percent(Color::NONE, 50),
                                ColorStop::percent(Color::NONE, 100),
                            ],
                            color_space: InterpolationColorSpace::Srgba,
                        }),
                    ])
                ),
                // thumb
                (
                    Node {
                        grid_row: GridPlacement::start(1),
                        grid_column: GridPlacement::start(1),
                        margin: {UiRect::horizontal(px(THUMB_SIZE * 0.5))},
                    }
                    Children [(
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(0),
                            top: percent(50),
                            width: px(THUMB_SIZE),
                            height: px(THUMB_SIZE),
                            border: px(2),
                            border_radius: BorderRadius::MAX,
                        }
                        SliderThumb
                        ColorSliderThumb
                        BorderColor::all(palette::WHITE)
                        Outline {
                            width: px(1),
                            offset: px(0),
                            color: palette::BLACK
                        }
                        UiTransform::from_translation(Val2::percent(-50., -50.))
                    )]
                )
            ]
        }
    }
}

fn update_slider_pos(
    mut q_sliders: Query<(Entity, &SliderValue, &SliderRange), With<ColorSlider>>,
    q_children: Query<&Children>,
    mut q_slider_thumb: Query<
        (&ChildOf, &mut UiTransform),
        (With<ColorSliderThumb>, Without<ToggleSwitchSlide>),
    >,
    q_computed_node: Query<&ComputedNode>,
) {
    for (slider_ent, value, range) in q_sliders.iter_mut() {
        for child in q_children.iter_descendants(slider_ent) {
            if let Ok((parent, mut thumb_transform)) = q_slider_thumb.get_mut(child)
                && let Ok(track_node) = q_computed_node.get(parent.parent())
                && track_node.size().x > 0.0
            {
                let track_width = track_node.size().x * track_node.inverse_scale_factor;
                let thumb_offset = range.thumb_position(value.0) * track_width - THUMB_SIZE * 0.5;
                let mut updated_transform = *thumb_transform;
                updated_transform.translation.x = px(thumb_offset);
                thumb_transform.set_if_neq(updated_transform);
            }
        }
    }
}

fn update_track_color(
    mut q_sliders: Query<(Entity, &ColorSlider, &SliderBaseColor), Changed<SliderBaseColor>>,
    q_children: Query<&Children>,
    // Without<FeathersTextInput> and Without<FeathersSlider> to avoid ambiguity with FeathersSlider systems
    mut q_gradient: Query<
        &mut BackgroundGradient,
        (Without<FeathersTextInput>, Without<FeathersSlider>),
    >,
) {
    for (slider_ent, slider, SliderBaseColor(base_color)) in q_sliders.iter_mut() {
        let (start, middle, end) = slider.channel.gradient_ends(*base_color);
        if let Some(gradient_ent) = q_children
            .get(slider_ent)
            .ok()
            .and_then(|children| children.get(1))
            && let Ok(mut gradient) = q_gradient.get_mut(*gradient_ent)
            && let [Gradient::Linear(left), Gradient::Linear(right)] = &mut gradient.0[..]
        {
            left.stops[0].color = start;
            left.stops[1].color = start;
            left.stops[2].color = middle;
            right.stops[0].color = end;
            right.stops[1].color = end;
            right.stops[2].color = middle;
            let color_space = match slider.channel {
                ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => {
                    InterpolationColorSpace::Srgba
                }
                ColorChannel::HslHue | ColorChannel::HslLightness | ColorChannel::HslSaturation => {
                    InterpolationColorSpace::Hsla
                }
                ColorChannel::OkhslHue
                | ColorChannel::OkhslLightness
                | ColorChannel::OkhslSaturation => InterpolationColorSpace::Okhsla,
                ColorChannel::Alpha => match base_color {
                    Color::Srgba(_) => InterpolationColorSpace::Srgba,
                    Color::LinearRgba(_) => InterpolationColorSpace::LinearRgba,
                    Color::Oklaba(_) => InterpolationColorSpace::Oklaba,
                    Color::Oklcha(_) => InterpolationColorSpace::OklchaLong,
                    Color::Hsla(_) | Color::Hsva(_) => InterpolationColorSpace::Hsla,
                    _ => {
                        warn_once!("Unsupported color space for ColorSlider: {:?}", base_color);
                        InterpolationColorSpace::Srgba
                    }
                },
            };

            left.color_space = color_space;
            right.color_space = color_space;
        }
    }
}

/// Plugin which registers the systems for updating the slider styles.
pub struct ColorSliderPlugin;

impl Plugin for ColorSliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            // .after(UiSystems::Focus) can be removed after Interaction is removed.
            (
                update_slider_pos.after(UiSystems::Focus),
                update_track_color,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
