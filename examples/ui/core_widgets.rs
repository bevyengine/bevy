//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{CoreButton, CoreWidgetsPlugin},
    input_focus::{
        tab_navigation::{TabGroup, TabIndex},
        InputDispatchPlugin,
    },
    picking::hover::IsHovered,
    prelude::*,
    ui::{Depressed, InteractionDisabled},
    winit::WinitSettings,
};
use bevy_ecs::system::SystemId;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CoreWidgetsPlugin, InputDispatchPlugin))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, (button_system, toggle_disabled))
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
    mut buttons: Query<
        (
            &Depressed,
            &IsHovered,
            &InteractionDisabled,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (
            Or<(
                Changed<Depressed>,
                Changed<IsHovered>,
                Changed<InteractionDisabled>,
            )>,
            With<CoreButton>,
        ),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (depressed, hovered, disabled, mut color, mut border_color, children) in &mut buttons {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match (disabled.get(), hovered.get(), depressed.0) {
            // Disabled button
            (true, _, _) => {
                **text = "Disabled".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = GRAY.into();
            }

            // Pressed and hovered button
            (false, true, true) => {
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            }

            // Hovered, unpressed button
            (false, true, false) => {
                **text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }

            // Unhovered button (either pressed or not).
            (false, false, _) => {
                **text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let on_click = commands.register_system(|| {
        info!("Button clicked!");
    });
    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(button(&assets, on_click));
}

fn button(asset_server: &AssetServer, on_click: SystemId) -> impl Bundle + use<> {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        TabGroup::default(),
        children![
            (
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                CoreButton {
                    on_click: Some(on_click),
                },
                IsHovered::default(),
                TabIndex(0),
                BorderColor(Color::BLACK),
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
                )]
            ),
            Text::new("Press 'D' to toggle button disabled state"),
        ],
    )
}

fn toggle_disabled(
    input: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<(Entity, &InteractionDisabled), With<CoreButton>>,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::KeyD) {
        for (entity, disabled) in &mut interaction_query {
            // disabled.0 = !disabled.0;
            if disabled.get() {
                info!("Button enabled");
                commands.entity(entity).insert(InteractionDisabled(false));
            } else {
                info!("Button disabled");
                commands.entity(entity).insert(InteractionDisabled(true));
            }
        }
    }
}
