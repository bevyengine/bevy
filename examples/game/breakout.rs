use bevy::{
    prelude::*,
    render::pass::ClearColor,
    sprite::collide_aabb::{collide, Collision},
};

/// An implementation of the classic game "Breakout"
fn main() {
    App::build()
        .add_default_plugins()
        .add_resource(Scoreboard { score: 0 })
        .add_resource(ClearColor(Color::rgb(0.7, 0.7, 0.7)))
        .add_startup_system(setup.system())
        .add_system(paddle_movement_system.system())
        .add_system(ball_collision_system.system())
        .add_system(ball_movement_system.system())
        .add_system(scoreboard_system.system())
        .run();
}

struct Paddle {
    speed: f32,
}

struct Ball {
    velocity: Vec3,
}

struct Scoreboard {
    score: usize,
}

enum Collider {
    Solid,
    Scorable,
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Add the game's entities to our world
    commands
        // cameras
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default())
        // paddle
        .spawn(SpriteComponents {
            material: materials.add(Color::rgb(0.2, 0.2, 0.8).into()),
            transform: Transform::from_translation(Vec3::new(0.0, -215.0, 0.0)),
            sprite: Sprite::new(Vec2::new(120.0, 30.0)),
            ..Default::default()
        })
        .with(Paddle { speed: 500.0 })
        .with(Collider::Solid)
        // ball
        .spawn(SpriteComponents {
            material: materials.add(Color::rgb(0.8, 0.2, 0.2).into()),
            transform: Transform::from_translation(Vec3::new(0.0, -50.0, 1.0)),
            sprite: Sprite::new(Vec2::new(30.0, 30.0)),
            ..Default::default()
        })
        .with(Ball {
            velocity: 400.0 * Vec3::new(0.5, -0.5, 0.0).normalize(),
        })
        // scoreboard
        .spawn(TextComponents {
            text: Text {
                font: asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap(),
                value: "Score:".to_string(),
                style: TextStyle {
                    color: Color::rgb(0.2, 0.2, 0.8),
                    font_size: 40.0,
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });

    // Add walls
    let wall_material = materials.add(Color::rgb(0.5, 0.5, 0.5).into());
    let wall_thickness = 10.0;
    let bounds = Vec2::new(900.0, 600.0);

    commands
        // left
        .spawn(SpriteComponents {
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(-bounds.x() / 2.0, 0.0, 0.0)),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y() + wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid)
        // right
        .spawn(SpriteComponents {
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(bounds.x() / 2.0, 0.0, 0.0)),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y() + wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid)
        // bottom
        .spawn(SpriteComponents {
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(0.0, -bounds.y() / 2.0, 0.0)),
            sprite: Sprite::new(Vec2::new(bounds.x() + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid)
        // top
        .spawn(SpriteComponents {
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(0.0, bounds.y() / 2.0, 0.0)),
            sprite: Sprite::new(Vec2::new(bounds.x() + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid);

    // Add bricks
    let brick_rows = 4;
    let brick_columns = 5;
    let brick_spacing = 20.0;
    let brick_size = Vec2::new(150.0, 30.0);
    let bricks_width = brick_columns as f32 * (brick_size.x() + brick_spacing) - brick_spacing;
    // center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(bricks_width - brick_size.x()) / 2.0, 100.0, 0.0);

    for row in 0..brick_rows {
        let y_position = row as f32 * (brick_size.y() + brick_spacing);
        for column in 0..brick_columns {
            let brick_position = Vec3::new(
                column as f32 * (brick_size.x() + brick_spacing),
                y_position,
                0.0,
            ) + bricks_offset;
            commands
                // brick
                .spawn(SpriteComponents {
                    material: materials.add(Color::rgb(0.2, 0.2, 0.8).into()),
                    sprite: Sprite::new(brick_size),
                    transform: Transform::from_translation(brick_position),
                    ..Default::default()
                })
                .with(Collider::Scorable);
        }
    }
}

fn paddle_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform)>,
) {
    for (paddle, mut transform) in &mut query.iter() {
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Left) {
            direction -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::Right) {
            direction += 1.0;
        }

        let translation = transform.translation_mut();
        // move the paddle horizontally
        *translation.x_mut() += time.delta_seconds * direction * paddle.speed;
        // bound the paddle within the walls
        *translation.x_mut() = translation.x().min(380.0).max(-380.0);
    }
}

fn ball_movement_system(time: Res<Time>, mut ball_query: Query<(&Ball, &mut Transform)>) {
    // clamp the timestep to stop the ball from escaping when the game starts
    let delta_seconds = f32::min(0.2, time.delta_seconds);

    for (ball, mut transform) in &mut ball_query.iter() {
        transform.translate(ball.velocity * delta_seconds);
    }
}

fn scoreboard_system(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    for mut text in &mut query.iter() {
        text.value = format!("Score: {}", scoreboard.score);
    }
}

fn ball_collision_system(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut ball_query: Query<(&mut Ball, &Transform, &Sprite)>,
    mut collider_query: Query<(Entity, &Collider, &Transform, &Sprite)>,
) {
    for (mut ball, ball_transform, sprite) in &mut ball_query.iter() {
        let ball_size = sprite.size;
        let velocity = &mut ball.velocity;

        // check collision with walls
        for (collider_entity, collider, transform, sprite) in &mut collider_query.iter() {
            let collision = collide(
                ball_transform.translation(),
                ball_size,
                transform.translation(),
                sprite.size,
            );
            if let Some(collision) = collision {
                // scorable colliders should be despawned and increment the scoreboard on collision
                if let Collider::Scorable = *collider {
                    scoreboard.score += 1;
                    commands.despawn(collider_entity);
                }

                // reflect the ball when it collides
                let mut reflect_x = false;
                let mut reflect_y = false;

                // only reflect if the ball's velocity is going in the opposite direction of the collision
                match collision {
                    Collision::Left => reflect_x = velocity.x() > 0.0,
                    Collision::Right => reflect_x = velocity.x() < 0.0,
                    Collision::Top => reflect_y = velocity.y() < 0.0,
                    Collision::Bottom => reflect_y = velocity.y() > 0.0,
                }

                // reflect velocity on the x-axis if we hit something on the x-axis
                if reflect_x {
                    *velocity.x_mut() = -velocity.x();
                }

                // reflect velocity on the y-axis if we hit something on the y-axis
                if reflect_y {
                    *velocity.y_mut() = -velocity.y();
                }

                break;
            }
        }
    }
}
