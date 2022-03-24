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
const PADDLE_SIZE: Vec3 = const_vec3!([120.0, 30.0, 0.0]);
const PADDLE_Y_OFFSET: f32 = -215.0;
const PADDLE_SPEED: f32 = 500.0;
// How close can the paddle get to the wall
const PADDLE_PADDING: f32 = 10.0;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const BALL_STARTING_POSITION: Vec3 = const_vec3!([0.0, -50.0, 1.0]);
const BALL_SIZE: Vec3 = const_vec3!([30.0, 30.0, 0.0]);
const BALL_SPEED: f32 = 400.0;
const INITIAL_BALL_DIRECTION: Vec2 = const_vec2!([0.5, -0.5]);

const PLAY_AREA_BOUNDS: Vec2 = const_vec2!([900.0, 600.0]);
const WALL_THICKNESS: f32 = 10.0;

const BRICK_ROWS: u8 = 4;
const BRICK_COLUMNS: u8 = 5;
const BRICK_SPACING_X: f32 = 20.0;
const BRICK_SPACING_Y: f32 = 20.0;
const BRICK_Y_OFFSET: f32 = 100.0;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const PADDLE_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const BALL_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const BRICK_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(move_paddle)
                .with_system(check_for_collisions)
                .with_system(apply_velocity),
        )
        .add_system(update_scoreboard)
        .add_system(bevy::input::system::exit_on_esc_system)
        .run();
}

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Component)]
struct Brick;

// This bundle is a collection of the components that define a "wall" in our game
#[derive(Bundle)]
struct WallBundle {
    // You can nest bundles inside of other bundles like this
    // Allowing you to compose their functionality
    #[bundle]
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

/// Which side of the arena is this wall located on?
enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl WallBundle {
    // This "builder method" allows us to reuse logic across our wall entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(orientation: WallLocation) -> WallBundle {
        // This allows us to just use `Left`, rather than `WallOrientation::Left` everywhere
        use WallLocation::*;

        let position = match orientation {
            Left => Vec2::new(-PLAY_AREA_BOUNDS.x / 2.0, 0.0),
            Right => Vec2::new(PLAY_AREA_BOUNDS.x / 2.0, 0.0),
            Bottom => Vec2::new(0.0, -PLAY_AREA_BOUNDS.y / 2.0),
            Top => Vec2::new(0.0, PLAY_AREA_BOUNDS.y / 2.0),
        };

        let size = match orientation {
            Left => Vec2::new(WALL_THICKNESS, PLAY_AREA_BOUNDS.y + WALL_THICKNESS),
            Right => Vec2::new(WALL_THICKNESS, PLAY_AREA_BOUNDS.y + WALL_THICKNESS),
            Bottom => Vec2::new(PLAY_AREA_BOUNDS.x + WALL_THICKNESS, WALL_THICKNESS),
            Top => Vec2::new(PLAY_AREA_BOUNDS.x + WALL_THICKNESS, WALL_THICKNESS),
        };

        WallBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                    // This is used to determine the order of our sprites
                    translation: position.extend(0.0),
                    // The z-scale of 2D objects must always be 1.0,
                    // or their ordering will be affected in surpirising ways.
                    // See https://github.com/bevyengine/bevy/issues/4149
                    scale: size.extend(1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: WALL_COLOR,
                    ..default()
                },
                ..default()
            },
            collider: Collider,
        }
    }
}

// This resource tracks the game's score
struct Scoreboard {
    score: usize,
}

