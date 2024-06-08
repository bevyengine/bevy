//! Demonstrates how to observe life-cycle triggers as well as define custom ones.

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SpatialIndex>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_shapes, handle_click))
        // Observers run when a certain event is triggered. This observer runs whenever `ExplodeMines` is triggered.
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
        .observe(add_mine)
        // Since observers run systems you can also define them as standalone functions rather than closures.
        // This observer runs whenever the `Mine` component is removed from an entity (including despawning it)
        // and removes it from the spatial index.
        .observe(remove_mine)
        .run();
}

#[derive(Component)]
struct Mine {
    pos: Vec2,
    size: f32,
}

#[derive(Event)]
struct ExplodeMines {
    pos: Vec2,
    radius: f32,
}

#[derive(Event)]
struct Explode;

fn setup(world: &mut World) {
    world.spawn(Camera2dBundle::default());

    let font = world
        .resource::<AssetServer>()
        .load("fonts/FiraMono-Medium.ttf");
    world.spawn(TextBundle::from_section(
        "Click on a \"Mine\" to trigger it.\n\
            When it explodes it will trigger all overlapping mines.",
        TextStyle {
            font,
            font_size: 24.,
            color: Color::WHITE,
        },
    ));

    // Observers can also listen for triggers for a specific component.

    // Now we spawn a set of random mines.
    for _ in 0..1000 {
        world
            .spawn(Mine {
                pos: Vec2::new(
                    (rand::random::<f32>() - 0.5) * 1200.0,
                    (rand::random::<f32>() - 0.5) * 600.0,
                ),
                size: 4.0 + rand::random::<f32>() * 16.0,
            })
            // Observers can also listen to events targeting a specific entity.
            // This observer listens to `Explode` event triggers targeted at our mine.
            .observe(
                |trigger: Trigger<Explode>, query: Query<&Mine>, mut commands: Commands| {
                    // If a triggered event is targeting a specific entity you can access it with `.source()`
                    let source = trigger.source();
                    let Some(mut entity) = commands.get_entity(source) else {
                        return;
                    };
                    println!("Boom! {:?} exploded.", source.index());
                    entity.despawn();
                    let mine = query.get(source).unwrap();
                    // Fire another trigger to cascade into other mines.
                    commands.trigger(ExplodeMines {
                        pos: mine.pos,
                        radius: mine.size,
                    });
                },
            );
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

fn add_mine(trigger: Trigger<OnAdd, Mine>, query: Query<&Mine>, mut index: ResMut<SpatialIndex>) {
    let mine = query.get(trigger.source()).unwrap();
    let tile = (
        (mine.pos.x / CELL_SIZE).floor() as i32,
        (mine.pos.y / CELL_SIZE).floor() as i32,
    );
    index.map.entry(tile).or_default().insert(trigger.source());
}

// Remove despawned mines from our index
fn remove_mine(
    trigger: Trigger<OnRemove, Mine>,
    query: Query<&Mine>,
    mut index: ResMut<SpatialIndex>,
) {
    let mine = query.get(trigger.source()).unwrap();
    let tile = (
        (mine.pos.x / CELL_SIZE).floor() as i32,
        (mine.pos.y / CELL_SIZE).floor() as i32,
    );
    index.map.entry(tile).and_modify(|set| {
        set.remove(&trigger.source());
    });
}

// Draw a circle for each mine using `Gizmos`
fn draw_shapes(mut gizmos: Gizmos, mines: Query<&Mine>) {
    for mine in mines.iter() {
        gizmos.circle_2d(
            mine.pos,
            mine.size,
            Color::hsl((mine.size - 4.0) / 16.0 * 360.0, 1.0, 0.8),
        );
    }
}

// Fire an initial `ExplodeMines` trigger on click
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
