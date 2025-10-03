//! Demonstrates how to observe events: both component lifecycle events and custom events.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SpatialIndex>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_shapes, handle_click))
        // Observers are systems that run when an event is "triggered". This observer runs whenever
        // `ExplodeMines` is triggered.
        .add_observer(
            |explode_mines: On<ExplodeMines>,
             mines: Query<&Mine>,
             index: Res<SpatialIndex>,
             mut commands: Commands| {
                // Access resources
                for entity in index.get_nearby(explode_mines.pos) {
                    // Run queries
                    let mine = mines.get(entity).unwrap();
                    if mine.pos.distance(explode_mines.pos) < mine.size + explode_mines.radius {
                        // And queue commands, including triggering additional events
                        // Here we trigger the `Explode` event for entity `e`
                        commands.trigger(Explode { entity });
                    }
                }
            },
        )
        // This observer runs whenever the `Mine` component is added to an entity, and places it in a simple spatial index.
        .add_observer(on_add_mine)
        // This observer runs whenever the `Mine` component is removed from an entity (including despawning it)
        // and removes it from the spatial index.
        .add_observer(on_remove_mine)
        .run();
}

#[derive(Component)]
struct Mine {
    pos: Vec2,
    size: f32,
}

impl Mine {
    fn random(rand: &mut ChaCha8Rng) -> Self {
        Mine {
            pos: Vec2::new(
                (rand.random::<f32>() - 0.5) * 1200.0,
                (rand.random::<f32>() - 0.5) * 600.0,
            ),
            size: 4.0 + rand.random::<f32>() * 16.0,
        }
    }
}

/// This is a normal [`Event`]. Any observer that watches for it will run when it is triggered.
#[derive(Event)]
struct ExplodeMines {
    pos: Vec2,
    radius: f32,
}

/// An [`EntityEvent`] is a specialized type of [`Event`] that can target a specific entity. In addition to
/// running normal "top level" observers when it is triggered (which target _any_ entity that Explodes), it will
/// also run any observers that target the _specific_ entity for that event.
#[derive(EntityEvent)]
struct Explode {
    entity: Entity,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new(
            "Click on a \"Mine\" to trigger it.\n\
            When it explodes it will trigger all overlapping mines.",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    let mut rng = ChaCha8Rng::seed_from_u64(19878367467713);

    commands
        .spawn(Mine::random(&mut rng))
        // Observers can watch for events targeting a specific entity.
        // This will create a new observer that runs whenever the Explode event
        // is triggered for this spawned entity.
        .observe(explode_mine);

    // We want to spawn a bunch of mines. We could just call the code above for each of them.
    // That would create a new observer instance for every Mine entity. Having duplicate observers
    // generally isn't worth worrying about as the overhead is low. But if you want to be maximally efficient,
    // you can reuse observers across entities.
    //
    // First, observers are actually just entities with the Observer component! The `observe()` functions
    // you've seen so far in this example are just shorthand for manually spawning an observer.
    let mut observer = Observer::new(explode_mine);

    // As we spawn entities, we can make this observer watch each of them:
    for _ in 0..1000 {
        let entity = commands.spawn(Mine::random(&mut rng)).id();
        observer.watch_entity(entity);
    }

    // By spawning the Observer component, it becomes active!
    commands.spawn(observer);
}

fn on_add_mine(add: On<Add, Mine>, query: Query<&Mine>, mut index: ResMut<SpatialIndex>) {
    let mine = query.get(add.entity).unwrap();
    let tile = (
        (mine.pos.x / CELL_SIZE).floor() as i32,
        (mine.pos.y / CELL_SIZE).floor() as i32,
    );
    index.map.entry(tile).or_default().insert(add.entity);
}

// Remove despawned mines from our index
fn on_remove_mine(remove: On<Remove, Mine>, query: Query<&Mine>, mut index: ResMut<SpatialIndex>) {
    let mine = query.get(remove.entity).unwrap();
    let tile = (
        (mine.pos.x / CELL_SIZE).floor() as i32,
        (mine.pos.y / CELL_SIZE).floor() as i32,
    );
    index.map.entry(tile).and_modify(|set| {
        set.remove(&remove.entity);
    });
}

fn explode_mine(explode: On<Explode>, query: Query<&Mine>, mut commands: Commands) {
    // Explode is an EntityEvent. `explode.entity` is the entity that Explode was triggered for.
    let Ok(mut entity) = commands.get_entity(explode.entity) else {
        return;
    };
    info!("Boom! {} exploded.", explode.entity);
    entity.despawn();
    let mine = query.get(explode.entity).unwrap();
    // Trigger another explosion cascade.
    commands.trigger(ExplodeMines {
        pos: mine.pos,
        radius: mine.size,
    });
}

// Draw a circle for each mine using `Gizmos`
fn draw_shapes(mut gizmos: Gizmos, mines: Query<&Mine>) {
    for mine in &mines {
        gizmos.circle_2d(
            mine.pos,
            mine.size,
            Color::hsl((mine.size - 4.0) / 16.0 * 360.0, 1.0, 0.8),
        );
    }
}

// Trigger `ExplodeMines` at the position of a given click
fn handle_click(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let Ok(windows) = windows.single() else {
        return;
    };

    let (camera, camera_transform) = *camera;
    if let Some(pos) = windows
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
        && mouse_button_input.just_pressed(MouseButton::Left)
    {
        commands.trigger(ExplodeMines { pos, radius: 1.0 });
    }
}

#[derive(Resource, Default)]
struct SpatialIndex {
    map: HashMap<(i32, i32), HashSet<Entity>>,
}

/// Cell size has to be bigger than any `TriggerMine::radius`
const CELL_SIZE: f32 = 64.0;

impl SpatialIndex {
    // Lookup all entities within adjacent cells of our spatial index
    fn get_nearby(&self, pos: Vec2) -> Vec<Entity> {
        let tile = (
            (pos.x / CELL_SIZE).floor() as i32,
            (pos.y / CELL_SIZE).floor() as i32,
        );
        let mut nearby = Vec::new();
        for x in -1..2 {
            for y in -1..2 {
                if let Some(mines) = self.map.get(&(tile.0 + x, tile.1 + y)) {
                    nearby.extend(mines.iter());
                }
            }
        }
        nearby
    }
}
