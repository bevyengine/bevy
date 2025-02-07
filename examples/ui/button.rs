//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::{color::palettes::basic::*, prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
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
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .observe(
                    |trigger: Trigger<Pointer<Over>>,
                     mut text_query: Query<&mut Text>,
                     mut button_query: Query<(&mut BackgroundColor, &Children)>| {
                        let (mut background_color, children) =
                            button_query.get_mut(trigger.target()).unwrap();
                        *background_color = HOVERED_BUTTON.into();
                        let mut text = text_query.get_mut(children[0]).unwrap();
                        **text = "Hover".to_string();
                    },
                )
                .observe(
                    |trigger: Trigger<Pointer<Pressed>>,
                     mut text_query: Query<&mut Text>,
                     mut button_query: Query<(&mut BackgroundColor, &Children)>| {
                        let (mut background_color, children) =
                            button_query.get_mut(trigger.target()).unwrap();
                        *background_color = PRESSED_BUTTON.into();
                        let mut text = text_query.get_mut(children[0]).unwrap();
                        **text = "Press".to_string();
                    },
                )
                .observe(
                    |trigger: Trigger<Pointer<DragEnd>>,
                     mut text_query: Query<&mut Text>,
                     mut button_query: Query<(&mut BackgroundColor, &Children)>| {
                        let (mut background_color, children) =
                            button_query.get_mut(trigger.target()).unwrap();
                        *background_color = PRESSED_BUTTON.into();
                        let mut text = text_query.get_mut(children[0]).unwrap();
                        **text = "Released".to_string();
                    },
                )
                .observe(
                    |trigger: Trigger<Pointer<Out>>,
                     mut text_query: Query<&mut Text>,
                     mut button_query: Query<(&mut BackgroundColor, &Children)>| {
                        let (mut background_color, children) =
                            button_query.get_mut(trigger.target()).unwrap();
                        *background_color = NORMAL_BUTTON.into();
                        let mut text = text_query.get_mut(children[0]).unwrap();
                        **text = "Button".to_string();
                    },
                )
                .observe(
                    |trigger: Trigger<Pointer<Cancel>>| {
                        // let (mut background_color, children) =
                        //     button_query.get_mut(trigger.target()).unwrap();
                        // *background_color = NORMAL_BUTTON.into();
                        // let mut text = text_query.get_mut(children[0]).unwrap();
                        // **text = "Button".to_string();
                        println!("point cancelled");
                    },
                )
                .with_child((
                    Text::new("Button"),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                ));
        });
}
