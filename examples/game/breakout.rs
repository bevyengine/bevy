use bevy::{
    core::FixedTimestep,
    input::system::exit_on_esc_system,
    prelude::*,
    render::pass::ClearColor,
    sprite::collide_aabb::{collide, Collision},
};

use components::*;
use config::*;
use resources::*;

/// Constants that can be used to fine-tune the behavior of our game
mod config {
    use bevy::math::{const_quat, const_vec2, const_vec3, Vec2};
    use bevy::render::color::Color;
    use bevy::transform::components::Transform;
    use bevy::ui::Val;

    pub const TIME_STEP: f32 = 1.0 / 60.0;
    pub const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

    pub const PADDLE_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
    pub const PADDLE_SPEED: f32 = 500.0;
    pub const PADDLE_SIZE: Vec2 = const_vec2!([120.0, 30.0]);
    pub const PADDLE_BOUND: f32 = 380.0;
    pub const PADDLE_STARTING_TRANSFORM: Transform = Transform {
        translation: const_vec3!([0.0, -215.0, 0.0]),
        // We don't want any rotation
        rotation: const_quat!([0.0, 0.0, 0.0, 0.0]),
        // We want the scale to be 1 in all directions
        scale: const_vec3!([1.0, 1.0, 1.0]),
    };

    pub const BALL_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
    // Our ball is actually a square. Shhh...
    pub const BALL_SIZE: Vec2 = const_vec2!([30.0, 30.0]);
    pub const BALL_STARTING_DIRECTION: Vec2 = const_vec2!([0.5, -0.5]);
    pub const BALL_STARTING_SPEED: f32 = 400.0;
    // We set the z-value to one to ensure it appears on top of our other objects in case of overlap
    pub const BALL_STARTING_TRANSFORM: Transform = Transform {
        translation: const_vec3!([0.0, -50.0, 1.0]),
        rotation: const_quat!([0.0, 0.0, 0.0, 0.0]),
        scale: const_vec3!([1.0, 1.0, 1.0]),
    };

    pub const ARENA_BOUNDS: Vec2 = const_vec2!([900.0, 600.0]);
    pub const WALL_THICKNESS: f32 = 10.0;
    pub const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);

    pub const BRICK_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
    pub const BRICK_WIDTH: f32 = 150.0;
    pub const BRICK_HEIGHT: f32 = 30.0;
    pub const BRICK_ROWS: i8 = 4;
    pub const BRICK_COLUMNS: i8 = 5;
    pub const BRICK_SPACING: f32 = 20.0;

    pub const SCOREBOARD_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
    pub const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
    pub const SCORE_FONT_SIZE: f32 = 40.0;
    pub const SCORE_PADDING: Val = Val::Px(5.0);
    pub const SCOREBOARD_FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";
    pub const SCORE_FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";
}

/// A simple implementation of the classic game "Breakout"
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        // This adds the Score resource with its default value of 0
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
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(kinematics.system().label("kinematics"))
                // We need to check for collisions before handling movement
                // to reduce the risk of the ball passing through objects
                .with_system(ball_collision.system().before("kinematics")),
        )
        // Ordinary systems run every frame
        // We need to handle input before we move our paddle,
        // to ensure that we're responding to the most recent frame's events,
        // avoiding input lag
        // See https://github.com/bevyengine/bevy/blob/latest/examples/ecs/ecs_guide.rs
        // for more information on system ordering
        .add_system(
            paddle_input
                .system()
                .before("bound_paddle")
                .before("kinematics"),
        )
        .add_system(
            bound_paddle
                .system()
                .label("bound_paddle")
                // This system must run after kinematics, or the velocity will be set to 0
                // before the paddle moves, causing it to be stuck to the wall
                .after("kinematics"),
        )
        .add_system(update_scoreboard.system())
        // Exits the game when `KeyCode::Esc` is pressed
        // This is a simple built-in system
        .add_system(exit_on_esc_system.system())
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
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(PADDLE_COLOR.into()),
            transform: PADDLE_STARTING_TRANSFORM,
            sprite: Sprite::new(PADDLE_SIZE),
            ..Default::default()
        })
        .insert(Paddle {
            speed: PADDLE_SPEED,
        })
        .insert(Collides)
        .insert(Velocity::default());
}

fn spawn_ball(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    // .normalize is not a const fn, so we have to perform this operation at runtime
    // FIXME: Blocked on https://github.com/bitshifter/glam-rs/issues/76
    let normalized_direction = BALL_STARTING_DIRECTION.normalize();
    let ball_starting_velocity: Velocity = Velocity {
        x: normalized_direction.x * BALL_STARTING_SPEED,
        y: normalized_direction.y * BALL_STARTING_SPEED,
    };

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(BALL_COLOR.into()),
            transform: BALL_STARTING_TRANSFORM,
            sprite: Sprite::new(BALL_SIZE),
            ..Default::default()
        })
        .insert(Ball)
        .insert(Collides)
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
    fn new(side: Side, material_handle: Handle<ColorMaterial>) -> Self {
        WallBundle {
            sprite_bundle: SpriteBundle {
                material: material_handle,
                transform: side.wall_coord(ARENA_BOUNDS),
                sprite: Sprite::new(side.wall_size(ARENA_BOUNDS, WALL_THICKNESS)),
                ..Default::default()
            },
            collides: Collides,
        }
    }
}

