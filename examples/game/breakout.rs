//! A simplified implementation of the classic game "Breakout"

use bevy::{
    core::FixedTimestep,
    math::{const_vec2, const_vec3},
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.
// The `const_vec3!` macros are needed as functions that operate on floats cannot be constant in Rust.
const PADDLE_HEIGHT: f32 = -215.0;
const PADDLE_SIZE: Vec3 = const_vec3!([120.0, 30.0, 0.0]);
const PADDLE_SPEED: f32 = 500.0;
const PADDLE_BOUNDS: f32 = 380.0;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const BALL_STARTING_POSITION: Vec3 = const_vec3!([0.0, -50.0, 1.0]);
const BALL_SIZE: Vec3 = const_vec3!([30.0, 30.0, 0.0]);
const BALL_SPEED: f32 = 400.0;
const INITIAL_BALL_DIRECTION: Vec2 = const_vec2!([0.5, -0.5]);

const PLAY_AREA_BOUNDS: Vec2 = const_vec2!([900.0, 600.0]);
const WALL_THICKNESS: f32 = 10.0;

const BRICK_ROWS: u8 = 4;
const BRICK_COLUMNS: u8 = 5;
const BRICK_SPACING: f32 = 20.0;
const BRICK_SIZE: Vec3 = const_vec3!([150.0, 30.0, 1.0]);

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::rgb(0.95, 0.95, 0.95);
const PADDLE_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const BALL_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const BRICK_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const WALL_COLOR: Color = Color::DARK_GRAY;
const TEXT_COLOR: Color = Color::BLUE;
const SCORE_COLOR: Color = Color::RED;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(paddle_movement_system)
                .with_system(ball_collision_system)
                .with_system(ball_movement_system),
        )
        .add_system(scoreboard_system)
        .add_system(bevy::input::system::exit_on_esc_system)
        .run();
}

#[derive(Component)]
struct Paddle {
    speed: f32,
}

#[derive(Component)]
struct Ball {
    velocity: Vec3,
}

#[derive(Component)]
enum Collider {
    Solid,
    Scorable,
    Paddle,
}

struct Scoreboard {
    score: usize,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Add the game's entities to our world

    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    // paddle
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, PADDLE_HEIGHT, 0.0),
                scale: PADDLE_SIZE,
                ..default()
            },
            sprite: Sprite {
                color: PADDLE_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Paddle {
            speed: PADDLE_SPEED,
        })
        .insert(Collider::Paddle);
    // ball
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                scale: BALL_SIZE,
                translation: BALL_STARTING_POSITION,
                ..default()
            },
            sprite: Sprite {
                color: BALL_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Ball {
            // We can create a velocity by multiplying our speed by a normalized direction.
            velocity: BALL_SPEED * INITIAL_BALL_DIRECTION.extend(0.0).normalize(),
        });
    // scoreboard
    commands.spawn_bundle(TextBundle {
        text: Text {
            sections: vec![
                TextSection {
                    value: "Score: ".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: SCOREBOARD_FONT_SIZE,
                        color: TEXT_COLOR,
                    },
                },
                TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: SCOREBOARD_FONT_SIZE,
                        color: SCORE_COLOR,
                    },
                },
            ],
            ..default()
        },
        style: Style {
            position_type: PositionType::Absolute,
            position: Rect {
                top: SCOREBOARD_TEXT_PADDING,
                left: SCOREBOARD_TEXT_PADDING,
                ..default()
            },
            ..default()
        },
        ..default()
    });

    // left
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-PLAY_AREA_BOUNDS.x / 2.0, 0.0, 0.0),
                scale: Vec3::new(WALL_THICKNESS, PLAY_AREA_BOUNDS.y + WALL_THICKNESS, 1.0),
                ..default()
            },
            sprite: Sprite {
                color: WALL_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Collider::Solid);
    // right
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(PLAY_AREA_BOUNDS.x / 2.0, 0.0, 0.0),
                scale: Vec3::new(WALL_THICKNESS, PLAY_AREA_BOUNDS.y + WALL_THICKNESS, 1.0),
                ..default()
            },
            sprite: Sprite {
                color: WALL_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Collider::Solid);
    // bottom
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, -PLAY_AREA_BOUNDS.y / 2.0, 0.0),
                scale: Vec3::new(PLAY_AREA_BOUNDS.x + WALL_THICKNESS, WALL_THICKNESS, 1.0),
                ..default()
            },
            sprite: Sprite {
                color: WALL_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Collider::Solid);
    // top
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, PLAY_AREA_BOUNDS.y / 2.0, 0.0),
                scale: Vec3::new(PLAY_AREA_BOUNDS.x + WALL_THICKNESS, WALL_THICKNESS, 1.0),
                ..default()
            },
            sprite: Sprite {
                color: WALL_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Collider::Solid);

    // Add bricks
    let bricks_width = BRICK_COLUMNS as f32 * (BRICK_SIZE.x + BRICK_SPACING) - BRICK_SPACING;
    // center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(bricks_width - BRICK_SIZE.x) / 2.0, 100.0, 0.0);
    for row in 0..BRICK_ROWS {
        let y_position = row as f32 * (BRICK_SIZE.y + BRICK_SPACING);
        for column in 0..BRICK_COLUMNS {
            let brick_position = Vec3::new(
                column as f32 * (BRICK_SIZE.x + BRICK_SPACING),
                y_position,
                0.0,
            ) + bricks_offset;
            // brick
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: BRICK_COLOR,
                        ..default()
                    },
                    transform: Transform {
                        translation: brick_position,
                        scale: BRICK_SIZE,
                        ..default()
                    },
                    ..default()
                })
                .insert(Collider::Scorable);
        }
    }
}

