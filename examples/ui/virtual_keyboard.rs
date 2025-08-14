//! Virtual keyboard example

use bevy::{
    color::palettes::css::NAVY,
    core_widgets::{Activate, CoreWidgetsPlugins},
    ecs::relationship::RelatedSpawnerCommands,
    feathers::{
        controls::virtual_keyboard, dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugin,
    },
    input_focus::{tab_navigation::TabNavigationPlugin, InputDispatchPlugin},
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CoreWidgetsPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
            FeathersPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct VirtualKey(String);

fn on_virtual_key_pressed(
    In(Activate(virtual_key_entity)): In<Activate>,
    virtual_key_query: Query<&VirtualKey>,
) {
    if let Ok(VirtualKey(label)) = virtual_key_query.get(virtual_key_entity) {
        println!("key pressed: {label}");
    }
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2d);
    let callback = commands.register_system(on_virtual_key_pressed);

    let layout = [
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", ".", ","],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L", "'"],
        vec!["Z", "X", "C", "V", "B", "N", "M", "-", "/"],
        vec!["space", "enter", "backspace"],
        vec!["left", "right", "up", "down", "home", "end"],
    ];

    let keys_iter = layout.into_iter().map(|row| {
        row.into_iter()
            .map(|label| {
                let label_string = label.to_string();
                (label_string.clone(), VirtualKey(label_string))
            })
            .collect()
    });

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::End,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent: &mut RelatedSpawnerCommands<ChildOf>| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        border: px(5).into(),
                        row_gap: px(5),
                        padding: px(5).into(),
                        align_items: AlignItems::Center,
                        margin: px(25).into(),
                        ..Default::default()
                    },
                    BackgroundColor(NAVY.into()),
                    BorderColor::all(Color::WHITE),
                    BorderRadius::all(px(10)),
                ))
                .with_children(|parent: &mut RelatedSpawnerCommands<ChildOf>| {
                    parent.spawn(Text::new("virtual keyboard"));
                    parent.spawn(virtual_keyboard(keys_iter, callback));
                });
        });
}
