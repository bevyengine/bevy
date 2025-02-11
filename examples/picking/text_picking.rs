//! Demo picking text.

use bevy::{prelude::*, text::text_pointer::TextPointer, ui::RelativeCursorPosition};

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
            // TODO: find a better text string that shows multibyte adherence within bevy's font
            // subset.
            cb.spawn(TextSpan::new(
                "i'm a new span\n●●●●i'm the same span...\n····",
            ))
            .observe(|mut t: Trigger<TextPointer<Click>>| {
                info!("Span specific observer clicked! {:?}", t);
                t.propagate(false);
            });
        })
        .observe(|t: Trigger<TextPointer<Click>>| {
            info!("Root observer clicked! {:?}", t);
            // TODO: Visualize a cursor on click.
        });

    commands
        .spawn((Text2d::new("I'm a Text2d"),))
        .with_children(|cb| {
            cb.spawn(TextSpan("And I'm a text span!".into())).observe(
                |_: Trigger<Pointer<Click>>| {
                    info!("text2d span click");
                },
            );
        })
        .observe(|_t: Trigger<TextPointer<Click>>| {
            info!("Textmode clicked text2d");
        })
        .observe(|_t: Trigger<Pointer<Click>>| info!("clicked text2d"));
}
