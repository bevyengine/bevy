use bevy::prelude::*;
use rand::{prelude::SliceRandom, Rng};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader},
    process::Stdio,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(velocity_system)
        .add_system(move_system)
        .add_system(collision_system)
        .add_system(select_system)
        .run();
}

type Contributors = BTreeSet<String>;

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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let contribs = contributors();

    let texture_handle = asset_server.load("branding/icon.png");

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    let mut sel = ContributorSelection {
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

        let e = commands
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
                    size: Vec2::new(1.0, 1.0) * SPRITE_SIZE,
                    resize_mode: SpriteResizeMode::Manual,
                    flip_x: flipped,
                    ..Default::default()
                },
                material: materials.add(ColorMaterial {
                    color: Color::hsla(hue, SATURATION_DESELECTED, LIGHTNESS_DESELECTED, ALPHA),
                    texture: Some(texture_handle.clone()),
                }),
                transform,
                ..Default::default()
            })
            .id();

        sel.order.push((name, e));
    }

    sel.order.shuffle(&mut rnd);

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

    commands.insert_resource(sel);
}

/// Finds the next contributor to display and selects the entity
fn select_system(
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut sel: ResMut<ContributorSelection>,
    mut dq: Query<&mut Text, With<ContributorDisplay>>,
    mut tq: Query<&mut Timer, With<SelectTimer>>,
    mut q: Query<(&Contributor, &Handle<ColorMaterial>, &mut Transform)>,
    time: Res<Time>,
) {
    let mut timer_fired = false;
    for mut t in tq.iter_mut() {
        if !t.tick(time.delta()).just_finished() {
            continue;
        }
        t.reset();
        timer_fired = true;
    }

    if !timer_fired {
        return;
    }

    let prev = sel.idx;

    if (sel.idx + 1) < sel.order.len() {
        sel.idx += 1;
    } else {
        sel.idx = 0;
    }

    {
        let (_, e) = &sel.order[prev];
        if let Ok((c, handle, mut tr)) = q.get_mut(*e) {
            deselect(&mut *materials, handle.clone(), c, &mut *tr);
        }
    }

    let (name, e) = &sel.order[sel.idx];

    if let Ok((c, handle, mut tr)) = q.get_mut(*e) {
        if let Some(mut text) = dq.iter_mut().next() {
            select(&mut *materials, handle, c, &mut *tr, &mut *text, name);
        }
    }
}

/// Change the modulate color to the "selected" colour,
/// bring the object to the front and display the name.
fn select(
    materials: &mut Assets<ColorMaterial>,
    mat_handle: &Handle<ColorMaterial>,
    cont: &Contributor,
    trans: &mut Transform,
    text: &mut Text,
    name: &str,
) -> Option<()> {
    let mat = materials.get_mut(mat_handle)?;
    mat.color = Color::hsla(cont.hue, SATURATION_SELECTED, LIGHTNESS_SELECTED, ALPHA);

    trans.translation.z = 100.0;

    text.sections[0].value = "Contributor: ".to_string();
    text.sections[1].value = name.to_string();
    text.sections[1].style.color = mat.color;

    Some(())
}

/// Change the modulate color to the "deselected" colour and push
/// the object to the back.
fn deselect(
    materials: &mut Assets<ColorMaterial>,
    mat_handle: Handle<ColorMaterial>,
    cont: &Contributor,
    trans: &mut Transform,
) -> Option<()> {
    let mat = materials.get_mut(mat_handle)?;
    mat.color = Color::hsla(cont.hue, SATURATION_DESELECTED, LIGHTNESS_DESELECTED, ALPHA);

    trans.translation.z = 0.0;

    Some(())
}

/// Applies gravity to all entities with velocity
fn velocity_system(time: Res<Time>, mut q: Query<&mut Velocity>) {
    let delta = time.delta_seconds();

    for mut v in q.iter_mut() {
        v.translation += Vec3::new(0.0, GRAVITY * delta, 0.0);
    }
}

/// Checks for collisions of contributor-birds.
///
/// On collision with left-or-right wall it resets the horizontal
/// velocity. On collision with the ground it applies an upwards
/// force.
fn collision_system(
    wins: Res<Windows>,
    mut q: Query<(&mut Velocity, &mut Transform), With<Contributor>>,
) {
    let mut rnd = rand::thread_rng();

    let win = wins.get_primary().unwrap();

    let ceiling = win.height() / 2.;
    let ground = -(win.height() / 2.);

    let wall_left = -(win.width() / 2.);
    let wall_right = win.width() / 2.;

    for (mut v, mut t) in q.iter_mut() {
        let left = t.translation.x - SPRITE_SIZE / 2.0;
        let right = t.translation.x + SPRITE_SIZE / 2.0;
        let top = t.translation.y + SPRITE_SIZE / 2.0;
        let bottom = t.translation.y - SPRITE_SIZE / 2.0;

        // clamp the translation to not go out of the bounds
        if bottom < ground {
            t.translation.y = ground + SPRITE_SIZE / 2.0;
            // apply an impulse upwards
            v.translation.y = rnd.gen_range(700.0..1000.0);
        }
        if top > ceiling {
            t.translation.y = ceiling - SPRITE_SIZE / 2.0;
        }
        // on side walls flip the horizontal velocity
        if left < wall_left {
            t.translation.x = wall_left + SPRITE_SIZE / 2.0;
            v.translation.x *= -1.0;
            v.rotation *= -1.0;
        }
        if right > wall_right {
            t.translation.x = wall_right - SPRITE_SIZE / 2.0;
            v.translation.x *= -1.0;
            v.rotation *= -1.0;
        }
    }
}

/// Apply velocity to positions and rotations.
fn move_system(time: Res<Time>, mut q: Query<(&Velocity, &mut Transform)>) {
    let delta = time.delta_seconds();

    for (v, mut t) in q.iter_mut() {
        t.translation += delta * v.translation;
        t.rotate(Quat::from_rotation_z(v.rotation * delta));
    }
}

/// Get the names of all contributors from the git log.
///
/// The names are deduplicated.
/// This function only works if `git` is installed and
/// the program is run through `cargo`.
fn contributors() -> Contributors {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("This example needs to run through `cargo run --example`.");

    let mut cmd = std::process::Command::new("git")
        .args(&["--no-pager", "log", "--pretty=format:%an"])
        .current_dir(manifest_dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("`git` needs to be installed.");

    let stdout = cmd.stdout.take().expect("`Child` should have a stdout.");

    BufReader::new(stdout)
        .lines()
        .filter_map(|x| x.ok())
        .collect()
}