fn paddle_movement_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform)>,
) {
    let (paddle, mut transform) = query.single_mut();
    let mut direction = 0.0;
    if keyboard_input.pressed(KeyCode::Left) {
        direction -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::Right) {
        direction += 1.0;
    }

    let translation = &mut transform.translation;
    // move the paddle horizontally
    translation.x += direction * paddle.speed * TIME_STEP;
    // bound the paddle within the walls
    translation.x = translation.x.min(PADDLE_BOUNDS).max(-PADDLE_BOUNDS);
}

fn ball_movement_system(mut ball_query: Query<(&Ball, &mut Transform)>) {
    let (ball, mut transform) = ball_query.single_mut();
    transform.translation += ball.velocity * TIME_STEP;
}

fn scoreboard_system(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[1].value = format!("{}", scoreboard.score);
}

fn ball_collision_system(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut ball_query: Query<(&mut Ball, &Transform)>,
    collider_query: Query<(Entity, &Collider, &Transform)>,
) {
    let (mut ball, ball_transform) = ball_query.single_mut();
    let ball_size = ball_transform.scale.truncate();
    let velocity = &mut ball.velocity;

    // check collision with walls
    for (collider_entity, collider, transform) in collider_query.iter() {
        let collision = collide(
            ball_transform.translation,
            ball_size,
            transform.translation,
            transform.scale.truncate(),
        );
        if let Some(collision) = collision {
            // scorable colliders should be despawned and increment the scoreboard on collision
            if let Collider::Scorable = *collider {
                scoreboard.score += 1;
                commands.entity(collider_entity).despawn();
            }

            // reflect the ball when it collides
            let mut reflect_x = false;
            let mut reflect_y = false;

            // only reflect if the ball's velocity is going in the opposite direction of the
            // collision
            match collision {
                Collision::Left => reflect_x = velocity.x > 0.0,
                Collision::Right => reflect_x = velocity.x < 0.0,
                Collision::Top => reflect_y = velocity.y < 0.0,
                Collision::Bottom => reflect_y = velocity.y > 0.0,
                Collision::Inside => { /* do nothing */ }
            }

            // reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                velocity.x = -velocity.x;
            }

            // reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                velocity.y = -velocity.y;
            }

            // break if this collide is on a solid, otherwise continue check whether a solid is
            // also in collision
            if let Collider::Solid = *collider {
                break;
            }
        }
    }
}
