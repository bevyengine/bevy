//! This examples shows how to use bevy's frame pacing and frame limiting
//!
//! The demo should also let you feel the effect of enabling it

use bevy::{
    color::palettes,
    diagnostic::LogDiagnosticsPlugin,
    prelude::*,
    render::framepace::{
        debug::FramepaceDiagnosticsPlugin, FramepacePlugin, FramepaceSettings, Limiter,
    },
    window::{CursorIcon, SystemCursorIcon, Window},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            LogDiagnosticsPlugin::default(),
            FramepacePlugin,
            FramepaceDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_plugin, update_ui, update_cursor))
        .run();
}

#[derive(Component)]
struct EnableText;

fn toggle_plugin(mut settings: ResMut<FramepaceSettings>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        settings.limiter = match settings.limiter {
            Limiter::Auto => Limiter::Off,
            Limiter::Off => Limiter::from_framerate(30.0),
            Limiter::Manual(_) => Limiter::Auto,
        }
    }
}

fn update_ui(mut text: Single<&mut TextSpan, With<EnableText>>, settings: Res<FramepaceSettings>) {
    text.0 = format!("{}", settings.limiter);
}

pub fn update_cursor(window: Single<&Window>, mut gizmos: Gizmos) {
    if let Some(pos) = window.cursor_position() {
        let pos = Vec2::new(pos.x - window.width() / 2.0, window.height() / 2.0 - pos.y);
        gizmos.circle_2d(pos, 10.0, palettes::basic::GREEN);
    }
}

/// set up the scene
fn setup(mut commands: Commands, window: Single<Entity, With<Window>>) {
    commands
        .entity(*window)
        .insert(CursorIcon::System(SystemCursorIcon::Crosshair));
    commands.spawn((
        Camera2d,
        Camera {
            order: 10,
            ..default()
        },
    ));
    commands.spawn(Camera3d::default());

    // UI
    let text_font = TextFont {
        font_size: FontSize::Px(50.0),
        ..default()
    };
    commands
        .spawn(Text::default())
        .with_child((TextSpan::new("Frame pacing: "), text_font.clone()))
        .with_child((TextSpan::new(""), text_font.clone(), EnableText))
        .with_child((TextSpan::new("\n[press space]"), text_font));
}
