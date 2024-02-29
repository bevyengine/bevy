//! This example displays each contributor to the bevy source code as a bouncing bevy-ball.

use bevy::{
    prelude::*,
    utils::{thiserror, HashMap},
};
use rand::{prelude::SliceRandom, Rng};
use std::{
    env::VarError,
    hash::{DefaultHasher, Hash, Hasher},
    io::{self, BufRead, BufReader},
    process::Stdio,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SelectionTimer>()
        .add_systems(Startup, (setup_contributor_selection, setup))
        .add_systems(
            Update,
            (
                velocity_system,
                move_system,
                collision_system,
                select_system,
            ),
        )
        .run();
}

// Store contributors in a collection that preserves the uniqueness
type Contributors = HashMap<String, usize>;

#[derive(Resource)]
struct ContributorSelection {
    order: Vec<Entity>,
    idx: usize,
}

#[derive(Resource)]
struct SelectionTimer(Timer);

impl Default for SelectionTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            SHOWCASE_TIMER_SECS,
            TimerMode::Repeating,
        ))
    }
}

#[derive(Component)]
struct ContributorDisplay;

#[derive(Component)]
struct Contributor {
    name: String,
    num_commits: usize,
    hue: f32,
}

#[derive(Component)]
struct Velocity {
    translation: Vec3,
    rotation: f32,
}

const GRAVITY: f32 = 9.821 * 100.0;
const SPRITE_SIZE: f32 = 75.0;

const SELECTED: Hsla = Hsla::hsl(0.0, 0.9, 0.7);
const DESELECTED: Hsla = Hsla::new(0.0, 0.3, 0.2, 0.92);

const SHOWCASE_TIMER_SECS: f32 = 3.0;

const CONTRIBUTORS_LIST: &[&str] = &["Carter Anderson", "And Many More"];

