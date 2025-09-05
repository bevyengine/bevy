use core::f32::consts::PI;

use bevy_app::{Plugin, PreUpdate};
use bevy_asset::Handle;
use bevy_color::{Alpha, Color, Hsla};
use bevy_core_widgets::{
    Callback, CoreSlider, CoreSliderThumb, SliderRange, SliderValue, TrackClick, ValueChange,
};
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    query::{Changed, Or, With},
    schedule::IntoScheduleConfigs,
    spawn::SpawnRelated,
    system::{In, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_log::warn_once;
use bevy_picking::PickingSystems;
use bevy_ui::{
    AlignItems, BackgroundColor, BackgroundGradient, BorderColor, BorderRadius, ColorStop, Display,
    FlexDirection, Gradient, InterpolationColorSpace, LinearGradient, Node, Outline, PositionType,
    UiRect, UiTransform, Val, Val2, ZIndex,
};
use bevy_ui_render::ui_material::MaterialNode;

use crate::{
    alpha_pattern::{AlphaPattern, AlphaPatternMaterial},
    cursor::EntityCursor,
    palette,
    rounded_corners::RoundedCorners,
};

const SLIDER_HEIGHT: f32 = 16.0;
const TRACK_PADDING: f32 = 3.0;
const TRACK_RADIUS: f32 = SLIDER_HEIGHT * 0.5 - TRACK_PADDING;
const THUMB_SIZE: f32 = SLIDER_HEIGHT - 2.0;

/// Indicates which color channel we want to edit.
#[derive(Component, Default, Clone)]
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
            | ColorChannel::HslLightness => SliderRange::new(0., 1.),
            ColorChannel::HslHue => SliderRange::new(0., 360.),
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
#[derive(Component, Default, Clone)]
pub struct SliderBaseColor(pub Color);

/// Color slider template properties, passed to [`color_slider`] function.
pub struct ColorSliderProps {
    /// Slider current value
    pub value: f32,
    /// On-change handler
    pub on_change: Callback<In<ValueChange<f32>>>,
    /// Which color component we're editing
    pub channel: ColorChannel,
}

impl Default for ColorSliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            on_change: Callback::Ignore,
            channel: ColorChannel::Alpha,
        }
    }
}

/// A color slider widget.
#[derive(Component, Default, Clone)]
#[require(CoreSlider, SliderBaseColor(Color::WHITE))]
pub struct ColorSlider {
    /// Which channel is being edited by this slider.
    pub channel: ColorChannel,
}

/// Marker for the track
#[derive(Component, Default, Clone)]
struct ColorSliderTrack;

/// Marker for the thumb
#[derive(Component, Default, Clone)]
struct ColorSliderThumb;

