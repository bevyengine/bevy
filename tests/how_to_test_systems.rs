use bevy::prelude::*;

#[derive(Component, Default)]
struct Enemy {
    hit_points: u32,
}

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

#[test]
fn did_hurt_enemy() {
    // Setup world
    let mut world = World::default();

    // Setup stage with our two systems
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(hurt_enemies.before("death"));
    update_stage.add_system(despawn_dead_enemies.label("death"));

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
    update_stage.add_system(hurt_enemies.before("death"));
    update_stage.add_system(despawn_dead_enemies.label("death"));

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
    update_stage.add_system(spawn_enemy);

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