fn setup_contributor_selection(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load contributors from the git history log or use default values from
    // the constant array. Contributors must be unique, so they are stored in a HashSet
    let contribs = contributors().unwrap_or_else(|_| {
        CONTRIBUTORS_LIST
            .iter()
            .map(|name| (name.to_string(), 1))
            .collect()
    });

    let texture_handle = asset_server.load("branding/icon.png");

    let mut contributor_selection = ContributorSelection {
        order: Vec::with_capacity(contribs.len()),
        idx: 0,
    };

    let mut rng = rand::thread_rng();

    for (name, num) in contribs {
        let pos = (rng.gen_range(-400.0..400.0), rng.gen_range(0.0..400.0));
        let dir = rng.gen_range(-1.0..1.0);
        let velocity = Vec3::new(dir * 500.0, 0.0, 0.0);
        let hue = str_to_hue(&name);

        // some sprites should be flipped
        let flipped = rng.gen_bool(0.5);

        let transform = Transform::from_xyz(pos.0, pos.1, 0.0);

        let entity = commands
            .spawn((
                Contributor {
                    name,
                    num_commits: num,
                    hue,
                },
                Velocity {
                    translation: velocity,
                    rotation: -dir * 5.0,
                },
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(1.0, 1.0) * SPRITE_SIZE),
                        color: DESELECTED.with_hue(hue).into(),
                        flip_x: flipped,
                        ..default()
                    },
                    texture: texture_handle.clone(),
                    transform,
                    ..default()
                },
            ))
            .id();

        contributor_selection.order.push(entity);
    }

    contributor_selection.order.shuffle(&mut rng);

    commands.insert_resource(contributor_selection);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 60.0,
        ..default()
    };

    commands.spawn((
        TextBundle::from_sections([
            TextSection::new("Contributor showcase", text_style.clone()),
            TextSection::from_style(TextStyle {
                font_size: 30.,
                ..text_style
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        }),
        ContributorDisplay,
    ));
}

/// Finds the next contributor to display and selects the entity
fn select_system(
    mut timer: ResMut<SelectionTimer>,
    mut contributor_selection: ResMut<ContributorSelection>,
    mut text_query: Query<&mut Text, With<ContributorDisplay>>,
    mut query: Query<(&Contributor, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let entity = contributor_selection.order[contributor_selection.idx];
    if let Ok((contributor, mut sprite, mut transform)) = query.get_mut(entity) {
        deselect(&mut sprite, contributor, &mut transform);
    }

    if (contributor_selection.idx + 1) < contributor_selection.order.len() {
        contributor_selection.idx += 1;
    } else {
        contributor_selection.idx = 0;
    }

    let entity = contributor_selection.order[contributor_selection.idx];

    if let Ok((contributor, mut sprite, mut transform)) = query.get_mut(entity) {
        let mut text = text_query.single_mut();
        select(&mut sprite, contributor, &mut transform, &mut text);
    }
}

/// Change the tint color to the "selected" color, bring the object to the front
/// and display the name.
fn select(
    sprite: &mut Sprite,
    contributor: &Contributor,
    transform: &mut Transform,
    text: &mut Text,
) {
    sprite.color = SELECTED.with_hue(contributor.hue).into();

    transform.translation.z = 100.0;

    text.sections[0].value.clone_from(&contributor.name);
    text.sections[1].value = format!(
        "\n{} commit{}",
        contributor.num_commits,
        if contributor.num_commits > 1 { "s" } else { "" }
    );
    text.sections[0].style.color = sprite.color;
}

/// Change the modulate color to the "deselected" color and push
/// the object to the back.
fn deselect(sprite: &mut Sprite, contributor: &Contributor, transform: &mut Transform) {
    sprite.color = DESELECTED.with_hue(contributor.hue).into();

    transform.translation.z = 0.0;
}

/// Applies gravity to all entities with velocity
fn velocity_system(time: Res<Time>, mut velocity_query: Query<&mut Velocity>) {
    let delta = time.delta_seconds();

    for mut velocity in &mut velocity_query {
        velocity.translation.y -= GRAVITY * delta;
    }
}

/// Checks for collisions of contributor-birds.
///
/// On collision with left-or-right wall it resets the horizontal
/// velocity. On collision with the ground it applies an upwards
/// force.
fn collision_system(
    windows: Query<&Window>,
    mut query: Query<(&mut Velocity, &mut Transform), With<Contributor>>,
) {
    let window = windows.single();

    let ceiling = window.height() / 2.;
    let ground = -window.height() / 2.;

    let wall_left = -window.width() / 2.;
    let wall_right = window.width() / 2.;

    // The maximum height the birbs should try to reach is one birb below the top of the window.
    let max_bounce_height = (window.height() - SPRITE_SIZE * 2.0).max(0.0);

    let mut rng = rand::thread_rng();

    for (mut velocity, mut transform) in &mut query {
        let left = transform.translation.x - SPRITE_SIZE / 2.0;
        let right = transform.translation.x + SPRITE_SIZE / 2.0;
        let top = transform.translation.y + SPRITE_SIZE / 2.0;
        let bottom = transform.translation.y - SPRITE_SIZE / 2.0;

        // clamp the translation to not go out of the bounds
        if bottom < ground {
            transform.translation.y = ground + SPRITE_SIZE / 2.0;

            // How high this birb will bounce.
            let bounce_height = rng.gen_range((max_bounce_height * 0.4)..=max_bounce_height);

            // Apply the velocity that would bounce the birb up to bounce_height.
            velocity.translation.y = (bounce_height * GRAVITY * 2.).sqrt();
        }
        if top > ceiling {
            transform.translation.y = ceiling - SPRITE_SIZE / 2.0;
            velocity.translation.y *= -1.0;
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

    for (velocity, mut transform) in &mut query {
        transform.translation += delta * velocity.translation;
        transform.rotate_z(velocity.rotation * delta);
    }
}

#[derive(Debug, thiserror::Error)]
enum LoadContributorsError {
    #[error("An IO error occurred while reading the git log.")]
    Io(#[from] io::Error),
    #[error("The CARGO_MANIFEST_DIR environment variable was not set.")]
    Var(#[from] VarError),
    #[error("The git process did not return a stdout handle.")]
    Stdout,
}

/// Get the names of all contributors from the git log.
///
/// The names are deduplicated.
/// This function only works if `git` is installed and
/// the program is run through `cargo`.
fn contributors() -> Result<Contributors, LoadContributorsError> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;

    let mut cmd = std::process::Command::new("git")
        .args(["--no-pager", "log", "--pretty=format:%an"])
        .current_dir(manifest_dir)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = cmd.stdout.take().ok_or(LoadContributorsError::Stdout)?;

    let contributors = BufReader::new(stdout).lines().map_while(Result::ok).fold(
        HashMap::new(),
        |mut acc, word| {
            *acc.entry(word.to_string()).or_insert(0) += 1;
            acc
        },
    );

    Ok(contributors)
}

fn str_to_hue(s: &str) -> f32 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() as f32 / u64::MAX as f32 * 360.
}
