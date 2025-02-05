//! Demo picking text.

use bevy::{
    prelude::*,
    ui::{text_picking_backend::TextPointer, RelativeCursorPosition},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup,))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Text::new("hello text picking"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(25.0),
                left: Val::Percent(25.0),
                ..default()
            },
            RelativeCursorPosition::default(),
        ))
        .with_children(|cb| {
            cb.spawn(TextSpan::new(
                "i'm a new span\n●●●●i'm the same span...\n····",
            ));
        })
        .observe(|t: Trigger<TextPointer<Click>>| info!("{:?}", t));
}
