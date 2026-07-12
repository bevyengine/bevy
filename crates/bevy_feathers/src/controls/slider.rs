use core::f32::consts::PI;

use bevy_app::{Plugin, PreUpdate};
use bevy_color::Color;
use bevy_ecs::{
    bundle::Bundle,
    change_detection::DetectChanges,
    children,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, Spawned, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{
    percent, px, widget::Text, AlignItems, BackgroundGradient, ColorStop, Display, FlexDirection,
    Gradient, InteractionDisabled, InterpolationColorSpace, JustifyContent, LinearGradient, Node,
    PositionType, Pressed, UiRect,
};
use bevy_ui_widgets::{
    Slider, SliderOrientation, SliderPrecision, SliderRange, SliderValue, TrackClick,
};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    display::caption,
    focus::FocusIndicator,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{InheritableThemeTextColor, ThemedText, UiTheme},
    tokens,
};

/// A slider widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`FeathersSliderProps`].
///
/// # Emitted events
///
/// * [`bevy_ui_widgets::ValueChange<f32>`] when the slider value is changed.
///
/// These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
///
/// A more complete explanation of how to control this widget can be found in the documentation
/// for [`Slider`] and [`bevy_ui_widgets`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersSliderProps)]
#[require(Slider)]
#[reflect(Component, Clone, Default)]
pub struct FeathersSlider;

/// Props used to construct the [`FeathersSlider`] scene.
pub struct FeathersSliderProps {
    /// Slider minimum value
    pub min: f32,
    /// Slider maximum value
    pub max: f32,
}

impl Default for FeathersSliderProps {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

impl FeathersSlider {
    fn scene(props: FeathersSliderProps) -> impl Scene {
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(8)),
                flex_grow: 1.0,
                border_radius: {RoundedCorners::All.to_border_radius(6.0)},
            }
            Hovered
            Slider {
                track_click: TrackClick::Drag,
                orientation: SliderOrientation::Horizontal,
            }
            FeathersSlider
            SliderValue({props.min})
            SliderRange::new(props.min, props.max)
            EntityCursor::System(bevy_window::SystemCursorIcon::EwResize)
            TabIndex(0)
            FocusIndicator
            InheritableThemeTextColor(tokens::SLIDER_TEXT)
            // Use a gradient to draw the moving bar
            BackgroundGradient(vec![Gradient::Linear(LinearGradient {
                angle: PI * 0.5,
                stops: vec![
                    ColorStop::new(Color::NONE, percent(0)),
                    ColorStop::new(Color::NONE, percent(50)),
                    ColorStop::new(Color::NONE, percent(50)),
                    ColorStop::new(Color::NONE, percent(100)),
                ],
                color_space: InterpolationColorSpace::Srgba,
            })])
            Children [(
                // Text container
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                InheritableFont {
                    font: fonts::MONO,
                    font_size: size::SMALL_FONT,
                    weight: FontWeight::NORMAL,
                }
                Children [(caption("10.0") SliderValueText)]
            )]
        }
    }
}

/// Marker for the text
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderValueText;

/// Spawn a new slider widget.
///
/// # Arguments
///
/// * `props` - construction properties for the slider.
/// * `overrides` - a bundle of components that are merged in with the normal slider components.
///
/// # Emitted events
///
/// * [`bevy_ui_widgets::ValueChange<f32>`] when the slider value is changed.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
#[deprecated(since = "0.19.0", note = "Use the slider() BSN function")]
pub fn slider_bundle<B: Bundle>(props: FeathersSliderProps, overrides: B) -> impl Bundle {
    (
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(8)),
            flex_grow: 1.0,
            border_radius: RoundedCorners::All.to_border_radius(6.0),
            ..Default::default()
        },
        Hovered::default(),
        Slider {
            track_click: TrackClick::Drag,
            orientation: SliderOrientation::Horizontal,
        },
        FeathersSlider,
        SliderValue(props.min),
        SliderRange::new(props.min, props.max),
        EntityCursor::System(bevy_window::SystemCursorIcon::EwResize),
        TabIndex(0),
        FocusIndicator,
        InheritableThemeTextColor(tokens::SLIDER_TEXT),
        // Use a gradient to draw the moving bar
        BackgroundGradient(vec![Gradient::Linear(LinearGradient {
            angle: PI * 0.5,
            stops: vec![
                ColorStop::new(Color::NONE, percent(0)),
                ColorStop::new(Color::NONE, percent(50)),
                ColorStop::new(Color::NONE, percent(50)),
                ColorStop::new(Color::NONE, percent(100)),
            ],
            color_space: InterpolationColorSpace::Srgba,
        })]),
        overrides,
        children![(
            // Text container
            Node {
                display: Display::Flex,
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            InheritableFont {
                font_size: size::SMALL_FONT,
                weight: FontWeight::NORMAL,
                ..Default::default()
            },
            children![(Text::new("10.0"), ThemedText, SliderValueText,)],
        )],
    )
}

