//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{CoreButton, CoreWidgetsPlugin},
    input_focus::{
        tab_navigation::{TabGroup, TabIndex},
        InputDispatchPlugin,
    },
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
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
            Has<InteractionDisabled>,
        ),
        // Note: we can't use change detection on `InteractionDisabled` here because
        // it's a marker, and query filters don't detect removals. For this example we will
        // just update the button color every frame.
        With<CoreButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    for (depressed, mut color, mut border_color, children, disabled) in &mut buttons {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match (disabled, depressed.0) {
            (true, _) => {
                **text = "Disabled".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = GRAY.into();
            }

            (false, true) => {
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            }
            // Interaction::Hovered => {
            //     **text = "Hover".to_string();
            //     *color = HOVERED_BUTTON.into();
            //     border_color.0 = Color::WHITE;
            // }
            (false, false) => {
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
                    // horizontally center child text
                    justify_content: JustifyContent::Center,
                    // vertically center child text
                    align_items: AlignItems::Center,
                    ..default()
                },
                CoreButton {
                    on_click: Some(on_click),
                },
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
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<(Entity, Has<InteractionDisabled>), With<CoreButton>>,
) {
    if input.just_pressed(KeyCode::KeyD) {
        for (button, disabled) in &mut interaction_query {
            if disabled {
                info!("Button enabled");
                commands.entity(button).remove::<InteractionDisabled>();
            } else {
                info!("Button disabled");
                commands.entity(button).insert(InteractionDisabled);
            }
        }
    }
}
