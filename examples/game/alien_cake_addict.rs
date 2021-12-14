use bevy::{
    core::FixedTimestep, ecs::schedule::SystemSet, prelude::*, render::camera::CameraPlugin,
};
use rand::Rng;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Playing,
    GameOver,
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .init_resource::<Game>()
        .add_plugins(DefaultPlugins)
        .add_state(GameState::Playing)
        .add_startup_system(setup_cameras)
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(move_player)
                .with_system(focus_camera)
                .with_system(rotate_bonus)
                .with_system(scoreboard_system),
        )
        .add_system_set(SystemSet::on_exit(GameState::Playing).with_system(teardown))
        .add_system_set(SystemSet::on_enter(GameState::GameOver).with_system(display_score))
        .add_system_set(SystemSet::on_update(GameState::GameOver).with_system(gameover_keyboard))
        .add_system_set(SystemSet::on_exit(GameState::GameOver).with_system(teardown))
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(5.0))
                .with_system(spawn_bonus),
        )
        .add_system(bevy::input::system::exit_on_esc_system)
        .run();
}

struct Cell {
    height: f32,
}

#[derive(Default)]
struct Player {
    entity: Option<Entity>,
    i: usize,
    j: usize,
}

#[derive(Default)]
struct Bonus {
    entity: Option<Entity>,
    i: usize,
    j: usize,
    handle: Handle<Scene>,
}

#[derive(Default)]
struct Game {
    board: Vec<Vec<Cell>>,
    player: Player,
    bonus: Bonus,
    score: i32,
    cake_eaten: u32,
    camera_should_focus: Vec3,
    camera_is_focus: Vec3,
}

const BOARD_SIZE_I: usize = 14;
const BOARD_SIZE_J: usize = 21;

const RESET_FOCUS: [f32; 3] = [
    BOARD_SIZE_I as f32 / 2.0,
    0.0,
    BOARD_SIZE_J as f32 / 2.0 - 0.5,
];

fn setup_cameras(mut commands: Commands, mut game: ResMut<Game>) {
    game.camera_should_focus = Vec3::from(RESET_FOCUS);
    game.camera_is_focus = game.camera_should_focus;
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(
            -(BOARD_SIZE_I as f32 / 2.0),
            2.0 * BOARD_SIZE_J as f32 / 3.0,
            BOARD_SIZE_J as f32 / 2.0 - 0.5,
        )
        .looking_at(game.camera_is_focus, Vec3::Y),
        ..Default::default()
    });
    commands.spawn_bundle(UiCameraBundle::default());
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut game: ResMut<Game>) {
    // reset the game state
    game.cake_eaten = 0;
    game.score = 0;
    game.player.i = BOARD_SIZE_I / 2;
    game.player.j = BOARD_SIZE_J / 2;

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 10.0, 4.0),
        point_light: PointLight {
            intensity: 3000.0,
            shadows_enabled: true,
            range: 30.0,
            ..Default::default()
        },
        ..Default::default()
    });

    // spawn the game board
    let cell_scene = asset_server.load("models/AlienCake/tile.glb#Scene0");
    game.board = (0..BOARD_SIZE_J)
        .map(|j| {
            (0..BOARD_SIZE_I)
                .map(|i| {
                    let height = rand::thread_rng().gen_range(-0.1..0.1);
                    commands
                        .spawn_bundle((
                            Transform::from_xyz(i as f32, height - 0.2, j as f32),
                            GlobalTransform::identity(),
                        ))
                        .with_children(|cell| {
                            cell.spawn_scene(cell_scene.clone());
                        });
                    Cell { height }
                })
                .collect()
        })
        .collect();

    // spawn the game character
    game.player.entity = Some(
        commands
            .spawn_bundle((
                Transform {
                    translation: Vec3::new(
                        game.player.i as f32,
                        game.board[game.player.j][game.player.i].height,
                        game.player.j as f32,
                    ),
                    rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                    ..Default::default()
                },
                GlobalTransform::identity(),
            ))
            .with_children(|cell| {
                cell.spawn_scene(asset_server.load("models/AlienCake/alien.glb#Scene0"));
            })
            .id(),
    );

    // load the scene for the cake
    game.bonus.handle = asset_server.load("models/AlienCake/cakeBirthday.glb#Scene0");

    // scoreboard
    commands.spawn_bundle(TextBundle {
        text: Text::with_section(
            "Score:",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            },
            Default::default(),
        ),
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
}

