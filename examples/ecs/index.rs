//! Demonstrates how to query for a component _value_ using indexes.

use bevy::{dev_tools::fps_overlay::FpsOverlayPlugin, prelude::*};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// To query by component value, first we need to ensure our component is suitable
// for indexing.
//
// The hard requirements are:
// * Immutability
// * Eq + Hash + Clone

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Conway's Game of Life".into(),
                    name: Some("conway.app".into()),
                    resolution: (680., 700.).into(),
                    ..default()
                }),
                ..default()
            }),
            FpsOverlayPlugin::default(),
        ))
        .add_index::<Chunk>()
        .insert_resource(Time::<Fixed>::from_seconds(0.1))
        .add_systems(Startup, setup)
        .add_systems(Update, randomly_revive)
        .add_systems(FixedUpdate, (spread_livlihood, update_state).chain())
        .run();
}

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[component(immutable, storage = "SparseSet")]
struct Alive;

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[component(immutable)]
struct Position(i8, i8);

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[component(immutable)]
struct Chunk(i8, i8);

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct LivingNeighbors(u8);

#[derive(Resource)]
struct LivingHandles {
    mesh: Handle<Mesh>,
    alive_material: Handle<ColorMaterial>,
    dead_material: Handle<ColorMaterial>,
}

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let rect = meshes.add(Rectangle::new(10., 10.));
    let alive = materials.add(Color::BLACK);
    let dead = materials.add(Color::WHITE);

    commands.insert_resource(LivingHandles {
        mesh: rect.clone(),
        alive_material: alive.clone(),
        dead_material: dead.clone(),
    });

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    commands.insert_resource(SeededRng(seeded_rng));

    // Spawn the cells
    commands.spawn_batch((-30..30).flat_map(|x| (-30..30).map(move |y| (x, y))).map(
        move |(x, y)| {
            (
                Position(x, y),
                Chunk(x / 4, y / 4),
                LivingNeighbors(0),
                Mesh2d(rect.clone()),
                MeshMaterial2d(dead.clone()),
                Transform::from_xyz(11. * x as f32 + 5.5, 11. * y as f32 + 5.5 - 20., 0.),
            )
        },
    ));
}

fn randomly_revive(
    mut commands: Commands,
    mut rng: ResMut<SeededRng>,
    handles: Res<LivingHandles>,
    mut query: Query<
        (Entity, &mut MeshMaterial2d<ColorMaterial>),
        (With<Position>, Without<Alive>),
    >,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for (entity, mut material) in query.iter_mut() {
            if rng.0.gen::<f32>() < 0.25 {
                commands.entity(entity).insert(Alive);
                material.0 = handles.alive_material.clone();
            }
        }
    }
}

fn spread_livlihood(
    mut neighbors: QueryByIndex<Chunk, (&Position, &mut LivingNeighbors)>,
    living: Query<(&Position, &Chunk), With<Alive>>,
) {
    for (this, Chunk(cx, cy)) in living.iter() {
        let mut found = 0;

        'cell: for dx in [0, -1, 1] {
            for dy in [0, -1, 1] {
                let mut lens = neighbors.at_mut(&Chunk(cx + dx, cy + dy));
                let mut query = lens.query();

                for (other, mut count) in query.iter_mut() {
                    let diff_x = this.0.abs_diff(other.0);
                    let diff_y = this.1.abs_diff(other.1);

                    let is_self = diff_x == 0 && diff_y == 0;
                    let is_adjacent = diff_x < 2 && diff_y < 2;

                    if is_adjacent && !is_self {
                        count.0 += 1;
                        found += 1;

                        if found == 8 {
                            break 'cell;
                        }

                        if count.0 > 8 {
                            panic!("Wait how did that happen???");
                        }
                    }
                }
            }
        }
    }
}

fn update_state(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut LivingNeighbors,
        Has<Alive>,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
    handles: Res<LivingHandles>,
) {
    for (entity, mut count, alive, mut color) in query.iter_mut() {
        match count.0 {
            0 | 1 | 3.. if alive => {
                commands.entity(entity).remove::<Alive>();
                color.0 = handles.dead_material.clone();
            }
            3 if !alive => {
                commands.entity(entity).insert(Alive);
                color.0 = handles.alive_material.clone();
            }
            _ => {}
        }

        // Reset for next frame
        count.0 = 0;
    }
}
