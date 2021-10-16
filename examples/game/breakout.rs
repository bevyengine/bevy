use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::{camera::Camera, pass::ClearColor},
    sprite::collide_aabb::{collide, Collision},
};
use rand::Rng;

/// An implementation of the classic game "Breakout"
const TIME_STEP: f32 = 1.0 / 60.0;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(Color::rgb(0.9, 0.9, 0.9)))
        .add_state(GameState::MainMenu)
        .add_event::<GameOverEvent>()
        .add_startup_system(setup_cameras)
        .add_system_set(SystemSet::on_enter(GameState::MainMenu).with_system(ui_system_setup))
        .add_system_set(SystemSet::on_update(GameState::MainMenu).with_system(key_input_system))
        .add_system_set(SystemSet::on_exit(GameState::MainMenu).with_system(teardown))
        .add_system_set(SystemSet::on_enter(GameState::InGame).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::InGame)
                .with_system(paddle_movement_system)
                .with_system(ball_collision_system)
                .with_system(ball_movement_system)
                .with_system(scoreboard_system)
                .with_system(on_game_over),
        )
        .add_system_set(SystemSet::on_exit(GameState::InGame).with_system(teardown))
        .add_system_set(SystemSet::on_enter(GameState::GameOver).with_system(ui_system_setup))
        .add_system_set(SystemSet::on_update(GameState::GameOver).with_system(key_input_system))
        .add_system_set(SystemSet::on_exit(GameState::GameOver).with_system(teardown))
        .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
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

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum GameState {
    InGame,
    GameOver,
    MainMenu,
}

struct GameOverEvent(usize);

fn setup_cameras(mut commands: Commands) {
    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Add the game's entities to our world
    // paddle
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            transform: Transform::from_xyz(0.0, -215.0, 0.0),
            sprite: Sprite::new(Vec2::new(120.0, 30.0)),
            ..Default::default()
        })
        .insert(Paddle { speed: 500.0 })
        .insert(Collider::Paddle);
    // ball
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
            transform: Transform::from_xyz(0.0, -50.0, 1.0),
            sprite: Sprite::new(Vec2::new(30.0, 30.0)),
            ..Default::default()
        })
        .insert(Ball {
            velocity: 400.0 * Vec3::new(0.5, -0.5, 0.0).normalize(),
        });
    // scoreboard
    commands.spawn_bundle(TextBundle {
        text: Text {
            sections: vec![
                TextSection {
                    value: "Score: ".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.5, 0.5, 1.0),
                    },
                },
                TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(1.0, 0.5, 0.5),
                    },
                },
            ],
            ..Default::default()
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
    let wall_material = materials.add(Color::rgb(0.8, 0.8, 0.8).into());
    let wall_thickness = 10.0;
    let bounds = Vec2::new(900.0, 600.0);

    // left
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_xyz(-bounds.x / 2.0, 0.0, 0.0),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y + wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);
    // right
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_xyz(bounds.x / 2.0, 0.0, 0.0),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y + wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);
    // bottom
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_xyz(0.0, -bounds.y / 2.0, 0.0),
            sprite: Sprite::new(Vec2::new(bounds.x + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);
    // top
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material,
            transform: Transform::from_xyz(0.0, bounds.y / 2.0, 0.0),
            sprite: Sprite::new(Vec2::new(bounds.x + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);

    // Add bricks
    let brick_rows = 4;
    let brick_columns = 5;
    let brick_spacing = 20.0;
    let brick_size = Vec2::new(150.0, 30.0);
    let bricks_width = brick_columns as f32 * (brick_size.x + brick_spacing) - brick_spacing;
    let mut rng = rand::thread_rng();
    // center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(bricks_width - brick_size.x) / 2.0, 100.0, 0.0);
    for row in 0..brick_rows {
        let y_position = row as f32 * (brick_size.y + brick_spacing);
        for column in 0..brick_columns {
            let brick_position = Vec3::new(
                column as f32 * (brick_size.x + brick_spacing),
                y_position,
                0.0,
            ) + bricks_offset;
            // brick
            let brick_material = materials.add(
                Color::rgb(
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                )
                .into(),
            );
            commands
                .spawn_bundle(SpriteBundle {
                    material: brick_material,
                    sprite: Sprite::new(brick_size),
                    transform: Transform::from_translation(brick_position),
                    ..Default::default()
                })
                .insert(Collider::Scorable);
        }
    }
}

// remove all entities that are not a camera
fn teardown(mut commands: Commands, entities: Query<Entity, Without<Camera>>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
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
    translation.x = translation.x.min(380.0).max(-380.0);
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
    mut ball_query: Query<(&mut Ball, &Transform, &Sprite)>,
    collider_query: Query<(Entity, &Collider, &Transform, &Sprite)>,
    mut ev_gameover: EventWriter<GameOverEvent>,
) {
    let (mut ball, ball_transform, sprite) = ball_query.single_mut();
    let ball_size = sprite.size;
    let velocity = &mut ball.velocity;

    // check collision with walls
    for (collider_entity, collider, transform, sprite) in collider_query.iter() {
        let collision = collide(
            ball_transform.translation,
            ball_size,
            transform.translation,
            sprite.size,
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
            }

            // reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                velocity.x = -velocity.x;
            }

            // reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                velocity.y = -velocity.y;
                if let Collider::Solid = collider {
                    ev_gameover.send(GameOverEvent(scoreboard.score));
                }
            }

            // break if this collide is on a solid, otherwise continue check whether a solid is
            // also in collision
            if let Collider::Solid = *collider {
                break;
            }
        }
    }
}

fn on_game_over(
    mut ev_gameover: EventReader<GameOverEvent>,
    mut app_state: ResMut<State<GameState>>,
) {
    if ev_gameover.iter().next().is_some() {
        app_state.set(GameState::GameOver).unwrap();
    }
}

fn ui_system_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scoreboard: ResMut<Scoreboard>,
    app_state: Res<State<GameState>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut menu_content = "".to_string();
    if let GameState::GameOver = app_state.current() {
        menu_content = format!("Game Over\nYour Score: {}", scoreboard.score);
        scoreboard.score = 0;
    } else if let GameState::MainMenu = app_state.current() {
        menu_content = "Main Menu".to_string();
    }

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,

                margin: Rect::all(Val::Auto),

                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Center,

                justify_content: JustifyContent::Center,

                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![
                        TextSection {
                            value: menu_content,
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::CRIMSON,
                            },
                        },
                        TextSection {
                            value: "\n[SPC] to play\n[ESC] to exit".to_string(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.5, 0.5, 1.0),
                            },
                        },
                    ],
                    alignment: TextAlignment {
                        horizontal: HorizontalAlign::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
        });
}

fn key_input_system(mut keys: ResMut<Input<KeyCode>>, mut app_state: ResMut<State<GameState>>) {
    if keys.just_pressed(KeyCode::Space) {
        app_state.set(GameState::InGame).unwrap();
        keys.reset(KeyCode::Space);
    }
}