fn spawn_walls(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let material_handle = materials.add(WALL_COLOR.into());

    // Each material handle must be uniquely owned as handles are ref-counted
    commands.spawn_bundle(WallBundle::new(Side::Top, material_handle.clone()));
    commands.spawn_bundle(WallBundle::new(Side::Bottom, material_handle.clone()));
    commands.spawn_bundle(WallBundle::new(Side::Left, material_handle.clone()));
    commands.spawn_bundle(WallBundle::new(Side::Right, material_handle));
}

#[derive(Bundle)]
struct BrickBundle {
    #[bundle]
    sprite_bundle: SpriteBundle,
    brick: Brick,
    collides: Collides,
}

impl BrickBundle {
    fn new(x: f32, y: f32, material_handle: Handle<ColorMaterial>) -> Self {
        BrickBundle {
            sprite_bundle: SpriteBundle {
                material: material_handle,
                transform: Transform::from_xyz(x, y, 0.0),
                sprite: Sprite::new(Vec2::new(BRICK_WIDTH, BRICK_HEIGHT)),
                ..Default::default()
            },
            brick: Brick,
            collides: Collides,
        }
    }
}

fn spawn_bricks(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let brick_material = materials.add(BRICK_COLOR.into());

    // Compute the total width that all of the bricks take
    const TOTAL_WIDTH: f32 = BRICK_COLUMNS as f32 * (BRICK_WIDTH + BRICK_SPACING) - BRICK_SPACING;
    // Center the bricks
    const OFFSET_X: f32 = -(TOTAL_WIDTH - BRICK_WIDTH) / 2.0;
    // Move the bricks up slightly
    const OFFSET_Y: f32 = 100.0;

    // Add the bricks
    let brick_iterator = (0..BRICK_ROWS)
        .flat_map(|row| (0..BRICK_COLUMNS).map(move |col| (row, col)))
        .map(move |(row, column)| {
            BrickBundle::new(
                column as f32 * (BRICK_WIDTH + BRICK_SPACING) + OFFSET_X,
                row as f32 * (BRICK_HEIGHT + BRICK_SPACING) + OFFSET_Y,
                brick_material.clone(),
            )
        });
    // spawn_batch is slightly more efficient than repeatedly calling .spawn_bundle due to memory pre-allocation
    // This approach is overkill for the small number of entities here, but serves to demonstrate how the function is used
    commands.spawn_batch(brick_iterator);

    /* Equivalently, you could spawn one brick at a time using for loops instead, at a small cost to performance
    for row in 0..BRICK_ROWS {
        for column in 0..BRICK_COLUMNS {
            commands.spawn_bundle(BrickBundle::new(
                column as f32 * (BRICK_WIDTH + BRICK_SPACING) + OFFSET_X,
                row as f32 * (BRICK_HEIGHT + BRICK_SPACING) + OFFSET_Y,
                &brick_material,
            ));
        }
    }
    */
}

fn spawn_scoreboard(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(TextBundle {
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Score: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load(SCOREBOARD_FONT_PATH),
                            font_size: SCORE_FONT_SIZE,
                            color: SCOREBOARD_COLOR,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load(SCORE_FONT_PATH),
                            font_size: SCORE_FONT_SIZE,
                            color: SCORE_COLOR,
                        },
                    },
                ],
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: SCORE_PADDING,
                    left: SCORE_PADDING,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Scoreboard);
}

/// Moves everything with both a Transform and a Velocity accordingly
fn kinematics(mut query: Query<(&mut Transform, &Velocity)>) {
    query.for_each_mut(|(mut transform, velocity)| {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
    });
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
    let (mut paddle_transform, mut paddle_velocity) = query.single_mut().unwrap();

    if paddle_transform.translation.x >= PADDLE_BOUND {
        paddle_transform.translation.x = PADDLE_BOUND;
        paddle_velocity.x = 0.0;
    } else if paddle_transform.translation.x <= -PADDLE_BOUND {
        paddle_transform.translation.x = -PADDLE_BOUND;
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

    collider_query.for_each(
        |(collider_entity, collider_transform, collider_sprite, maybe_brick)| {
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
        },
    );
}

/// Updates the Scoreboard entity's Text based on the value of the Score resource
fn update_scoreboard(score: Res<Score>, mut query: Query<&mut Text, With<Scoreboard>>) {
    let mut scoreboard_text = query.single_mut().unwrap();
    // We need to access the second section, so we need to access the sections field at the [1] index
    // (Rust is 0-indexed: https://medium.com/analytics-vidhya/array-indexing-0-based-or-1-based-dd89d631d11c)
    scoreboard_text.sections[1].value = format!("{}", score.0);
}
