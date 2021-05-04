use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::pass::ClearColor,
    sprite::collide_aabb::{collide, Collision},
};

use components::*;
use resources::*;

/// Constants that can be used to fine-tune the behavior of our game
mod config {
    use bevy::render::color::Color;
    use bevy::ui::Val;
    // TODO: add various Vec2's and Transforms to this config module for clarity and consistency
    // Blocked on https://github.com/bitshifter/glam-rs/issues/76

    pub const TIME_STEP: f32 = 1.0 / 60.0;
    pub const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

    pub const PADDLE_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
    pub const PADDLE_SPEED: f32 = 500.0;

    pub const BALL_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

    pub const WALL_THICKNESS: f32 = 10.0;
    pub const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);

    pub const BRICK_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);

    pub const SCOREBOARD_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
    pub const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
    pub const SCORE_FONT_SIZE: f32 = 40.0;
    pub const SCORE_PADDING: Val = Val::Px(5.0);
}

/// A simple implementation of the classic game "Breakout"
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(config::BACKGROUND_COLOR))
        // This adds the Score resource with its default values: 0
        .init_resource::<Score>()
        // These systems run only once, before all other systems
        .add_startup_system(spawn_cameras.system())
        .add_startup_system(spawn_paddle.system())
        .add_startup_system(spawn_ball.system())
        .add_startup_system(spawn_walls.system())
        .add_startup_system(spawn_bricks.system())
        .add_startup_system(spawn_scoreboard.system())
        // These systems run repeatedly, whnever the FixedTimeStep's duration has elapsed
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(config::TIME_STEP as f64))
                .with_system(kinematics.system().label("kinematics"))
                .with_system(ball_collision.system().before("kinematics")),
        )
        // Ordinary systems run every frame
        .add_system(bound_paddle.system().label("bound_paddle"))
        .add_system(paddle_input.system().after("bound_paddle"))
        .add_system(update_scoreboard.system())
        .run();
}

mod resources {
    #[derive(Default)]
    pub struct Score(pub usize);
}

mod components {
    pub struct Paddle {
        pub speed: f32,
    }
    // These are data-less marker components
    // which let us query for the correct entities
    // and specialize behavior
    pub struct Ball;
    pub struct Brick;
    pub struct Scoreboard;
    pub struct Collides;

    // The derived default values of numeric fields in Rust are zero
    #[derive(Default)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }
}

fn spawn_cameras(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

fn spawn_paddle(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let paddle_starting_location: Transform = Transform::from_xyz(0.0, -215.0, 0.0);
    let paddle_size: Vec2 = Vec2::new(120.0, 30.0);

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(config::PADDLE_COLOR.into()),
            transform: paddle_starting_location,
            sprite: Sprite::new(paddle_size),
            ..Default::default()
        })
        .insert(Paddle {
            speed: config::PADDLE_SPEED,
        })
        .insert(Collides)
        .insert(Velocity::default());
}

fn spawn_ball(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    // We set the z-value to one to ensure it appears on top of our other objects in case of overlap
    let ball_starting_location: Transform = Transform::from_xyz(0.0, -50.0, 1.0);
    // Our ball is actually a square. Shhh...
    let ball_size: Vec2 = Vec2::new(30.0, 30.0);

    let ball_starting_direction: Vec2 = Vec2::new(0.5, -0.5).normalize();
    let ball_starting_speed: f32 = 400.0;
    let ball_starting_velocity: Velocity = Velocity {
        x: ball_starting_direction.x * ball_starting_speed,
        y: ball_starting_direction.y * ball_starting_speed,
    };

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(config::BALL_COLOR.into()),
            transform: ball_starting_location,
            sprite: Sprite::new(ball_size),
            ..Default::default()
        })
        .insert(Ball)
        .insert(Collides)
        // Adds a `Velocity` component with the value defined in the `config` module
        .insert(ball_starting_velocity);
}

/// Defines which side of the arena a wall is part of
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    fn wall_coord(&self, bounds: Vec2) -> Transform {
        let (x, y) = match self {
            Side::Top => (0.0, bounds.y / 2.0),
            Side::Bottom => (0.0, -bounds.y / 2.0),
            Side::Left => (-bounds.x / 2.0, 0.0),
            Side::Right => (bounds.x / 2.0, 0.0),
        };
        // We need to convert these coordinates into a 3D transform to add to our SpriteBundle
        Transform::from_xyz(x, y, 0.0)
    }

    fn wall_size(&self, bounds: Vec2, thickness: f32) -> Vec2 {
        match self {
            Side::Top => Vec2::new(bounds.x + thickness, thickness),
            Side::Bottom => Vec2::new(bounds.x + thickness, thickness),
            Side::Left => Vec2::new(thickness, bounds.y + thickness),
            Side::Right => Vec2::new(thickness, bounds.y + thickness),
        }
    }
}

// By creating our own bundles, we can avoid duplicating code
#[derive(Bundle)]
struct WallBundle {
    // Use #[bundle] like this to nest bundles correctly
    #[bundle]
    sprite_bundle: SpriteBundle,
    collides: Collides,
}

impl WallBundle {
    fn new(side: Side, material_handle: &Handle<ColorMaterial>) -> Self {
        let arena_bounds: Vec2 = Vec2::new(900.0, 600.0);

        let bounds = arena_bounds;
        let thickness = config::WALL_THICKNESS;

        WallBundle {
            sprite_bundle: SpriteBundle {
                material: material_handle.clone(),
                transform: side.wall_coord(bounds),
                sprite: Sprite::new(side.wall_size(bounds, thickness)),
                ..Default::default()
            },
            collides: Collides,
        }
    }
}

