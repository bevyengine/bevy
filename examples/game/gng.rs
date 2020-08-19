use bevy::{
    input::{keyboard::KeyCode, Input},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(player_movement_system.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    audio_output: Res<AudioOutput>,
) {
    let texture_handle = asset_server
        .load_sync(&mut textures, "assets/textures/gng/player.png")
        .unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let texture_atlas = TextureAtlas::from_grid(texture_handle, texture.size, 26, 13);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands
        .spawn(Camera2dComponents::default())
        .spawn(SpriteSheetComponents {
            texture_atlas: texture_atlas_handle,
            scale: Scale(1.0),
            ..Default::default()
        })
        .with(Timer::from_seconds(0.1));

    let music = asset_server
        .load("assets/sounds/Windless Slopes.mp3")
        .unwrap();
    audio_output.play(music);
}

fn player_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(
        &mut Timer,
        &mut TextureAtlasSprite,
        &mut Translation,
        &mut Rotation,
    )>,
) {
    for (mut timer, mut sprite, mut translation, mut rotation) in &mut query.iter() {
        let mut h_direction = 0.0;
        let mut v_direction = 0.0;

        if keyboard_input.just_released(KeyCode::Right)
            || keyboard_input.just_released(KeyCode::Left)
            || keyboard_input.just_released(KeyCode::Down)
        {
            sprite.index = 0;
        }

        if keyboard_input.pressed(KeyCode::Left) {
            h_direction -= 1.0;

            *rotation = Rotation::from_rotation_yxz(3.14, 0.0, 0.0);

            if timer.finished {
                if sprite.index == 0 || sprite.index > 5 {
                    sprite.index = 1;
                } else {
                    sprite.index += 1;
                }
                timer.reset();
            }
        } else {
            *rotation = Rotation::from_rotation_yxz(0.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Right) {
            h_direction += 1.0;

            if timer.finished {
                if sprite.index == 0 || sprite.index > 5 {
                    sprite.index = 1;
                } else {
                    sprite.index += 1;
                }
                timer.reset();
            }
        }

        if keyboard_input.pressed(KeyCode::Up) {
            v_direction += 1.0;

            if timer.finished {
                if sprite.index < 18 || sprite.index > 20 {
                    sprite.index = 18;
                } else {
                    sprite.index += 1;
                }
                timer.reset();
            }
        }

        if keyboard_input.pressed(KeyCode::Down) {
            if keyboard_input.pressed(KeyCode::LShift) {
                h_direction = 0.0;
                sprite.index = 7;
            } else {
                v_direction -= 1.0;

                if timer.finished {
                    if sprite.index < 18 || sprite.index > 20 {
                        sprite.index = 18;
                    } else {
                        sprite.index += 1;
                    }
                    timer.reset();
                }
            }
        }

        if keyboard_input.pressed(KeyCode::Space) {
            if keyboard_input.pressed(KeyCode::Right) {
                v_direction += 1.0;
                sprite.index = 8;
            } else if keyboard_input.pressed(KeyCode::Left) {
                v_direction += 1.0;
                sprite.index = 8;
            } else {
                v_direction += 1.0;
                sprite.index = 9;
            }
        }

        *translation.0.x_mut() += time.delta_seconds * h_direction * 200.0;
        // bound within the walls
        *translation.0.x_mut() = f32::max(-624.0, f32::min(624.0, translation.0.x()));

        *translation.0.y_mut() += time.delta_seconds * v_direction * 100.0;
        // bound within the walls
        *translation.0.y_mut() = f32::max(-340.0, f32::min(340.0, translation.0.y()));
    }
}
