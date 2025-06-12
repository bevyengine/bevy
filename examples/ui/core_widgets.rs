//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{
        CoreButton, CoreSlider, CoreSliderThumb, CoreWidgetsPlugin, SliderRange, SliderValue,
        TrackClick,
    },
    ecs::system::SystemId,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex},
        InputDispatchPlugin,
    },
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CoreWidgetsPlugin, InputDispatchPlugin))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(DemoWidgetStates { slider_value: 50.0 })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_widget_values,
                update_button_style,
                update_button_style2,
                update_slider_style.after(update_widget_values),
                update_slider_style2.after(update_widget_values),
                toggle_disabled,
            ),
        )
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);

/// Marker which identifies buttons with a particular style, in this case the "Demo style".
#[derive(Component)]
struct DemoButton;

/// Marker which identifies sliders with a particular style.
#[derive(Component, Default)]
struct DemoSlider;

/// Marker which identifies the slider's thumb element.
#[derive(Component, Default)]
struct DemoSliderThumb;

/// A struct to hold the state of various widgets shown in the demo.
///
/// While it is possible to use the widget's own state components as the source of truth,
/// in many cases widgets will be used to display dynamic data coming from deeper within the app,
/// using some kind of data-binding. This example shows how to maintain an external source of
/// truth for widget states.
#[derive(Resource)]
struct DemoWidgetStates {
    slider_value: f32,
}

