//! Spawns a simple grid layout with nodes laid out covering a white background useful for catching layout rounding errors.
//! Any white lines seen are gaps in the layout are caused by coordinate rounding bugs.

use bevy::{
    color::palettes::css::{DARK_BLUE, MAROON},
    prelude::*,
    ui::UiScale,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(UiScale(1.5))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, UiAntiAlias::On));

    commands
        .spawn((
            Node {
                display: Display::Grid,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                grid_template_rows: vec![RepeatedGridTrack::fr(10, 1.)],
                ..Default::default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .with_children(|commands| {
            for i in 2..12 {
                commands
                    .spawn(Node {
                        display: Display::Grid,
                        grid_template_columns: vec![RepeatedGridTrack::fr(i, 1.)],
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        for _ in 0..i {
                            commands.spawn((
                                Node {
                                    border: UiRect::all(Val::Px(5.)),
                                    ..Default::default()
                                },
                                BackgroundColor(MAROON.into()),
                                BorderColor(DARK_BLUE.into()),
                            ));
                        }
                    });
            }
        });
}
