use std::collections::HashMap;

use bevy::{ecs::system::CommandQueue, prelude::*};

struct Enemy {
    hit_points: u32,
}

struct CharacterTemplate {
    hit_points: HashMap<&'static str, u32>,
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

fn spawn_enemy(mut commands: Commands, character_template: Res<CharacterTemplate>) {
    commands.spawn().insert(Enemy {
        hit_points: *character_template.hit_points.get("enemy").unwrap(),
    });
}

#[test]
fn did_hurt_enemy() {
    // Setup world and commands
    let mut world = World::default();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);

    // Setup stage with our two systems
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(hurt_enemies.system().before("death"));
    update_stage.add_system(despawn_dead_enemies.system().label("death"));
    let mut schedule = Schedule::default();
    schedule.add_stage("update", update_stage);

    // Setup test entities
    let ennemy_id = commands.spawn().insert(Enemy { hit_points: 5 }).id();
    queue.apply(&mut world);

    // Run systems
    schedule.run(&mut world);

    // Check resulting changes
    assert!(world.get::<Enemy>(ennemy_id).is_some());
    assert_eq!(world.get::<Enemy>(ennemy_id).unwrap().hit_points, 4);
}

#[test]
fn did_despawn_enemy() {
    // Setup world and commands
    let mut world = World::default();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);

    // Setup stage with our two systems
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(hurt_enemies.system().before("death"));
    update_stage.add_system(despawn_dead_enemies.system().label("death"));
    let mut schedule = Schedule::default();
    schedule.add_stage("update", update_stage);

    // Setup test entities
    let ennemy_id = commands.spawn().insert(Enemy { hit_points: 1 }).id();
    queue.apply(&mut world);

    // Run systems
    schedule.run(&mut world);

    // Check resulting changes
    assert!(world.get::<Enemy>(ennemy_id).is_none());
}

#[test]
fn spawned_from_resource() {
    // Setup world and commands
    let mut world = World::default();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);

    // Setup stage with a system
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(spawn_enemy.system());
    let mut schedule = Schedule::default();
    schedule.add_stage("update", update_stage);

    // Setup test resource
    let mut hit_points = HashMap::new();
    hit_points.insert("enemy", 25);
    commands.insert_resource(CharacterTemplate { hit_points });
    queue.apply(&mut world);

    // Run systems
    schedule.run(&mut world);

    // Check resulting changes
    let mut query = world.query::<&Enemy>();
    let results = query
        .iter(&world)
        .map(|enemy| enemy.hit_points)
        .collect::<Vec<_>>();
    assert_eq!(results, vec![25]);
}
