use bevy::{
    math::vec2,
    prelude::*,
    utils::{Duration, Instant},
};

#[derive(Default)]
struct Enemy {
    hit_points: u32,
}

#[derive(Default)]
struct Velocity(Vec2);

#[derive(Default, Clone, Copy)]
struct Position(Vec2);

fn despawn_dead_enemies(mut commands: Commands, enemies: Query<(Entity, &Enemy)>) {
    for (entity, enemy) in enemies.iter() {
        if enemy.hit_points == 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn hurt_enemies(mut enemies: Query<&mut Enemy>) {
    for mut enemy in enemies.iter_mut() {
        enemy.hit_points -= 1;
    }
}

fn spawn_enemy(mut commands: Commands, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        commands.spawn().insert(Enemy { hit_points: 5 });
    }
}

fn update_position(time: Res<Time>, mut units: Query<(&Velocity, &mut Position)>) {
    for (velocity, mut position) in units.iter_mut() {
        position.0 += velocity.0 * time.delta_seconds();
    }
}

#[test]
fn did_hurt_enemy() {
    // Setup world
    let mut world = World::default();

    // Setup stage with our two systems
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(hurt_enemies.system().before("death"));
    update_stage.add_system(despawn_dead_enemies.system().label("death"));

    // Setup test entities
    let enemy_id = world.spawn().insert(Enemy { hit_points: 5 }).id();

    // Run systems
    update_stage.run(&mut world);

    // Check resulting changes
    assert!(world.get::<Enemy>(enemy_id).is_some());
    assert_eq!(world.get::<Enemy>(enemy_id).unwrap().hit_points, 4);
}

#[test]
fn did_despawn_enemy() {
    // Setup world
    let mut world = World::default();

    // Setup stage with our two systems
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(hurt_enemies.system().before("death"));
    update_stage.add_system(despawn_dead_enemies.system().label("death"));

    // Setup test entities
    let enemy_id = world.spawn().insert(Enemy { hit_points: 1 }).id();

    // Run systems
    update_stage.run(&mut world);

    // Check resulting changes
    assert!(world.get::<Enemy>(enemy_id).is_none());
}

#[test]
fn spawn_enemy_using_input_resource() {
    // Setup world
    let mut world = World::default();

    // Setup stage with a system
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(spawn_enemy.system());

    // Setup test resource
    let mut input = Input::<KeyCode>::default();
    input.press(KeyCode::Space);
    world.insert_resource(input);

    // Run systems
    update_stage.run(&mut world);

    // Check resulting changes, one entity has been spawned with `Enemy` component
    assert_eq!(world.query::<&Enemy>().iter(&world).len(), 1);

    // Clear the `just_pressed` status for all `KeyCode`s
    world.get_resource_mut::<Input<KeyCode>>().unwrap().clear();

    // Run systems
    update_stage.run(&mut world);

    // Check resulting changes, no new entity has been spawned
    assert_eq!(world.query::<&Enemy>().iter(&world).len(), 1);
}

#[test]
fn confirm_system_is_framerate_independent() {
    // Setup world
    let mut world = World::default();

    // Setup stage with a system
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(update_position.system());

    // Closure that gets the resulting position for a certain fps
    let mut test_fps = |fps: u32| -> Position {
        // The frame time delta we want to simulate
        let delta = Duration::from_secs_f32(1.0 / fps as f32);

        // Setup test entities
        let entity = world
            .spawn()
            .insert_bundle((Velocity(vec2(1.0, 0.0)), Position(vec2(0.0, 0.0))))
            .id();

        // Setup test resource
        let time = Time::default();
        world.insert_resource(time);

        // Set initial time
        let mut time = world.get_resource_mut::<Time>().unwrap();
        let initial_instant = Instant::now();
        time.update_with_instant(initial_instant);

        // Simulate one second
        for i in 0..fps + 1 {
            // Update time
            let mut time = world.get_resource_mut::<Time>().unwrap();
            time.update_with_instant(initial_instant + delta * i);

            // Run systems
            update_stage.run(&mut world);
        }

        // Remove mocked Time
        world.remove_resource::<Time>();

        // Return resulting position
        *world.get_entity(entity).unwrap().get::<Position>().unwrap()
    };

    // Test at 30 and 60 fps
    let result_30fps = test_fps(30);
    let result_60fps = test_fps(60);

    // Calculate the difference
    let difference = (result_30fps.0 - result_60fps.0).length();

    // A tiny difference is expected due to f32 precision
    assert!(difference < f32::EPSILON * 10.0);
}
