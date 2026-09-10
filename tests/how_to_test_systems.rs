#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
use bevy::prelude::*;

#[derive(Component, Default)]
struct Enemy {
    hit_points: u32,
    score_value: u32,
}

#[derive(Message)]
struct EnemyDied(u32);

#[derive(Resource)]
struct Score(u32);

fn update_score(mut dead_enemies: MessageReader<EnemyDied>, mut score: ResMut<Score>) {
    for value in dead_enemies.read() {
        score.0 += value.0;
    }
}

fn despawn_dead_enemies(
    mut commands: Commands,
    mut dead_enemies: MessageWriter<EnemyDied>,
    enemies: Query<(Entity, &Enemy)>,
) {
    for (entity, enemy) in &enemies {
        if enemy.hit_points == 0 {
            commands.entity(entity).despawn();
            dead_enemies.write(EnemyDied(enemy.score_value));
        }
    }
}

fn hurt_enemies(mut enemies: Query<&mut Enemy>) {
    for mut enemy in &mut enemies {
        enemy.hit_points -= 1;
    }
}

fn spawn_enemy(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>) {
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
    app.add_message::<EnemyDied>();

    // Add our two systems
    app.add_systems(Update, (hurt_enemies, despawn_dead_enemies).chain());

    // Setup test entities
    let enemy_id = app
        .world_mut()
        .spawn(Enemy {
            hit_points: 5,
            score_value: 3,
        })
        .id();

    // Run systems
    app.update();

    // Check resulting changes
    assert!(app.world().get::<Enemy>(enemy_id).is_some());
    assert_eq!(app.world().get::<Enemy>(enemy_id).unwrap().hit_points, 4);
}

#[test]
fn did_despawn_enemy() {
    // Setup app
    let mut app = App::new();

    // Add Score resource
    app.insert_resource(Score(0));

    // Add `EnemyDied` event
    app.add_message::<EnemyDied>();

    // Add our two systems
    app.add_systems(Update, (hurt_enemies, despawn_dead_enemies).chain());

    // Setup test entities
    let enemy_id = app
        .world_mut()
        .spawn(Enemy {
            hit_points: 1,
            score_value: 1,
        })
        .id();

    // Run systems
    app.update();

    // Check enemy was despawned
    assert!(app.world().get::<Enemy>(enemy_id).is_none());

    // Get `EnemyDied` message reader
    let enemy_died_messages = app.world().resource::<Messages<EnemyDied>>();
    let mut enemy_died_cursor = enemy_died_messages.get_cursor();
    let enemy_died = enemy_died_cursor.read(enemy_died_messages).next().unwrap();

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
    let mut input = ButtonInput::<KeyCode>::default();
    input.press(KeyCode::Space);
    app.insert_resource(input);

    // Run systems
    app.update();

    // Check resulting changes, one entity has been spawned with `Enemy` component
    assert_eq!(app.world_mut().query::<&Enemy>().iter(app.world()).len(), 1);

    // Clear the `just_pressed` status for all `KeyCode`s
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();

    // Run systems
    app.update();

    // Check resulting changes, no new entity has been spawned
    assert_eq!(app.world_mut().query::<&Enemy>().iter(app.world()).len(), 1);
}

#[test]
fn update_score_on_event() {
    // Setup app
    let mut app = App::new();

    // Add Score resource
    app.insert_resource(Score(0));

    // Add `EnemyDied` message
    app.add_message::<EnemyDied>();

    // Add our systems
    app.add_systems(Update, update_score);

    // Write an `EnemyDied` event
    app.world_mut()
        .resource_mut::<Messages<EnemyDied>>()
        .write(EnemyDied(3));

    // Run systems
    app.update();

    // Check resulting changes
    assert_eq!(app.world().resource::<Score>().0, 3);
}
