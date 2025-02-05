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
        .add_index(IndexOptions::<Chunk>::default())
        .insert_resource(Time::<Fixed>::from_seconds(0.1))
        .add_systems(Startup, setup)
        .add_systems(Update, randomly_revive)
        .add_systems(FixedUpdate, (spread_livelihood, update_state).chain())
        .run();
}

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[component(immutable, storage = "SparseSet")]
struct Alive;

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[component(immutable)]
struct Position(i8, i8);

/// To increase cache performance, we group a 3x3 square of cells into a chunk, and index against that.
/// If you instead indexed against the [`Position`] directly, you would have a single archetype per tile,
/// massively decreasing query performance.
#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[component(immutable)]
struct Chunk(i8, i8);

impl From<Position> for Chunk {
    fn from(Position(x, y): Position) -> Self {
        Chunk(x / 3, y / 3)
    }
}

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct LivingNeighbors(u8);

#[derive(Resource)]
struct LivingHandles {
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
        alive_material: alive.clone(),
        dead_material: dead.clone(),
    });

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    commands.insert_resource(SeededRng(seeded_rng));

    // Spawn the cells
    commands.spawn_batch(
        (-30..30)
            .flat_map(|x| (-30..30).map(move |y| Position(x, y)))
            .map(move |position| {
                (
                    position,
                    Chunk::from(position),
                    LivingNeighbors(0),
                    Mesh2d(rect.clone()),
                    MeshMaterial2d(dead.clone()),
                    Transform::from_xyz(
                        11. * position.0 as f32 + 5.5,
                        11. * position.1 as f32 + 5.5 - 20.,
                        0.,
                    ),
                )
            }),
    );
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

fn spread_livelihood(
    mut neighbors_by_chunk: QueryByIndex<Chunk, (&Position, &mut LivingNeighbors)>,
    living: Query<&Position, With<Alive>>,
) {
    /// In Conway's Game of Life, we consider the adjacent cells (including diagonals) as our neighbors:
    ///
    /// ```no_run
    /// O O O O O
    /// O N N N O
    /// O N X N O
    /// O N N N O
    /// O O O O O
    /// ```
    ///
    /// In the above diagram, if `X` denotes a particular cell, `N` denotes neighbors, while `O` denotes a non-neighbor cell.
    const NEIGHBORS: [Position; 8] = [
        // Position(0, 0), // Excluded, as this is us!
        Position(0, 1),
        Position(0, -1),
        Position(1, 0),
        Position(1, 1),
        Position(1, -1),
        Position(-1, 0),
        Position(-1, 1),
        Position(-1, -1),
    ];

    for this in living.iter() {
        for delta in NEIGHBORS {
            let other_pos = Position(this.0 + delta.0, this.1 + delta.1);
            let mut lens = neighbors_by_chunk.at_mut(&Chunk::from(other_pos));
            let mut query = lens.query();

            let Some((_, mut count)) = query.iter_mut().find(|(&pos, _)| pos == other_pos) else {
                continue;
            };

            count.0 += 1;
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