fn spawn_walls(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let material_handle = materials.add(config::WALL_COLOR.into());

    commands.spawn_bundle(WallBundle::new(Side::Top, &material_handle));
    commands.spawn_bundle(WallBundle::new(Side::Bottom, &material_handle));
    commands.spawn_bundle(WallBundle::new(Side::Left, &material_handle));
    commands.spawn_bundle(WallBundle::new(Side::Right, &material_handle));
}

fn spawn_bricks(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let brick_material = materials.add(config::BRICK_COLOR.into());

    // Brick layout constants
    const BRICK_ROWS: i8 = 4;
    const BRICK_COLUMNS: i8 = 5;
    const BRICK_SPACING: f32 = 20.0;
    // TODO: change to const when https://github.com/bitshifter/glam-rs/issues/76 is fixed
    let brick_size: Vec2 = Vec2::new(150.0, 30.0);

    // Compute the total width that all of the bricks take
    let total_width = BRICK_COLUMNS as f32 * (brick_size.x + BRICK_SPACING) - BRICK_SPACING;
    // Center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(total_width - brick_size.x) / 2.0, 100.0, 0.0);

    // Add the bricks
    for row in 0..BRICK_ROWS {
        for column in 0..BRICK_COLUMNS {
            let brick_position = Vec3::new(
                column as f32 * (brick_size.x + BRICK_SPACING),
                row as f32 * (brick_size.y + BRICK_SPACING),
                0.0,
            ) + bricks_offset;
            // Adding one brick at a time
            commands
                .spawn_bundle(SpriteBundle {
                    material: brick_material.clone(),
                    sprite: Sprite::new(brick_size),
                    transform: Transform::from_translation(brick_position),
                    ..Default::default()
                })
                .insert(Brick)
                .insert(Collides);
        }
    }
}

fn spawn_scoreboard(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(TextBundle {
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Score: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: config::SCORE_FONT_SIZE,
                            color: config::SCOREBOARD_COLOR,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                            font_size: config::SCORE_FONT_SIZE,
                            color: config::SCORE_COLOR,
                        },
                    },
                ],
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: config::SCORE_PADDING,
                    left: config::SCORE_PADDING,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Scoreboard);
}

/// Moves everything with both a Transform and a Velovity accordingly
fn kinematics(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation.x += velocity.x * config::TIME_STEP;
        transform.translation.y += velocity.y * config::TIME_STEP;
    }
}

/// Turns left and right arrow key inputs to set paddle velocity
fn paddle_input(keyboard_input: Res<Input<KeyCode>>, mut query: Query<(&Paddle, &mut Velocity)>) {
    let (paddle, mut velocity) = query.single_mut().unwrap();

    let mut direction = 0.0;
    if keyboard_input.pressed(KeyCode::Left) {
        direction -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::Right) {
        direction += 1.0;
    }

    velocity.x = direction * paddle.speed;
}

/// Ensures our paddle never goes out of bounds
fn bound_paddle(mut query: Query<(&mut Transform, &mut Velocity), With<Paddle>>) {
    const BOUND: f32 = 380.0;
    let (mut paddle_transform, mut paddle_velocity) = query.single_mut().unwrap();

    if paddle_transform.translation.x >= BOUND {
        paddle_transform.translation.x = BOUND;
        paddle_velocity.x = 0.0;
    } else if paddle_transform.translation.x <= -BOUND {
        paddle_transform.translation.x = -BOUND;
        paddle_velocity.x = 0.0;
    }
}

/// Detects and handles ball collisions
fn ball_collision(
    mut ball_query: Query<(&Transform, &mut Velocity, &Sprite), With<Ball>>,
    // Option<&C> returns Some(c: C) if the component exists on the entity, and None if it does not
    collider_query: Query<
        (Entity, &Transform, &Sprite, Option<&Brick>),
        (With<Collides>, Without<Ball>),
    >,
    mut commands: Commands,
    mut score: ResMut<Score>,
) {
    let (ball_transform, mut ball_velocity, ball_sprite) = ball_query.single_mut().unwrap();
    let ball_size = ball_sprite.size;

    for (collider_entity, collider_transform, collider_sprite, maybe_brick) in collider_query.iter()
    {
        // Check for collisions
        let collider_size = collider_sprite.size;
        let potential_collision = collide(
            ball_transform.translation,
            ball_size,
            collider_transform.translation,
            collider_size,
        );

        // Handle collisions
        if let Some(collision) = potential_collision {
            // Reflect the ball when it collides
            let mut reflect_x = false;
            let mut reflect_y = false;

            // Only reflect if the ball's velocity is going
            // in the opposite direction of the collision
            match collision {
                Collision::Left => reflect_x = ball_velocity.x > 0.0,
                Collision::Right => reflect_x = ball_velocity.x < 0.0,
                Collision::Top => reflect_y = ball_velocity.y < 0.0,
                Collision::Bottom => reflect_y = ball_velocity.y > 0.0,
            }

            // Reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                ball_velocity.x = -ball_velocity.x;
            }

            // Reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                ball_velocity.y = -ball_velocity.y;
            }

            // Perform special brick collision behavior
            if maybe_brick.is_some() {
                // Despawn bricks that are hit
                commands.entity(collider_entity).despawn();

                // Increase the score by 1 for each brick hit
                score.0 += 1;
            }
        }
    }
}

/// Updates the Scoreboard entity's Text based on the value of the Score resource
fn update_scoreboard(score: Res<Score>, mut query: Query<&mut Text, With<Scoreboard>>) {
    let mut scoreboard_text = query.single_mut().unwrap();
    scoreboard_text.sections[1].value = format!("{}", score.0);
}