/// Spawn a new slider widget.
///
/// # Arguments
///
/// * `props` - construction properties for the slider.
/// * `overrides` - a bundle of components that are merged in with the normal slider components.
pub fn color_slider<B: Bundle>(props: ColorSliderProps, overrides: B) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            height: Val::Px(SLIDER_HEIGHT),
            align_items: AlignItems::Stretch,
            flex_grow: 1.0,
            ..Default::default()
        },
        CoreSlider {
            on_change: props.on_change,
            track_click: TrackClick::Snap,
        },
        ColorSlider {
            channel: props.channel.clone(),
        },
        SliderValue(props.value),
        props.channel.range(),
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer),
        TabIndex(0),
        overrides,
        children![
            // track
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.),
                    right: Val::Px(0.),
                    top: Val::Px(TRACK_PADDING),
                    bottom: Val::Px(TRACK_PADDING),
                    ..Default::default()
                },
                RoundedCorners::All.to_border_radius(TRACK_RADIUS),
                ColorSliderTrack,
                AlphaPattern,
                MaterialNode::<AlphaPatternMaterial>(Handle::default()),
                children![
                    // Left endcap
                    (
                        Node {
                            width: Val::Px(THUMB_SIZE * 0.5),
                            ..Default::default()
                        },
                        RoundedCorners::Left.to_border_radius(TRACK_RADIUS),
                        BackgroundColor(palette::X_AXIS),
                    ),
                    // Track with gradient
                    (
                        Node {
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        BackgroundGradient(vec![Gradient::Linear(LinearGradient {
                            angle: PI * 0.5,
                            stops: vec![
                                ColorStop::new(Color::NONE, Val::Percent(0.)),
                                ColorStop::new(Color::NONE, Val::Percent(50.)),
                                ColorStop::new(Color::NONE, Val::Percent(100.)),
                            ],
                            color_space: InterpolationColorSpace::Srgba,
                        })]),
                        ZIndex(1),
                        children![(
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(0.),
                                top: Val::Percent(50.),
                                width: Val::Px(THUMB_SIZE),
                                height: Val::Px(THUMB_SIZE),
                                border: UiRect::all(Val::Px(2.0)),
                                ..Default::default()
                            },
                            CoreSliderThumb,
                            ColorSliderThumb,
                            BorderRadius::MAX,
                            BorderColor::all(palette::WHITE),
                            Outline {
                                width: Val::Px(1.),
                                offset: Val::Px(0.),
                                color: palette::BLACK
                            },
                            UiTransform::from_translation(Val2::new(
                                Val::Percent(-50.0),
                                Val::Percent(-50.0),
                            ))
                        )]
                    ),
                    // Right endcap
                    (
                        Node {
                            width: Val::Px(THUMB_SIZE * 0.5),
                            ..Default::default()
                        },
                        RoundedCorners::Right.to_border_radius(TRACK_RADIUS),
                        BackgroundColor(palette::Z_AXIS),
                    ),
                ]
            ),
        ],
    )
}

fn update_slider_pos(
    mut q_sliders: Query<
        (Entity, &SliderValue, &SliderRange),
        (
            With<ColorSlider>,
            Or<(Changed<SliderValue>, Changed<SliderRange>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_slider_thumb: Query<&mut Node, With<ColorSliderThumb>>,
) {
    for (slider_ent, value, range) in q_sliders.iter_mut() {
        for child in q_children.iter_descendants(slider_ent) {
            if let Ok(mut thumb_node) = q_slider_thumb.get_mut(child) {
                thumb_node.left = Val::Percent(range.thumb_position(value.0) * 100.0);
            }
        }
    }
}

fn update_track_color(
    mut q_sliders: Query<(Entity, &ColorSlider, &SliderBaseColor), Changed<SliderBaseColor>>,
    q_children: Query<&Children>,
    q_track: Query<(), With<ColorSliderTrack>>,
    mut q_background: Query<&mut BackgroundColor>,
    mut q_gradient: Query<&mut BackgroundGradient>,
) {
    for (slider_ent, slider, SliderBaseColor(base_color)) in q_sliders.iter_mut() {
        let (start, middle, end) = slider.channel.gradient_ends(*base_color);
        if let Some(track_ent) = q_children
            .iter_descendants(slider_ent)
            .find(|ent| q_track.contains(*ent))
        {
            let Ok(track_children) = q_children.get(track_ent) else {
                continue;
            };

            if let Ok(mut cap_bg) = q_background.get_mut(track_children[0]) {
                cap_bg.0 = start;
            }

            if let Ok(mut gradient) = q_gradient.get_mut(track_children[1])
                && let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..]
            {
                linear_gradient.stops[0].color = start;
                linear_gradient.stops[1].color = middle;
                linear_gradient.stops[2].color = end;
                linear_gradient.color_space = match slider.channel {
                    ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => {
                        InterpolationColorSpace::Srgba
                    }
                    ColorChannel::HslHue
                    | ColorChannel::HslLightness
                    | ColorChannel::HslSaturation => InterpolationColorSpace::Hsla,
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
            }

            if let Ok(mut cap_bg) = q_background.get_mut(track_children[2]) {
                cap_bg.0 = end;
            }
        }
    }
}

/// Plugin which registers the systems for updating the slider styles.
pub struct ColorSliderPlugin;

impl Plugin for ColorSliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_slider_pos, update_track_color).in_set(PickingSystems::Last),
        );
    }
}