// Add the game's entities to our world
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    // Paddle
    commands
        .spawn()
        .insert(Paddle)
        .insert_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, PADDLE_Y_OFFSET, 0.0),
                scale: PADDLE_SIZE,
                ..default()
            },
            sprite: Sprite {
                color: PADDLE_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Collider);

    // Ball
    commands
        .spawn()
        .insert(Ball)
        .insert_bundle(SpriteBundle {
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
        .insert(Velocity(INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED));

    // Scoreboard
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

    // Walls
    commands.spawn_bundle(WallBundle::new(WallLocation::Left));
    commands.spawn_bundle(WallBundle::new(WallLocation::Right));
    commands.spawn_bundle(WallBundle::new(WallLocation::Bottom));
    commands.spawn_bundle(WallBundle::new(WallLocation::Top));

    // Bricks
    let total_width_of_bricks = PLAY_AREA_BOUNDS.x - 2. * BRICK_SPACING_X;

    let width_available_for_bricks =
        total_width_of_bricks - (BRICK_COLUMNS - 1) as f32 * BRICK_SPACING_X;
    let brick_width = (width_available_for_bricks / BRICK_COLUMNS as f32);

    let total_height_of_bricks = PLAY_AREA_BOUNDS.y / 2.0 - BRICK_Y_OFFSET - 2. * BRICK_SPACING_X;
    let height_available_for_bricks =
        total_height_of_bricks - (BRICK_ROWS - 1) as f32 * BRICK_SPACING_Y;
    let brick_height = (height_available_for_bricks / BRICK_COLUMNS as f32);

    // Negative scales result in flipped sprites / meshes,
    // which is definitely not what we want here
    assert!(brick_width > 0.0);
    assert!(brick_height > 0.0);

    // Center the bricks and move them up a bit
    let brick_offset = Vec2::new(-(total_width_of_bricks - brick_width) / 2.0, BRICK_Y_OFFSET);

    for row in 0..BRICK_ROWS {
        for column in 0..BRICK_COLUMNS {
            let brick_position = Vec2::new(
                column as f32 * (brick_width + BRICK_SPACING_X),
                row as f32 * (brick_height + BRICK_SPACING_X),
            ) + brick_offset;

            // brick
            commands
                .spawn()
                .insert(Brick)
                .insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: BRICK_COLOR,
                        ..default()
                    },
                    transform: Transform {
                        translation: brick_position.extend(0.0),
                        scale: Vec3::new(brick_width, brick_height, 1.0),
                        ..default()
                    },
                    ..default()
                })
                .insert(Collider);
        }
    }
}

fn move_paddle(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Paddle>>,
) {
    let mut paddle_transform = query.single_mut();
    let mut direction = 0.0;

    if keyboard_input.pressed(KeyCode::Left) {
        direction -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::Right) {
        direction += 1.0;
    }

    // This is the maximum x value that the paddle's transform can have without leaving the play area
    let paddle_bounds =
        (PLAY_AREA_BOUNDS.x - WALL_THICKNESS - PADDLE_SIZE.x) / 2.0 - PADDLE_PADDING;

    // Calculate the new horizontal paddle position based on player input
    let new_paddle_position = paddle_transform.translation.x + direction * PADDLE_SPEED * TIME_STEP;

    // Update the paddle position,
    // making sure it doesn't cause the paddle to leave the arena
    paddle_transform.translation.x = new_paddle_position.clamp(-paddle_bounds, paddle_bounds);
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation.x += velocity.0.x * TIME_STEP;
        transform.translation.y += velocity.0.y * TIME_STEP;
    }
}

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[1].value = format!("{}", scoreboard.score);
}

fn check_for_collisions(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut ball_query: Query<(&mut Velocity, &Transform), With<Ball>>,
    collider_query: Query<(Entity, &Transform, Option<&Brick>), With<Collider>>,
) {
    let (mut ball_velocity, ball_transform) = ball_query.single_mut();
    let ball_size = ball_transform.scale.truncate();

    // check collision with walls
    for (collider_entity, transform, maybe_brick) in collider_query.iter() {
        let collision = collide(
            ball_transform.translation,
            ball_size,
            transform.translation,
            transform.scale.truncate(),
        );
        if let Some(collision) = collision {
            // Bricks should be despawned and increment the scoreboard on collision
            if maybe_brick.is_some() {
                scoreboard.score += 1;
                commands.entity(collider_entity).despawn();
            }

            // reflect the ball when it collides
            let mut reflect_x = false;
            let mut reflect_y = false;

            // only reflect if the ball's velocity is going in the opposite direction of the
            // collision
            match collision {
                Collision::Left => reflect_x = ball_velocity.0.x > 0.0,
                Collision::Right => reflect_x = ball_velocity.0.x < 0.0,
                Collision::Top => reflect_y = ball_velocity.0.y < 0.0,
                Collision::Bottom => reflect_y = ball_velocity.0.y > 0.0,
                Collision::Inside => { /* do nothing */ }
            }

            // reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                ball_velocity.0.x = -ball_velocity.0.x;
            }

            // reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                ball_velocity.0.y = -ball_velocity.0.y;
            }
        }
    }
}
