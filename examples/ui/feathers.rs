//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    core_widgets::{CoreButton, CoreSlider, CoreWidgetsPlugin, SliderValue},
    ecs::system::SystemId,
    feathers::{
        controls::{button, slider, ButtonProps, ButtonVariant, SliderProps},
        dark::create_dark_theme,
        theme::{self, corners::RoundedCorners, ThemeBackgroundColor, UiTheme, UseTheme},
        FeathersPlugin,
    },
    input_focus::{tab_navigation::TabGroup, InputDispatchPlugin},
    prelude::*,
    ui::InteractionDisabled,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CoreWidgetsPlugin,
            InputDispatchPlugin,
            FeathersPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(DemoWidgetStates { slider_value: 50.0 })
        .add_systems(Startup, setup)
        .add_systems(Update, (update_widget_values, toggle_disabled))
        .run();
}

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

fn setup(mut commands: Commands) {
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
    commands.spawn(demo_root(on_click, on_change_value));
}

fn demo_root(on_click: SystemId, on_change_value: SystemId<In<f32>>) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        TabGroup::default(),
        ThemeBackgroundColor(theme::tokens::WINDOW_BG),
        children![(
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(8.0),
                width: Val::Percent(30.),
                min_width: Val::Px(200.),
                ..default()
            },
            children![
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    children![
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Normal"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::All,
                            overrides: (),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Disabled"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::All,
                            overrides: (InteractionDisabled),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Primary"), UseTheme)),
                            variant: ButtonVariant::Primary,
                            corners: RoundedCorners::All,
                            overrides: (),
                            // ..Default::default()
                        }),
                    ]
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        column_gap: Val::Px(1.0),
                        ..default()
                    },
                    children![
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Left"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::Left,
                            overrides: (),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Center"), UseTheme)),
                            variant: ButtonVariant::Normal,
                            corners: RoundedCorners::None,
                            overrides: (),
                            // ..Default::default()
                        }),
                        button(ButtonProps {
                            on_click: Some(on_click),
                            children: Spawn((Text::new("Right"), UseTheme)),
                            variant: ButtonVariant::Primary,
                            corners: RoundedCorners::Right,
                            overrides: (),
                            // ..Default::default()
                        }),
                    ]
                ),
                button(ButtonProps {
                    on_click: Some(on_click),
                    children: Spawn((Text::new("Button"), UseTheme)),
                    variant: ButtonVariant::Normal,
                    corners: RoundedCorners::All,
                    overrides: (),
                    // ..Default::default()
                }),
                slider(SliderProps {
                    min: 0.0,
                    max: 100.0,
                    value: 20.0,
                    precision: 1,
                    on_change: None,
                    // on_change: Some(on_change_value)
                }),
            ]
        ),],
    )
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
