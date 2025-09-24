use core::f32::consts::PI;

use bevy_app::{Plugin, PreUpdate};
use bevy_color::Color;
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, Spawned, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    spawn::SpawnRelated,
    system::{Commands, In, Query, Res},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::PickingSystems;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    widget::Text, AlignItems, BackgroundGradient, ColorStop, Display, FlexDirection, Gradient,
    InteractionDisabled, InterpolationColorSpace, JustifyContent, LinearGradient, Node,
    PositionType, UiRect, Val,
};
use bevy_ui_widgets::{Callback, Slider, SliderRange, SliderValue, TrackClick, ValueChange};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    handle_or_path::HandleOrPath,
    rounded_corners::RoundedCorners,
    theme::{ThemeFontColor, ThemedText, UiTheme},
    tokens,
};

/// Slider template properties, passed to [`slider`] function.
pub struct SliderProps {
    /// Slider current value
    pub value: f32,
    /// Slider minimum value
    pub min: f32,
    /// Slider maximum value
    pub max: f32,
    /// On-change handler
    pub on_change: Callback<In<ValueChange<f32>>>,
}

impl Default for SliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 1.0,
            on_change: Callback::Ignore,
        }
    }
}

#[derive(Component, Default, Clone)]
#[require(Slider)]
#[derive(Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderStyle;

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
pub fn slider<B: Bundle>(props: SliderProps, overrides: B) -> impl Bundle {
    (
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
            flex_grow: 1.0,
            ..Default::default()
        },
        Slider {
            on_change: props.on_change,
            track_click: TrackClick::Drag,
        },
        SliderStyle,
        SliderValue(props.value),
        SliderRange::new(props.min, props.max),
        EntityCursor::System(bevy_window::SystemCursorIcon::EwResize),
        TabIndex(0),
        RoundedCorners::All.to_border_radius(6.0),
        // Use a gradient to draw the moving bar
        BackgroundGradient(vec![Gradient::Linear(LinearGradient {
            angle: PI * 0.5,
            stops: vec![
                ColorStop::new(Color::NONE, Val::Percent(0.)),
                ColorStop::new(Color::NONE, Val::Percent(50.)),
                ColorStop::new(Color::NONE, Val::Percent(50.)),
                ColorStop::new(Color::NONE, Val::Percent(100.)),
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
            ThemeFontColor(tokens::SLIDER_TEXT),
            InheritableFont {
                font: HandleOrPath::Path(fonts::MONO.to_owned()),
                font_size: 12.0,
            },
            children![(Text::new("10.0"), ThemedText, SliderValueText,)],
        )],
    )
}

fn update_slider_styles(
    mut q_sliders: Query<
        (Entity, Has<InteractionDisabled>, &mut BackgroundGradient),
        (With<SliderStyle>, Or<(Spawned, Added<InteractionDisabled>)>),
    >,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (slider_ent, disabled, mut gradient) in q_sliders.iter_mut() {
        set_slider_styles(
            slider_ent,
            &theme,
            disabled,
            gradient.as_mut(),
            &mut commands,
        );
    }
}

fn update_slider_styles_remove(
    mut q_sliders: Query<(Entity, Has<InteractionDisabled>, &mut BackgroundGradient)>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|ent| {
        if let Ok((slider_ent, disabled, mut gradient)) = q_sliders.get_mut(ent) {
            set_slider_styles(
                slider_ent,
                &theme,
                disabled,
                gradient.as_mut(),
                &mut commands,
            );
        }
    });
}

fn set_slider_styles(
    slider_ent: Entity,
    theme: &Res<'_, UiTheme>,
    disabled: bool,
    gradient: &mut BackgroundGradient,
    commands: &mut Commands,
) {
    let bar_color = theme.color(&match disabled {
        true => tokens::SLIDER_BAR_DISABLED,
        false => tokens::SLIDER_BAR,
    });

    let bg_color = theme.color(&tokens::SLIDER_BG);

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

    // Change cursor shape
    commands
        .entity(slider_ent)
        .insert(EntityCursor::System(cursor_shape));
}

fn update_slider_pos(
    mut q_sliders: Query<
        (Entity, &SliderValue, &SliderRange, &mut BackgroundGradient),
        (
            With<SliderStyle>,
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
pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_slider_styles,
                update_slider_styles_remove,
                update_slider_pos,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
