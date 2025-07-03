use core::f32::consts::PI;

use bevy_app::{Plugin, PreUpdate};
use bevy_color::Color;
use bevy_core_widgets::{Callback, CoreSlider, SliderRange, SliderValue, TrackClick};
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, Spawned, With},
    schedule::IntoScheduleConfigs,
    spawn::SpawnRelated,
    system::{In, Query, Res},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::PickingSystems;
use bevy_ui::{
    widget::Text, AlignItems, BackgroundGradient, ColorStop, Display, FlexDirection, Gradient,
    InteractionDisabled, InterpolationColorSpace, LinearGradient, Node, UiRect, Val,
};
use bevy_winit::cursor::CursorIcon;

use crate::{constants::size, rounded_corners::RoundedCorners, theme::UiTheme, tokens};

pub enum ColorChannel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl ColorChannel {
    /// Return the range of this color channel.
    pub fn range(&self) -> SliderRange {
        match self {
            ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue | ColorChannel::Alpha => {
                SliderRange::new(0., 1.)
            }
        }
    }
}

/// Slider template properties, passed to [`slider`] function.
pub struct GradientSliderProps {
    /// Slider current value
    pub value: f32,
    /// On-change handler
    pub on_change: Callback<In<f32>>,
    /// Which color component we're editing
    pub channel: ColorChannel,
}

impl Default for GradientSliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            on_change: Callback::Ignore,
            channel: ColorChannel::Alpha,
        }
    }
}

#[derive(Component, Default, Clone)]
#[require(CoreSlider)]
struct GradientSlider;

/// Marker for the thumb
#[derive(Component, Default, Clone)]
struct GradientSliderThumb;

#[derive(Component, Default, Clone)]
#[require(CoreSlider)]
struct GradientSliderEndcap0;

#[derive(Component, Default, Clone)]
#[require(CoreSlider)]
struct GradientSliderTrack;

#[derive(Component, Default, Clone)]
#[require(CoreSlider)]
struct GradientSliderEndcap1;

/// Spawn a new slider widget.
///
/// # Arguments
///
/// * `props` - construction properties for the slider.
/// * `overrides` - a bundle of components that are merged in with the normal slider components.
pub fn gradient_slider<B: Bundle>(props: GradientSliderProps, overrides: B) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            height: size::ROW_HEIGHT,
            align_items: AlignItems::Stretch,
            flex_grow: 1.0,
            padding: UiRect::axes(Val::Px(0.), Val::Px(2.)),
            ..Default::default()
        },
        CoreSlider {
            on_change: props.on_change,
            track_click: TrackClick::Drag,
        },
        GradientSlider,
        SliderValue(props.value),
        props.channel.range(),
        CursorIcon::System(bevy_window::SystemCursorIcon::EwResize),
        TabIndex(0),
        overrides,
        children![
            // Left endcap
            (
                Node {
                    width: Val::Px(6.0),
                    ..Default::default()
                },
                RoundedCorners::Left.to_border_radius(6.0),
                GradientSliderEndcap0
            ),
            // Track with gradient
            (
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                GradientSliderTrack,
                BackgroundGradient(vec![Gradient::Linear(LinearGradient {
                    angle: PI * 0.5,
                    stops: vec![
                        ColorStop::new(Color::NONE, Val::Percent(0.)),
                        ColorStop::new(Color::NONE, Val::Percent(50.)),
                        ColorStop::new(Color::NONE, Val::Percent(50.)),
                        ColorStop::new(Color::NONE, Val::Percent(100.)),
                    ],
                    color_space: InterpolationColorSpace::Srgb,
                })]),
            ),
            // Right endcap
            (
                Node {
                    width: Val::Px(6.0),
                    ..Default::default()
                },
                RoundedCorners::Right.to_border_radius(6.0),
                GradientSliderEndcap1
            ),
        ],
    )
}

fn update_slider_colors(
    mut q_sliders: Query<
        (Has<InteractionDisabled>, &mut BackgroundGradient),
        (
            With<GradientSlider>,
            Or<(Spawned, Added<InteractionDisabled>)>,
        ),
    >,
    theme: Res<UiTheme>,
) {
    for (disabled, mut gradient) in q_sliders.iter_mut() {
        set_slider_colors(&theme, disabled, gradient.as_mut());
    }
}

fn update_slider_colors_remove(
    mut q_sliders: Query<(Has<InteractionDisabled>, &mut BackgroundGradient)>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    theme: Res<UiTheme>,
) {
    removed_disabled.read().for_each(|ent| {
        if let Ok((disabled, mut gradient)) = q_sliders.get_mut(ent) {
            set_slider_colors(&theme, disabled, gradient.as_mut());
        }
    });
}

fn set_slider_colors(theme: &Res<'_, UiTheme>, disabled: bool, gradient: &mut BackgroundGradient) {
    let bar_color = theme.color(match disabled {
        true => tokens::SLIDER_BAR_DISABLED,
        false => tokens::SLIDER_BAR,
    });
    let bg_color = theme.color(tokens::SLIDER_BG);
    if let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..] {
        linear_gradient.stops[0].color = bar_color;
        linear_gradient.stops[1].color = bar_color;
        linear_gradient.stops[2].color = bg_color;
        linear_gradient.stops[3].color = bg_color;
    }
}

fn update_slider_pos(
    mut q_sliders: Query<
        (Entity, &SliderValue, &SliderRange, &mut BackgroundGradient),
        (
            With<GradientSlider>,
            Or<(
                Changed<SliderValue>,
                Changed<SliderRange>,
                Changed<Children>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_slider_text: Query<&mut Text, With<GradientSliderThumb>>,
) {
    for (slider_ent, value, range, mut gradient) in q_sliders.iter_mut() {
        if let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..] {
            let percent_value = range.thumb_position(value.0) * 100.0;
            linear_gradient.stops[1].point = Val::Percent(percent_value);
            linear_gradient.stops[2].point = Val::Percent(percent_value);
        }

        // Find slider text child entity and update its text with the formatted value
        q_children.iter_descendants(slider_ent).for_each(|child| {
            if let Ok(mut text) = q_slider_text.get_mut(child) {
                text.0 = format!("{}", value.0);
            }
        });
    }
}

/// Plugin which registers the systems for updating the slider styles.
pub struct GradientSliderPlugin;

impl Plugin for GradientSliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_slider_colors,
                update_slider_colors_remove,
                update_slider_pos,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
