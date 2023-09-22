use bevy::{ecs::event::Events, prelude::*};

#[derive(Component, Default)]
struct Enemy {
    hit_points: u32,
    score_value: u32,
}

#[derive(Event)]
struct EnemyDied(u32);

#[derive(Resource)]
struct Score(u32);

fn update_score(mut dead_enemies: EventReader<EnemyDied>, mut score: ResMut<Score>) {
    for value in dead_enemies.read() {
        score.0 += value.0;
    }
}

fn despawn_dead_enemies(
    mut commands: Commands,
    mut dead_enemies: EventWriter<EnemyDied>,
    enemies: Query<(Entity, &Enemy)>,
) {
    for (entity, enemy) in &enemies {
        if enemy.hit_points == 0 {
            commands.entity(entity).despawn_recursive();
            dead_enemies.send(EnemyDied(enemy.score_value));
        }
    }
}

fn hurt_enemies(mut enemies: Query<&mut Enemy>) {
    for mut enemy in &mut enemies {
        enemy.hit_points -= 1;
    }
}

fn spawn_enemy(mut commands: Commands, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        commands.spawn(Enemy {
            hit_points: 5,
            score_value: 3,
        });
    }
}

#[test]
fn did_hurt_enemy() {
    // Setup app
    let mut app = App::new();

    // Add Score resource
    app.insert_resource(Score(0));

    // Add `EnemyDied` event
    app.add_event::<EnemyDied>();

    // Add our two systems
    app.add_systems(Update, (hurt_enemies, despawn_dead_enemies).chain());

    // Setup test entities
    let enemy_id = app
        .world
        .spawn(Enemy {
            hit_points: 5,
            score_value: 3,
        })
        .id();

    // Run systems
    app.update();

    // Check resulting changes
    assert!(app.world.get::<Enemy>(enemy_id).is_some());
    assert_eq!(app.world.get::<Enemy>(enemy_id).unwrap().hit_points, 4);
}

#[test]
fn did_despawn_enemy() {
    // Setup app
    let mut app = App::new();

    // Add Score resource
    app.insert_resource(Score(0));

    // Add `EnemyDied` event
    app.add_event::<EnemyDied>();

    // Add our two systems
    app.add_systems(Update, (hurt_enemies, despawn_dead_enemies).chain());

    // Setup test entities
    let enemy_id = app
        .world
        .spawn(Enemy {
            hit_points: 1,
            score_value: 1,
        })
        .id();

    // Run systems
    app.update();

    // Check enemy was despawned
    assert!(app.world.get::<Enemy>(enemy_id).is_none());

    // Get `EnemyDied` event reader
    let enemy_died_events = app.world.resource::<Events<EnemyDied>>();
    let mut enemy_died_reader = enemy_died_events.get_reader();
    let enemy_died = enemy_died_reader.read(enemy_died_events).next().unwrap();

    // Check the event has been sent
    assert_eq!(enemy_died.0, 1);
}

#[test]
fn spawn_enemy_using_input_resource() {
    // Setup app
    let mut app = App::new();

    // Add our systems
    app.add_systems(Update, spawn_enemy);

    // Setup test resource
    let mut input = Input::<KeyCode>::default();
    input.press(KeyCode::Space);
    app.insert_resource(input);

    // Run systems
    app.update();

    // Check resulting changes, one entity has been spawned with `Enemy` component
    assert_eq!(app.world.query::<&Enemy>().iter(&app.world).len(), 1);

    // Clear the `just_pressed` status for all `KeyCode`s
    app.world.resource_mut::<Input<KeyCode>>().clear();

    // Run systems
    app.update();

    // Check resulting changes, no new entity has been spawned
    assert_eq!(app.world.query::<&Enemy>().iter(&app.world).len(), 1);
}

#[test]
fn update_score_on_event() {
    // Setup app
    let mut app = App::new();

    // Add Score resource
    app.insert_resource(Score(0));

    // Add `EnemyDied` event
    app.add_event::<EnemyDied>();

    // Add our systems
    app.add_systems(Update, update_score);

    // Send an `EnemyDied` event
    app.world
        .resource_mut::<Events<EnemyDied>>()
        .send(EnemyDied(3));

    // Run systems
    app.update();

    // Check resulting changes
    assert_eq!(app.world.resource::<Score>().0, 3);
}
