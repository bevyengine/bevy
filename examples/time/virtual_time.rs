//! Shows how `Time<Virtual>` can be used to pause, resume, slow down
//! and speed up a game.

use std::time::Duration;

use bevy::{
    color::palettes::css::*, input::common_conditions::input_just_pressed, prelude::*,
    time::common_conditions::on_real_timer,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_virtual_time_sprites,
                move_real_time_sprites,
                toggle_pause.run_if(input_just_pressed(KeyCode::Space)),
                change_time_speed::<1>.run_if(input_just_pressed(KeyCode::ArrowUp)),
                change_time_speed::<-1>.run_if(input_just_pressed(KeyCode::ArrowDown)),
                (update_virtual_time_info_text, update_real_time_info_text)
                    // update the texts on a timer to make them more readable
                    // `on_timer` run condition uses `Virtual` time meaning it's scaled
                    // and would result in the UI updating at different intervals based
                    // on `Time<Virtual>::relative_speed` and `Time<Virtual>::is_paused()`
                    .run_if(on_real_timer(Duration::from_millis(250))),
            ),
        )
        .run();
}

/// `Real` time related marker
#[derive(Component)]
struct RealTime;

/// `Virtual` time related marker
#[derive(Component)]
struct VirtualTime;

/// Setup the example
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut time: ResMut<Time<Virtual>>) {
    // start with double `Virtual` time resulting in one of the sprites moving at twice the speed
    // of the other sprite which moves based on `Real` (unscaled) time
    time.set_relative_speed(2.);

    commands.spawn(Camera2dBundle::default());

    let virtual_color = GOLD.into();
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
                    "CONTROLS\nUn/Pause: Space\nSpeed+: Up\nSpeed-: Down",
                    TextStyle {
                        font_size,
                        color: Color::srgb(0.85, 0.85, 0.85),
                        ..default()
                    },
                )
                .with_text_justify(JustifyText::Center),
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
                .with_text_justify(JustifyText::Right),
                VirtualTime,
            ));
        });
}

/// Move sprites using `Real` (unscaled) time
fn move_real_time_sprites(
    mut sprite_query: Query<&mut Transform, (With<Sprite>, With<RealTime>)>,
    // `Real` time which is not scaled or paused
    time: Res<Time<Real>>,
) {
    for mut transform in sprite_query.iter_mut() {
        // move roughly half the screen in a `Real` second
        // when the time is scaled the speed is going to change
        // and the sprite will stay still the time is paused
        transform.translation.x = get_sprite_translation_x(time.elapsed_seconds());
    }
}

/// Move sprites using `Virtual` (scaled) time
fn move_virtual_time_sprites(
    mut sprite_query: Query<&mut Transform, (With<Sprite>, With<VirtualTime>)>,
    // the default `Time` is either `Time<Virtual>` in regular systems
    // or `Time<Fixed>` in fixed timestep systems so `Time::delta()`,
    // `Time::elapsed()` will return the appropriate values either way
    time: Res<Time>,
) {
    for mut transform in sprite_query.iter_mut() {
        // move roughly half the screen in a `Virtual` second
        // when time is scaled using `Time<Virtual>::set_relative_speed` it's going
        // to move at a different pace and the sprite will stay still when time is
        // `Time<Virtual>::is_paused()`
        transform.translation.x = get_sprite_translation_x(time.elapsed_seconds());
    }
}

fn get_sprite_translation_x(elapsed: f32) -> f32 {
    elapsed.sin() * 500.
}

/// Update the speed of `Time<Virtual>.` by `DELTA`
fn change_time_speed<const DELTA: i8>(mut time: ResMut<Time<Virtual>>) {
    let time_speed = (time.relative_speed() + DELTA as f32)
        .round()
        .clamp(0.25, 5.);

    // set the speed of the virtual time to speed it up or slow it down
    time.set_relative_speed(time_speed);
}

/// pause or resume `Relative` time
fn toggle_pause(mut time: ResMut<Time<Virtual>>) {
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
    }
}

/// Update the `Real` time info text
fn update_real_time_info_text(time: Res<Time<Real>>, mut query: Query<&mut Text, With<RealTime>>) {
    for mut text in &mut query {
        text.sections[0].value = format!(
            "REAL TIME\nElapsed: {:.1}\nDelta: {:.5}\n",
            time.elapsed_seconds(),
            time.delta_seconds(),
        );
    }
}

/// Update the `Virtual` time info text
fn update_virtual_time_info_text(
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