fn update_slider_styles(
    mut q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &mut BackgroundGradient,
            &InheritableThemeTextColor,
        ),
        (
            With<FeathersSlider>,
            Or<(
                Spawned,
                Added<InteractionDisabled>,
                Changed<Hovered>,
                Added<Pressed>,
            )>,
        ),
    >,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (slider_ent, disabled, pressed, hovered, mut gradient, font_color) in q_sliders.iter_mut() {
        set_slider_styles(
            slider_ent,
            &theme,
            disabled,
            pressed,
            hovered.0,
            gradient.as_mut(),
            font_color,
            &mut commands,
        );
    }
}

fn update_slider_styles_remove(
    mut q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &mut BackgroundGradient,
            &InheritableThemeTextColor,
        ),
        With<FeathersSlider>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut remove_pressed: RemovedComponents<Pressed>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(remove_pressed.read())
        .for_each(|ent| {
            if let Ok((slider_ent, disabled, pressed, hovered, mut gradient, font_color)) =
                q_sliders.get_mut(ent)
            {
                set_slider_styles(
                    slider_ent,
                    &theme,
                    disabled,
                    pressed,
                    hovered.0,
                    gradient.as_mut(),
                    font_color,
                    &mut commands,
                );
            }
        });
}

/// Re-apply slider styles to every slider when the theme changes.
fn update_slider_styles_theme(
    mut q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &mut BackgroundGradient,
            &InheritableThemeTextColor,
        ),
        With<FeathersSlider>,
    >,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if !theme.is_changed() {
        return;
    }
    for (slider_ent, disabled, pressed, hovered, mut gradient, font_color) in q_sliders.iter_mut() {
        set_slider_styles(
            slider_ent,
            &theme,
            disabled,
            pressed,
            hovered.0,
            gradient.as_mut(),
            font_color,
            &mut commands,
        );
    }
}

fn set_slider_styles(
    slider_ent: Entity,
    theme: &Res<'_, UiTheme>,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    gradient: &mut BackgroundGradient,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let bar_color = theme.color(&if disabled {
        tokens::SLIDER_BAR_DISABLED
    } else if pressed {
        tokens::SLIDER_BAR_PRESSED
    } else if hovered {
        tokens::SLIDER_BAR_HOVER
    } else {
        tokens::SLIDER_BAR
    });

    let bg_color = theme.color(&if disabled {
        tokens::SLIDER_BG_DISABLED
    } else if pressed {
        tokens::SLIDER_BG_PRESSED
    } else if hovered {
        tokens::SLIDER_BG_HOVER
    } else {
        tokens::SLIDER_BG
    });

    let text_token = if disabled {
        tokens::SLIDER_TEXT_DISABLED
    } else {
        tokens::SLIDER_TEXT
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::EwResize,
    };

    if let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..] {
        linear_gradient.stops[0].color = bar_color;
        linear_gradient.stops[1].color = bar_color;
        linear_gradient.stops[2].color = bg_color;
        linear_gradient.stops[3].color = bg_color;
    }

    // Change value-text color (dim when disabled)
    if font_color.0 != text_token {
        commands
            .entity(slider_ent)
            .insert(InheritableThemeTextColor(text_token));
    }

    // Change cursor shape
    commands
        .entity(slider_ent)
        .insert(EntityCursor::System(cursor_shape));
}

fn update_slider_pos(
    mut q_sliders: Query<
        (
            Entity,
            &SliderValue,
            &SliderRange,
            Option<&SliderPrecision>,
            &mut BackgroundGradient,
        ),
        (
            With<FeathersSlider>,
            Or<(
                Changed<SliderValue>,
                Changed<SliderRange>,
                Changed<Children>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_slider_text: Query<&mut Text, With<SliderValueText>>,
) {
    for (slider_ent, value, range, precision, mut gradient) in q_sliders.iter_mut() {
        if let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..] {
            let percent_value = (range.thumb_position(value.0) * 100.0).clamp(0.0, 100.0);
            linear_gradient.stops[1].point = percent(percent_value);
            linear_gradient.stops[2].point = percent(percent_value);
        }

        // Find slider text child entity and update its text with the formatted value
        let precision = precision.cloned().unwrap_or_default().0;

        q_children.iter_descendants(slider_ent).for_each(|child| {
            if let Ok(mut text) = q_slider_text.get_mut(child) {
                let label = format!("{}", value.0);
                let decimals_len = label
                    .split_once('.')
                    .map(|(_, decimals)| decimals.len() as i32)
                    .unwrap_or(precision);

                // Don't format with precision if the value has more decimals than the precision
                text.0 = if precision >= 0 && decimals_len <= precision {
                    format!("{:.precision$}", value.0, precision = precision as usize)
                } else {
                    label
                };
            }
        });
    }
}

/// Plugin which registers the systems for updating the slider styles.
pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_slider_styles,
                update_slider_styles_remove,
                update_slider_styles_theme,
                update_slider_pos,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
