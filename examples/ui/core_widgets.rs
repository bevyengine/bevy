//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{CoreButton, CoreWidgetsPlugin},
    prelude::*,
    ui::Depressed,
    winit::WinitSettings,
};
use bevy_ecs::system::SystemId;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CoreWidgetsPlugin))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (
            &Depressed,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Depressed>, With<CoreButton>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (depressed, mut color, mut border_color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match depressed.0 {
            true => {
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            }
            // Interaction::Hovered => {
            //     **text = "Hover".to_string();
            //     *color = HOVERED_BUTTON.into();
            //     border_color.0 = Color::WHITE;
            // }
            false => {
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
            ..default()
        },
        children![(
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
        )],
    )
}
