//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::{prelude::*, ui_navigation::NavRequestSystem, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_system(button_color.after(NavRequestSystem))
        .add_system(press_color.after(NavRequestSystem))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn press_color(
    mut events: EventReader<NavEvent>,
    mut interaction_query: Query<(&mut UiColor, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    for event in events.iter() {
        if let NavEvent::NoChanges {
            from,
            request: NavRequest::Action,
        } = event
        {
            if let Ok((mut color, children)) = interaction_query.get_mut(*from.first()) {
                let mut text = text_query.get_mut(children[0]).unwrap();
                *color = PRESSED_BUTTON.into();
                text.sections[0].value = "Press".to_string();
            }
        }
    }
}

fn button_color(
    mut interaction_query: Query<
        (&Focusable, &mut UiColor, &Children),
        (Changed<Focusable>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (focusable, mut color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        // TODO handle hovering with Hover component
        let new_color = match focusable.state() {
            FocusState::Focused => {
                text.sections[0].value = "Hover".to_string();
                HOVERED_BUTTON
            }
            _ => {
                text.sections[0].value = "Button".to_string();
                NORMAL_BUTTON
            }
        };
        *color = new_color.into();
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                // center button
                margin: UiRect::all(Val::Auto),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Button",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ));
        });
}