// remove all entities that are not a camera
fn teardown(mut commands: Commands, entities: Query<Entity, Without<Camera>>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

// control the game character
fn move_player(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut game: ResMut<Game>,
    mut transforms: Query<&mut Transform>,
) {
    let mut moved = false;
    let mut rotation = 0.0;
    if keyboard_input.just_pressed(KeyCode::Up) {
        if game.player.i < BOARD_SIZE_I - 1 {
            game.player.i += 1;
        }
        rotation = -std::f32::consts::FRAC_PI_2;
        moved = true;
    }
    if keyboard_input.just_pressed(KeyCode::Down) {
        if game.player.i > 0 {
            game.player.i -= 1;
        }
        rotation = std::f32::consts::FRAC_PI_2;
        moved = true;
    }
    if keyboard_input.just_pressed(KeyCode::Right) {
        if game.player.j < BOARD_SIZE_J - 1 {
            game.player.j += 1;
        }
        rotation = std::f32::consts::PI;
        moved = true;
    }
    if keyboard_input.just_pressed(KeyCode::Left) {
        if game.player.j > 0 {
            game.player.j -= 1;
        }
        rotation = 0.0;
        moved = true;
    }

    // move on the board
    if moved {
        *transforms.get_mut(game.player.entity.unwrap()).unwrap() = Transform {
            translation: Vec3::new(
                game.player.i as f32,
                game.board[game.player.j][game.player.i].height,
                game.player.j as f32,
            ),
            rotation: Quat::from_rotation_y(rotation),
            ..Default::default()
        };
    }

    // eat the cake!
    if let Some(entity) = game.bonus.entity {
        if game.player.i == game.bonus.i && game.player.j == game.bonus.j {
            game.score += 2;
            game.cake_eaten += 1;
            commands.entity(entity).despawn_recursive();
            game.bonus.entity = None;
        }
    }
}

// change the focus of the camera
fn focus_camera(
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut transforms: QuerySet<(
        QueryState<(&mut Transform, &Camera)>,
        QueryState<&Transform>,
    )>,
) {
    const SPEED: f32 = 2.0;
    // if there is both a player and a bonus, target the mid-point of them
    if let (Some(player_entity), Some(bonus_entity)) = (game.player.entity, game.bonus.entity) {
        let transform_query = transforms.q1();
        if let (Ok(player_transform), Ok(bonus_transform)) = (
            transform_query.get(player_entity),
            transform_query.get(bonus_entity),
        ) {
            game.camera_should_focus = player_transform
                .translation
                .lerp(bonus_transform.translation, 0.5);
        }
    // otherwise, if there is only a player, target the player
    } else if let Some(player_entity) = game.player.entity {
        if let Ok(player_transform) = transforms.q1().get(player_entity) {
            game.camera_should_focus = player_transform.translation;
        }
    // otherwise, target the middle
    } else {
        game.camera_should_focus = Vec3::from(RESET_FOCUS);
    }
    // calculate the camera motion based on the difference between where the camera is looking
    // and where it should be looking; the greater the distance, the faster the motion;
    // smooth out the camera movement using the frame time
    let mut camera_motion = game.camera_should_focus - game.camera_is_focus;
    if camera_motion.length() > 0.2 {
        camera_motion *= SPEED * time.delta_seconds();
        // set the new camera's actual focus
        game.camera_is_focus += camera_motion;
    }
    // look at that new camera's actual focus
    for (mut transform, camera) in transforms.q0().iter_mut() {
        if camera.name == Some(CameraPlugin::CAMERA_3D.to_string()) {
            *transform = transform.looking_at(game.camera_is_focus, Vec3::Y);
        }
    }
}

// despawn the bonus if there is one, then spawn a new one at a random location
fn spawn_bonus(
    mut state: ResMut<State<GameState>>,
    mut commands: Commands,
    mut game: ResMut<Game>,
) {
    if *state.current() != GameState::Playing {
        return;
    }
    if let Some(entity) = game.bonus.entity {
        game.score -= 3;
        commands.entity(entity).despawn_recursive();
        game.bonus.entity = None;
        if game.score <= -5 {
            state.set(GameState::GameOver).unwrap();
            return;
        }
    }

    // ensure bonus doesn't spawn on the player
    loop {
        game.bonus.i = rand::thread_rng().gen_range(0..BOARD_SIZE_I);
        game.bonus.j = rand::thread_rng().gen_range(0..BOARD_SIZE_J);
        if game.bonus.i != game.player.i || game.bonus.j != game.player.j {
            break;
        }
    }
    game.bonus.entity = Some(
        commands
            .spawn_bundle((
                Transform {
                    translation: Vec3::new(
                        game.bonus.i as f32,
                        game.board[game.bonus.j][game.bonus.i].height + 0.2,
                        game.bonus.j as f32,
                    ),
                    ..Default::default()
                },
                GlobalTransform::identity(),
            ))
            .with_children(|children| {
                children.spawn_bundle(PointLightBundle {
                    point_light: PointLight {
                        color: Color::rgb(1.0, 1.0, 0.0),
                        intensity: 1000.0,
                        range: 10.0,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(0.0, 2.0, 0.0),
                    ..Default::default()
                });
                children.spawn_scene(game.bonus.handle.clone());
            })
            .id(),
    );
}

// let the cake turn on itself
fn rotate_bonus(game: Res<Game>, time: Res<Time>, mut transforms: Query<&mut Transform>) {
    if let Some(entity) = game.bonus.entity {
        if let Ok(mut cake_transform) = transforms.get_mut(entity) {
            cake_transform.rotate(Quat::from_rotation_y(time.delta_seconds()));
            cake_transform.scale = Vec3::splat(
                1.0 + (game.score as f32 / 10.0 * time.seconds_since_startup().sin() as f32).abs(),
            );
        }
    }
}

// update the score displayed during the game
fn scoreboard_system(game: Res<Game>, mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[0].value = format!("Sugar Rush: {}", game.score);
}

// restart the game when pressing spacebar
fn gameover_keyboard(mut state: ResMut<State<GameState>>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set(GameState::Playing).unwrap();
    }
}

// display the number of cake eaten before losing
fn display_score(mut commands: Commands, asset_server: Res<AssetServer>, game: Res<Game>) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                margin: Rect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section(
                    format!("Cake eaten: {}", game.cake_eaten),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 80.0,
                        color: Color::rgb(0.5, 0.5, 1.0),
                    },
                    Default::default(),
                ),
                ..Default::default()
            });
        });
}