fn update_button_style(
    mut buttons: Query<
        (
            Has<Pressed>,
            &Hovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (
            Or<(
                Changed<Pressed>,
                Changed<Hovered>,
                Added<InteractionDisabled>,
            )>,
            With<DemoButton>,
        ),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (pressed, hovered, disabled, mut color, mut border_color, children) in &mut buttons {
        let mut text = text_query.get_mut(children[0]).unwrap();
        set_button_style(
            disabled,
            hovered.get(),
            pressed,
            &mut color,
            &mut border_color,
            &mut text,
        );
    }
}

/// Supplementary system to detect removed marker components
fn update_button_style2(
    mut buttons: Query<
        (
            Has<Pressed>,
            &Hovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut removed_depressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut text_query: Query<&mut Text>,
) {
    removed_depressed.read().for_each(|entity| {
        if let Ok((pressed, hovered, disabled, mut color, mut border_color, children)) =
            buttons.get_mut(entity)
        {
            let mut text = text_query.get_mut(children[0]).unwrap();
            set_button_style(
                disabled,
                hovered.get(),
                pressed,
                &mut color,
                &mut border_color,
                &mut text,
            );
        }
    });
    removed_disabled.read().for_each(|entity| {
        if let Ok((pressed, hovered, disabled, mut color, mut border_color, children)) =
            buttons.get_mut(entity)
        {
            let mut text = text_query.get_mut(children[0]).unwrap();
            set_button_style(
                disabled,
                hovered.get(),
                pressed,
                &mut color,
                &mut border_color,
                &mut text,
            );
        }
    });
}

fn set_button_style(
    disabled: bool,
    hovered: bool,
    pressed: bool,
    color: &mut BackgroundColor,
    border_color: &mut BorderColor,
    text: &mut Text,
) {
    match (disabled, hovered, pressed) {
        // Disabled button
        (true, _, _) => {
            **text = "Disabled".to_string();
            *color = NORMAL_BUTTON.into();
            border_color.set_all(GRAY);
        }

        // Pressed and hovered button
        (false, true, true) => {
            **text = "Press".to_string();
            *color = PRESSED_BUTTON.into();
            border_color.set_all(RED);
        }

        // Hovered, unpressed button
        (false, true, false) => {
            **text = "Hover".to_string();
            *color = HOVERED_BUTTON.into();
            border_color.set_all(WHITE);
        }

        // Unhovered button (either pressed or not).
        (false, false, _) => {
            **text = "Button".to_string();
            *color = NORMAL_BUTTON.into();
            border_color.set_all(BLACK);
        }
    }
}

/// Update the widget states based on the changing resource.
fn update_widget_values(
    res: Res<DemoWidgetStates>,
    mut sliders: Query<Entity, With<DemoSlider>>,
    mut commands: Commands,
) {
    if res.is_changed() {
        for slider_ent in sliders.iter_mut() {
            commands
                .entity(slider_ent)
                .insert(SliderValue(res.slider_value));
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // System to print a value when the button is clicked.
    let on_click = commands.register_system(|| {
        info!("Button clicked!");
    });

    // System to update a resource when the slider value changes. Note that we could have
    // updated the slider value directly, but we want to demonstrate externalizing the state.
    let on_change_value = commands.register_system(
        |value: In<f32>, mut widget_states: ResMut<DemoWidgetStates>| {
            widget_states.slider_value = *value;
        },
    );

    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(&assets, on_click, on_change_value));
}

fn demo_root(
    asset_server: &AssetServer,
    on_click: SystemId,
    on_change_value: SystemId<In<f32>>,
) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        TabGroup::default(),
        children![
            button(asset_server, on_click),
            slider(0.0, 100.0, 50.0, Some(on_change_value)),
            Text::new("Press 'D' to toggle widget disabled states"),
        ],
    )
}

fn button(asset_server: &AssetServer, on_click: SystemId) -> impl Bundle {
    (
        Node {
            width: Val::Px(150.0),
            height: Val::Px(65.0),
            border: UiRect::all(Val::Px(5.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        DemoButton,
        CoreButton {
            on_click: Some(on_click),
        },
        Hovered::default(),
        TabIndex(0),
        BorderColor::all(Color::BLACK),
        BorderRadius::MAX,
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new("Button"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 33.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}

/// Create a demo slider
fn slider(min: f32, max: f32, value: f32, on_change: Option<SystemId<In<f32>>>) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            justify_items: JustifyItems::Center,
            column_gap: Val::Px(4.0),
            height: Val::Px(12.0),
            width: Val::Percent(30.0),
            ..default()
        },
        Name::new("Slider"),
        Hovered::default(),
        DemoSlider,
        CoreSlider {
            on_change,
            thumb_size: 12.0,
            track_click: TrackClick::Snap,
        },
        SliderValue(value),
        SliderRange(min..=max),
        TabIndex(0),
        Children::spawn((
            // Slider background rail
            Spawn((
                Node {
                    height: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK), // Border color for the checkbox
                BorderRadius::all(Val::Px(3.0)),
            )),
            // Invisible track to allow absolute placement of thumb entity. This is narrower than
            // the actual slider, which allows us to position the thumb entity using simple
            // percentages, without having to measure the actual width of the slider thumb.
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    // Track is short by 12px to accommodate the thumb. This should match thumb_size
                    right: Val::Px(12.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                children![(
                    // Thumb
                    DemoSliderThumb,
                    CoreSliderThumb,
                    Node {
                        display: Display::Flex,
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(0.0), // This will be updated by the slider's value
                        ..default()
                    },
                    BorderRadius::all(Val::Px(6.0)),
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

/// Update the visuals of the slider based on the slider state.
fn update_slider_style(
    sliders: Query<
        (
            Entity,
            &SliderValue,
            &SliderRange,
            &Hovered,
            Has<InteractionDisabled>,
        ),
        (
            Or<(
                Changed<SliderValue>,
                Changed<SliderRange>,
                Changed<Hovered>,
                Added<InteractionDisabled>,
            )>,
            With<DemoSlider>,
        ),
    >,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, &mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    for (slider_ent, value, range, hovered, disabled) in sliders.iter() {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_node.left = Val::Percent(range.thumb_position(value.0) * 100.0);
                    thumb_bg.0 = thumb_color(disabled, hovered.0);
                }
            }
        }
    }
}

fn update_slider_style2(
    sliders: Query<(Entity, &Hovered, Has<InteractionDisabled>), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
) {
    removed_disabled.read().for_each(|entity| {
        if let Ok((slider_ent, hovered, disabled)) = sliders.get(entity) {
            for child in children.iter_descendants(slider_ent) {
                if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                    if is_thumb {
                        thumb_bg.0 = thumb_color(disabled, hovered.0);
                    }
                }
            }
        }
    });
}

fn thumb_color(disabled: bool, hovered: bool) -> Color {
    match (disabled, hovered) {
        (true, _) => GRAY.into(),

        (false, true) => SLIDER_THUMB.lighter(0.3),

        _ => SLIDER_THUMB,
    }
}

fn toggle_disabled(
    input: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<
        (Entity, Has<InteractionDisabled>),
        Or<(With<CoreButton>, With<CoreSlider>)>,
    >,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::KeyD) {
        for (entity, disabled) in &mut interaction_query {
            if disabled {
                info!("Widgets enabled");
                commands.entity(entity).remove::<InteractionDisabled>();
            } else {
                info!("Widgets disabled");
                commands.entity(entity).insert(InteractionDisabled);
            }
        }
    }
}
