use bevy::prelude::*;
use rand::{prelude::SliceRandom, Rng};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader},
    process::Stdio,
};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(velocity_system.system())
        .add_system(move_system.system())
        .add_system(collision_system.system())
        .add_system(select_system.system())
        .run();
}

type Contributors = BTreeSet<String>;

struct ContributorSelection {
    order: Vec<(String, Entity)>,
    idx: usize,
}

struct SelectTimer;

struct ContributorDisplay;

struct Contributor {
    color: [f32; 3],
}

struct Velocity {
    translation: Vec3,
    rotation: f32,
}

const GRAVITY: f32 = -9.821 * 100.0;
const SPRITE_SIZE: f32 = 75.0;

const COL_DESELECTED: Color = Color::rgb_linear(0.03, 0.03, 0.03);
const COL_SELECTED: Color = Color::rgb_linear(5.0, 5.0, 5.0);

const SHOWCASE_TIMER_SECS: f32 = 3.0;

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let contribs = contributors();

    let texture_handle = asset_server.load("branding/icon.png");

    commands
        .spawn(Camera2dBundle::default())
        .spawn(CameraUiBundle::default());

    let mut sel = ContributorSelection {
        order: vec![],
        idx: 0,
    };

    let mut rnd = rand::thread_rng();

    for name in contribs {
        let pos = (rnd.gen_range(-400.0..400.0), rnd.gen_range(0.0..400.0));
        let dir = rnd.gen_range(-1.0..1.0);
        let velocity = Vec3::new(dir * 500.0, 0.0, 0.0);
        let col = gen_color(&mut rnd);

        // some sprites should be flipped
        let flipped = rnd.gen_bool(0.5);

        let mut transform = Transform::from_xyz(pos.0, pos.1, 0.0);
        transform.scale.x *= if flipped { -1.0 } else { 1.0 };

        commands
            .spawn((Contributor { color: col },))
            .with(Velocity {
                translation: velocity,
                rotation: -dir * 5.0,
            })
            .with_bundle(SpriteBundle {
                sprite: Sprite {
                    size: Vec2::new(1.0, 1.0) * SPRITE_SIZE,
                    resize_mode: SpriteResizeMode::Manual,
                },
                material: materials.add(ColorMaterial {
                    color: COL_DESELECTED * col,
                    texture: Some(texture_handle.clone()),
                }),
                ..Default::default()
            })
            .with(transform);

        let e = commands.current_entity().unwrap();

        sel.order.push((name, e));
    }

    sel.order.shuffle(&mut rnd);

    commands.spawn((SelectTimer, Timer::from_seconds(SHOWCASE_TIMER_SECS, true)));

    commands
        .spawn((ContributorDisplay,))
        .with_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                value: "Contributor showcase".to_string(),
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            },
            ..Default::default()
        });

    commands.insert_resource(sel);
}

/// Finds the next contributor to display and selects the entity
fn select_system(
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut sel: ResMut<ContributorSelection>,
    mut dq: Query<Mut<Text>, With<ContributorDisplay>>,
    mut tq: Query<Mut<Timer>, With<SelectTimer>>,
    mut q: Query<(&Contributor, &Handle<ColorMaterial>, &mut Transform)>,
    time: Res<Time>,
) {
    let mut timer_fired = false;
    for mut t in tq.iter_mut() {
        if !t.tick(time.delta_seconds()).just_finished() {
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
        for mut text in dq.iter_mut() {
            select(
                &mut *materials,
                handle.clone(),
                c,
                &mut *tr,
                &mut *text,
                name,
            );
        }
    }
}

/// Change the modulate color to the "selected" colour,
/// bring the object to the front and display the name.
fn select(
    materials: &mut Assets<ColorMaterial>,
    mat_handle: Handle<ColorMaterial>,
    cont: &Contributor,
    trans: &mut Transform,
    text: &mut Text,
    name: &str,
) -> Option<()> {
    let mat = materials.get_mut(mat_handle)?;
    mat.color = COL_SELECTED * cont.color;

    trans.translation.z = 100.0;

    text.value = format!("Contributor: {}", name);

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
    mat.color = COL_DESELECTED * cont.color;

    trans.translation.z = 0.0;

    Some(())
}

/// Applies gravity to all entities with velocity
fn velocity_system(time: Res<Time>, mut q: Query<Mut<Velocity>>) {
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
    mut q: Query<(Mut<Velocity>, Mut<Transform>), With<Contributor>>,
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
fn move_system(time: Res<Time>, mut q: Query<(&Velocity, Mut<Transform>)>) {
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

/// Generate a color modulation
///
/// Because there is no `Mul<Color> for Color` instead `[f32; 3]` is
/// used.
fn gen_color(rng: &mut impl Rng) -> [f32; 3] {
    let r = rng.gen_range(0.2..1.0);
    let g = rng.gen_range(0.2..1.0);
    let b = rng.gen_range(0.2..1.0);
    let v = Vec3::new(r, g, b);
    v.normalize().into()
}
