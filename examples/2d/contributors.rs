use bevy::{prelude::*, utils::HashSet};
use rand::{prelude::SliceRandom, Rng};
use std::{
    env::VarError,
    io::{self, BufRead, BufReader},
    process::Stdio,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_contributor_selection)
        .add_startup_system(setup)
        .add_system(velocity_system)
        .add_system(move_system)
        .add_system(collision_system)
        .add_system(select_system)
        .run();
}

// Store contributors in a collection that preserves the uniqueness
type Contributors = HashSet<String>;

struct ContributorSelection {
    order: Vec<(String, Entity)>,
    idx: usize,
}

#[derive(Component)]
struct SelectTimer;

#[derive(Component)]
struct ContributorDisplay;

#[derive(Component)]
struct Contributor {
    hue: f32,
}

#[derive(Component)]
struct Velocity {
    translation: Vec3,
    rotation: f32,
}

const GRAVITY: f32 = -9.821 * 100.0;
const SPRITE_SIZE: f32 = 75.0;

const SATURATION_DESELECTED: f32 = 0.3;
const LIGHTNESS_DESELECTED: f32 = 0.2;
const SATURATION_SELECTED: f32 = 0.9;
const LIGHTNESS_SELECTED: f32 = 0.7;
const ALPHA: f32 = 0.92;

const SHOWCASE_TIMER_SECS: f32 = 3.0;

const CONTRIBUTORS_LIST: &[&str] = &["Carter Anderson", "And Many More"];

fn setup_contributor_selection(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load contributors from the git history log or use default values from
    // the constant array. Contributors must be unique, so they are stored in a HashSet
    let contribs = contributors().unwrap_or_else(|_| {
        CONTRIBUTORS_LIST
            .iter()
            .map(|name| name.to_string())
            .collect()
    });

    let texture_handle = asset_server.load("branding/icon.png");

    let mut contributor_selection = ContributorSelection {
        order: vec![],
        idx: 0,
    };

    let mut rnd = rand::thread_rng();

    for name in contribs {
        let pos = (rnd.gen_range(-400.0..400.0), rnd.gen_range(0.0..400.0));
        let dir = rnd.gen_range(-1.0..1.0);
        let velocity = Vec3::new(dir * 500.0, 0.0, 0.0);
        let hue = rnd.gen_range(0.0..=360.0);

        // some sprites should be flipped
        let flipped = rnd.gen_bool(0.5);

        let transform = Transform::from_xyz(pos.0, pos.1, 0.0);

        let entity = commands
            .spawn()
            .insert_bundle((
                Contributor { hue },
                Velocity {
                    translation: velocity,
                    rotation: -dir * 5.0,
                },
            ))
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(1.0, 1.0) * SPRITE_SIZE),
                    color: Color::hsla(hue, SATURATION_DESELECTED, LIGHTNESS_DESELECTED, ALPHA),
                    flip_x: flipped,
                    ..Default::default()
                },
                texture: texture_handle.clone(),
                transform,
                ..Default::default()
            })
            .id();

        contributor_selection.order.push((name, entity));
    }

    contributor_selection.order.shuffle(&mut rnd);

    commands.insert_resource(contributor_selection);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    commands.spawn_bundle((SelectTimer, Timer::from_seconds(SHOWCASE_TIMER_SECS, true)));

    commands
        .spawn()
        .insert(ContributorDisplay)
        .insert_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Contributor showcase".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 60.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 60.0,
                            color: Color::WHITE,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        });
}

