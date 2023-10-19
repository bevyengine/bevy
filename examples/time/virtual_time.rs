//! Shows how `Time<Virtual>` can be used to pause, resume, slow down
//! and speed up a game.

use std::time::Duration;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_internal::{math::bool, time::common_conditions::on_timer, window::WindowResolution};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resizable: false,
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGTH),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_virtual_time_sprites,
                move_real_time_sprites,
                toggle_pause.run_if(input_just_pressed(KeyCode::Space)),
                change_time_speed::<1>.run_if(input_just_pressed(KeyCode::Up)),
                change_time_speed::<-1>.run_if(input_just_pressed(KeyCode::Down)),
                (update_virtual_time_text, update_real_time_text)
                    .run_if(on_timer(Duration::from_millis(250))),
            ),
        )
        .run();
}

const WINDOW_WIDTH: f32 = 1000.;
const WINDOW_HEIGTH: f32 = 600.;

#[derive(Component)]
struct RealTime;

#[derive(Component)]
struct VirtualTime;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let virtual_color = Color::GOLD;
    let sprite_scale = Vec2::splat(0.5).extend(1.);
    let texture_handle = asset_server.load("branding/icon.png");

    // the sprite moving based on real time
    commands.spawn((
        SpriteBundle {
            texture: texture_handle.clone(),
            transform: Transform::from_scale(sprite_scale),
            ..default()
        },
        RealTime,
    ));

    // the sprite moving based on virtual time
    commands.spawn((
        SpriteBundle {
            texture: texture_handle,
            sprite: Sprite {
                color: virtual_color,
                ..default()
            },
            transform: Transform {
                scale: sprite_scale,
                translation: Vec3::new(0., -160., 0.),
                ..default()
            },
            ..default()
        },
        VirtualTime,
    ));

    // info UI
    let font_size = 40.;

    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                justify_content: JustifyContent::SpaceBetween,
                width: Val::Percent(100.),
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            // real time info
            builder.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size,
                        ..default()
                    },
                ),
                RealTime,
            ));

            // keybindings
            builder.spawn(
                TextBundle::from_section(
                    "CONTROLS\nUn/Pause: Space\nSpeed+: +/Up\nSpeed-: -/Down",
                    TextStyle {
                        font_size,
                        color: Color::rgb(0.7, 0.7, 0.7),
                        ..default()
                    },
                )
                .with_text_alignment(TextAlignment::Center),
            );

            // virtual time info
            builder.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size,
                        color: virtual_color,
                        ..default()
                    },
                )
                .with_text_alignment(TextAlignment::Right),
                VirtualTime,
            ));
        });
}

fn move_real_time_sprites(
    mut sprite_query: Query<&mut Transform, (With<Sprite>, With<RealTime>)>,
    time: Res<Time<Real>>,
) {
    for mut transform in sprite_query.iter_mut() {
        transform.translation.x = get_sprite_translation_x(time.elapsed_seconds());
    }
}

fn move_virtual_time_sprites(
    mut sprite_query: Query<&mut Transform, (With<Sprite>, With<VirtualTime>)>,
    // in Update systems this's Time<Virtual> so scaling (todo: whatever is the proper name, also type ref) and applies meaning
    time: Res<Time>,
) {
    for mut transform in sprite_query.iter_mut() {
        // move roughly half the screen in a (scaled/virtual) second
        // when the time is scaled the speed is going to change
        // and the sprite will stay still the the time is paused
        transform.translation.x = get_sprite_translation_x(time.elapsed_seconds());
    }
}

fn get_sprite_translation_x(elapsed: f32) -> f32 {
    elapsed.sin() * (WINDOW_WIDTH / 2. - 80.)
}

fn change_time_speed<const DELTA: i8>(mut time: ResMut<Time<Virtual>>) {
    let time_speed = (time.relative_speed() + DELTA as f32)
        .round()
        .clamp(0.25, 5.);
    time.set_relative_speed(time_speed);
}

fn toggle_pause(mut time: ResMut<Time<Virtual>>) {
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
    }
}

fn update_virtual_time_text(
    time: Res<Time<Virtual>>,
    mut query: Query<&mut Text, With<VirtualTime>>,
) {
    for mut text in &mut query {
        text.sections[0].value = format!(
            "VIRTUAL TIME\nElapsed: {:.1}\nDelta: {:.5}\nSpeed: {:.2}",
            time.elapsed_seconds(),
            time.delta_seconds(),
            time.relative_speed()
        );
    }
}

fn update_real_time_text(time: Res<Time<Real>>, mut query: Query<&mut Text, With<RealTime>>) {
    for mut text in &mut query {
        text.sections[0].value = format!(
            "REAL TIME\nElapsed: {:.1}\nDelta: {:.5}\n",
            time.elapsed_seconds(),
            time.delta_seconds(),
        );
    }
}
