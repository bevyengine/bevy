//! Demo picking text.

use bevy::{
    color::palettes::css::GREEN,
    prelude::*,
    text::{cosmic_text::Affinity, text_pointer::TextPointer, TextLayoutInfo},
    ui::RelativeCursorPosition,
    window::WindowResolution,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::default().with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<CursorTarget>()
        .add_systems(Startup, (setup,))
        .add_systems(Update, cursor_to_target)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(2.0, 12.0)),
            color: GREEN.into(),
            ..default()
        },
        MyCursor,
    ));
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
                // Prevent parent observer triggering when span clicked.
                t.propagate(false);
            });
        })
        .observe(|t: Trigger<TextPointer<Click>>| {
            info!("Root observer clicked! {:?}", t);
            // TODO: Visualize a cursor on click.
        });

    commands
        .spawn((
            Text2d::new("I'm a Text2d"),
            Transform::from_xyz(10., 10., 0.),
        ))
        .with_children(|cb| {
            cb.spawn(TextSpan("And I'm a text span!".into())).observe(
                |mut t: Trigger<TextPointer<Click>>| {
                    info!("Textmode clicked text2d span {:?}", t);
                    // t.propagate(false);
                },
            );
        })
        .observe(
            |t: Trigger<TextPointer<Click>>,
             mut target: ResMut<CursorTarget>,
             q: Query<(&Transform, &TextLayoutInfo)>| {
                info!("Textmode clicked text2d {:?} ", t);

                let Ok((transform, tli)) = q.get(t.target()) else {
                    return;
                };

                const LINEHEIGHT: f32 = 12.0;

                let xoff = match t.cursor.affinity {
                    Affinity::Before => -t.glyph.size.x / 2.0,
                    Affinity::After => t.glyph.size.x / 2.0,
                };

                let xpos = transform.translation.x + t.glyph.position.x + xoff - tli.size.x / 2.;
                let ypos = transform.translation.y + (t.cursor.line + 1) as f32 * LINEHEIGHT
                    - tli.size.y / 2.;

                target.0 = Vec3::new(xpos, ypos, transform.translation.z);
            },
        )
        .observe(|_t: Trigger<Pointer<Click>>| info!("clicked text2d"));
}

#[derive(Component)]
struct MyCursor;

#[derive(Resource, Default)]
struct CursorTarget(pub Vec3);

fn cursor_to_target(target: Res<CursorTarget>, mut q: Query<&mut Transform, With<MyCursor>>) {
    for mut t in &mut q {
        t.translation = target.0
    }
}