/// Finds the next contributor to display and selects the entity
fn select_system(
    mut contributor_selection: ResMut<ContributorSelection>,
    mut text_query: Query<&mut Text, With<ContributorDisplay>>,
    mut timer_query: Query<&mut Timer, With<SelectTimer>>,
    mut query: Query<(&Contributor, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    let mut timer_fired = false;
    for mut timer in timer_query.iter_mut() {
        if !timer.tick(time.delta()).just_finished() {
            continue;
        }
        timer.reset();
        timer_fired = true;
    }

    if !timer_fired {
        return;
    }

    let prev = contributor_selection.idx;

    if (contributor_selection.idx + 1) < contributor_selection.order.len() {
        contributor_selection.idx += 1;
    } else {
        contributor_selection.idx = 0;
    }

    {
        let (_, entity) = &contributor_selection.order[prev];
        if let Ok((contributor, mut sprite, mut transform)) = query.get_mut(*entity) {
            deselect(&mut sprite, contributor, &mut *transform);
        }
    }

    let (name, entity) = &contributor_selection.order[contributor_selection.idx];

    if let Ok((contributor, mut sprite, mut transform)) = query.get_mut(*entity) {
        if let Some(mut text) = text_query.iter_mut().next() {
            select(&mut sprite, contributor, &mut *transform, &mut *text, name);
        }
    }
}

/// Change the modulate color to the "selected" colour,
/// bring the object to the front and display the name.
fn select(
    sprite: &mut Sprite,
    contributor: &Contributor,
    transform: &mut Transform,
    text: &mut Text,
    name: &str,
) {
    sprite.color = Color::hsla(
        contributor.hue,
        SATURATION_SELECTED,
        LIGHTNESS_SELECTED,
        ALPHA,
    );

    transform.translation.z = 100.0;

    text.sections[0].value = "Contributor: ".to_string();
    text.sections[1].value = name.to_string();
    text.sections[1].style.color = sprite.color;
}

/// Change the modulate color to the "deselected" colour and push
/// the object to the back.
fn deselect(sprite: &mut Sprite, contributor: &Contributor, transform: &mut Transform) {
    sprite.color = Color::hsla(
        contributor.hue,
        SATURATION_DESELECTED,
        LIGHTNESS_DESELECTED,
        ALPHA,
    );

    transform.translation.z = 0.0;
}

/// Applies gravity to all entities with velocity
fn velocity_system(time: Res<Time>, mut velocity_query: Query<&mut Velocity>) {
    let delta = time.delta_seconds();

    for mut velocity in velocity_query.iter_mut() {
        velocity.translation += Vec3::new(0.0, GRAVITY * delta, 0.0);
    }
}

/// Checks for collisions of contributor-birds.
///
/// On collision with left-or-right wall it resets the horizontal
/// velocity. On collision with the ground it applies an upwards
/// force.
fn collision_system(
    windows: Res<Windows>,
    mut query: Query<(&mut Velocity, &mut Transform), With<Contributor>>,
) {
    let mut rnd = rand::thread_rng();

    let window = windows.get_primary().unwrap();

    let ceiling = window.height() / 2.;
    let ground = -(window.height() / 2.);

    let wall_left = -(window.width() / 2.);
    let wall_right = window.width() / 2.;

    for (mut velocity, mut transform) in query.iter_mut() {
        let left = transform.translation.x - SPRITE_SIZE / 2.0;
        let right = transform.translation.x + SPRITE_SIZE / 2.0;
        let top = transform.translation.y + SPRITE_SIZE / 2.0;
        let bottom = transform.translation.y - SPRITE_SIZE / 2.0;

        // clamp the translation to not go out of the bounds
        if bottom < ground {
            transform.translation.y = ground + SPRITE_SIZE / 2.0;
            // apply an impulse upwards
            velocity.translation.y = rnd.gen_range(700.0..1000.0);
        }
        if top > ceiling {
            transform.translation.y = ceiling - SPRITE_SIZE / 2.0;
        }
        // on side walls flip the horizontal velocity
        if left < wall_left {
            transform.translation.x = wall_left + SPRITE_SIZE / 2.0;
            velocity.translation.x *= -1.0;
            velocity.rotation *= -1.0;
        }
        if right > wall_right {
            transform.translation.x = wall_right - SPRITE_SIZE / 2.0;
            velocity.translation.x *= -1.0;
            velocity.rotation *= -1.0;
        }
    }
}

/// Apply velocity to positions and rotations.
fn move_system(time: Res<Time>, mut query: Query<(&Velocity, &mut Transform)>) {
    let delta = time.delta_seconds();

    for (velocity, mut transform) in query.iter_mut() {
        transform.translation += delta * velocity.translation;
        transform.rotate(Quat::from_rotation_z(velocity.rotation * delta));
    }
}

enum LoadContributorsError {
    IO(io::Error),
    Var(VarError),
    Stdout,
}

/// Get the names of all contributors from the git log.
///
/// The names are deduplicated.
/// This function only works if `git` is installed and
/// the program is run through `cargo`.
fn contributors() -> Result<Contributors, LoadContributorsError> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(LoadContributorsError::Var)?;

    let mut cmd = std::process::Command::new("git")
        .args(&["--no-pager", "log", "--pretty=format:%an"])
        .current_dir(manifest_dir)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(LoadContributorsError::IO)?;

    let stdout = cmd.stdout.take().ok_or(LoadContributorsError::Stdout)?;

    let contributors = BufReader::new(stdout)
        .lines()
        .filter_map(|x| x.ok())
        .collect();

    Ok(contributors)
}
