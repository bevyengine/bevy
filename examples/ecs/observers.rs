//! Demonstrates how to observe life-cycle triggers as well as define custom ones.

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
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
        .observe(
            |trigger: Trigger<ExplodeMines>,
             mines: Query<&Mine>,
             index: Res<SpatialIndex>,
             mut commands: Commands| {
                // You can access the trigger data via the `Observer`
                let event = trigger.event();
                // Access resources
                for e in index.get_nearby(event.pos) {
                    // Run queries
                    let mine = mines.get(e).unwrap();
                    if mine.pos.distance(event.pos) < mine.size + event.radius {
                        // And queue commands, including triggering additional events
                        // Here we trigger the `Explode` event for entity `e`
                        commands.trigger_targets(Explode, e);
                    }
                }
            },
        )
        // This observer runs whenever the `Mine` component is added to an entity, and places it in a simple spatial index.
        .observe(on_add_mine)
        // This observer runs whenever the `Mine` component is removed from an entity (including despawning it)
        // and removes it from the spatial index.
        .observe(on_remove_mine)
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
                (rand.gen::<f32>() - 0.5) * 1200.0,
                (rand.gen::<f32>() - 0.5) * 600.0,
            ),
            size: 4.0 + rand.gen::<f32>() * 16.0,
        }
    }
}

#[derive(Event)]
struct ExplodeMines {
    pos: Vec2,
    radius: f32,
}

#[derive(Event)]
struct Explode;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_section(
            "Click on a \"Mine\" to trigger it.\n\
            When it explodes it will trigger all overlapping mines.",
            TextStyle {
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        }),
    );

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

fn on_add_mine(
    trigger: Trigger<OnAdd, Mine>,
    query: Query<&Mine>,
    mut index: ResMut<SpatialIndex>,
) {
    let mine = query.get(trigger.entity()).unwrap();
    let tile = (
        (mine.pos.x / CELL_SIZE).floor() as i32,
        (mine.pos.y / CELL_SIZE).floor() as i32,
    );
    index.map.entry(tile).or_default().insert(trigger.entity());
}

// Remove despawned mines from our index
fn on_remove_mine(
    trigger: Trigger<OnRemove, Mine>,
    query: Query<&Mine>,
    mut index: ResMut<SpatialIndex>,
) {
    let mine = query.get(trigger.entity()).unwrap();
    let tile = (
        (mine.pos.x / CELL_SIZE).floor() as i32,
        (mine.pos.y / CELL_SIZE).floor() as i32,
    );
    index.map.entry(tile).and_modify(|set| {
        set.remove(&trigger.entity());
    });
}

fn explode_mine(trigger: Trigger<Explode>, query: Query<&Mine>, mut commands: Commands) {
    // If a triggered event is targeting a specific entity you can access it with `.entity()`
    let id = trigger.entity();
    let Some(mut entity) = commands.get_entity(id) else {
        return;
    };
    info!("Boom! {:?} exploded.", id.index());
    entity.despawn();
    let mine = query.get(id).unwrap();
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
    camera: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let (camera, camera_transform) = camera.single();
    if let Some(pos) = windows
        .single()
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        if mouse_button_input.just_pressed(MouseButton::Left) {
            commands.trigger(ExplodeMines { pos, radius: 1.0 });
        }
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
