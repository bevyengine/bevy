use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::{camera::Camera, render_graph::base::camera::CAMERA_3D},
};
use rand::Rng;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum GameStage {
    InGame,
    BonusUpdate,
}

#[derive(Clone, PartialEq, Debug)]
enum GameState {
    Playing,
    GameOver,
}

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .init_resource::<Game>()
        .add_plugins(DefaultPlugins)
        .insert_resource(State::new(GameState::Playing))
        .add_startup_system(setup_cameras.system())
        .add_stage_after(
            CoreStage::Update,
            GameStage::InGame,
            StateStage::<GameState>::default(),
        )
        .on_state_enter(GameStage::InGame, GameState::Playing, setup.system())
        .on_state_update(GameStage::InGame, GameState::Playing, move_player.system())
        .on_state_update(GameStage::InGame, GameState::Playing, focus_camera.system())
        .on_state_update(GameStage::InGame, GameState::Playing, rotate_bonus.system())
        .on_state_update(
            GameStage::InGame,
            GameState::Playing,
            scoreboard_system.system(),
        )
        .on_state_exit(GameStage::InGame, GameState::Playing, teardown.system())
        .on_state_enter(
            GameStage::InGame,
            GameState::GameOver,
            display_score.system(),
        )
        .on_state_update(
            GameStage::InGame,
            GameState::GameOver,
            gameover_keyboard.system(),
        )
        .on_state_exit(GameStage::InGame, GameState::GameOver, teardown.system())
        .add_stage_after(
            CoreStage::Update,
            GameStage::BonusUpdate,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(5.0))
                .with_system(spawn_bonus.system()),
        )
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

fn setup_cameras(commands: &mut Commands, mut game: ResMut<Game>) {
    game.camera_should_focus = Vec3::from(RESET_FOCUS);
    game.camera_is_focus = game.camera_should_focus;
    commands
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_xyz(
                -(BOARD_SIZE_I as f32 / 2.0),
                2.0 * BOARD_SIZE_J as f32 / 3.0,
                BOARD_SIZE_J as f32 / 2.0 - 0.5,
            )
            .looking_at(game.camera_is_focus, Vec3::unit_y()),
            ..Default::default()
        })
        .spawn(UiCameraBundle::default());
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>, mut game: ResMut<Game>) {
    // reset the game state
    game.cake_eaten = 0;
    game.score = 0;
    game.player.i = BOARD_SIZE_I / 2;
    game.player.j = BOARD_SIZE_J / 2;

    commands.spawn(LightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
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
                        .spawn((
                            Transform::from_xyz(i as f32, height - 0.2, j as f32),
                            GlobalTransform::default(),
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
    game.player.entity = commands
        .spawn((
            Transform {
                translation: Vec3::new(
                    game.player.i as f32,
                    game.board[game.player.j][game.player.i].height,
                    game.player.j as f32,
                ),
                rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                ..Default::default()
            },
            GlobalTransform::default(),
        ))
        .with_children(|cell| {
            cell.spawn_scene(asset_server.load("models/AlienCake/alien.glb#Scene0"));
        })
        .current_entity();

    // load the scene for the cake
    game.bonus.handle = asset_server.load("models/AlienCake/cakeBirthday.glb#Scene0");

    // scoreboard
    commands.spawn(TextBundle {
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
fn teardown(commands: &mut Commands, entities: Query<Entity, Without<Camera>>) {
    for entity in entities.iter() {
        commands.despawn_recursive(entity);
    }
}

// control the game character
fn move_player(
    commands: &mut Commands,
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
            commands.despawn_recursive(entity);
            game.bonus.entity = None;
        }
    }
}

// change the focus of the camera
fn focus_camera(
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut transforms: QuerySet<(Query<(&mut Transform, &Camera)>, Query<&Transform>)>,
) {
    const SPEED: f32 = 2.0;
    // if there is both a player and a bonus, target the mid-point of them
    if let (Some(player_entity), Some(bonus_entity)) = (game.player.entity, game.bonus.entity) {
        if let (Ok(player_transform), Ok(bonus_transform)) = (
            transforms.q1().get(player_entity),
            transforms.q1().get(bonus_entity),
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
    for (mut transform, camera) in transforms.q0_mut().iter_mut() {
        if camera.name == Some(CAMERA_3D.to_string()) {
            *transform = transform.looking_at(game.camera_is_focus, Vec3::unit_y());
        }
    }
}

// despawn the bonus if there is one, then spawn a new one at a random location
fn spawn_bonus(
    mut state: ResMut<State<GameState>>,
    commands: &mut Commands,
    mut game: ResMut<Game>,
) {
    if *state.current() != GameState::Playing {
        return;
    }
    if let Some(entity) = game.bonus.entity {
        game.score -= 3;
        commands.despawn_recursive(entity);
        game.bonus.entity = None;
        if game.score <= -5 {
            state.set_next(GameState::GameOver).unwrap();
            return;
        }
    }
    game.bonus.i = rand::thread_rng().gen_range(0..BOARD_SIZE_I);
    game.bonus.j = rand::thread_rng().gen_range(0..BOARD_SIZE_J);
    game.bonus.entity = commands
        .spawn((
            Transform {
                translation: Vec3::new(
                    game.bonus.i as f32,
                    game.board[game.player.j][game.player.i].height + 0.2,
                    game.bonus.j as f32,
                ),
                ..Default::default()
            },
            GlobalTransform::default(),
        ))
        .with_children(|cell| {
            cell.spawn_scene(game.bonus.handle.clone());
        })
        .current_entity();
}

// let the cake turn on itself
fn rotate_bonus(game: Res<Game>, time: Res<Time>, mut transforms: Query<&mut Transform>) {
    if let Some(entity) = game.bonus.entity {
        let mut cake_transform = transforms.get_mut(entity).unwrap();
        cake_transform.rotate(Quat::from_rotation_y(time.delta_seconds()));
        cake_transform.scale = Vec3::splat(
            1.0 + (game.score as f32 / 10.0 * time.seconds_since_startup().sin() as f32).abs(),
        );
    }
}

// update the score displayed during the game
fn scoreboard_system(game: Res<Game>, mut query: Query<&mut Text>) {
    for mut text in query.iter_mut() {
        text.sections[0].value = format!("Sugar Rush: {}", game.score);
    }
}

// restart the game when pressing spacebar
fn gameover_keyboard(mut state: ResMut<State<GameState>>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set_next(GameState::Playing).unwrap();
    }
}

// display the number of cake eaten before losing
fn display_score(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    game: Res<Game>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn(NodeBundle {
            style: Style {
                margin: Rect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle {
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
