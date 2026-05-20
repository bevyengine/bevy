//! Saves restricted components when a save request is active.

use bevy::prelude::*;

#[derive(RestrictedAccess)]
struct CharacterRecord {
    name: &'static str,
    level: u32,
    dirty: bool,
}

#[derive(Resource)]
struct SaveRequested(bool);

#[derive(Resource, Default)]
struct SaveBuffer(Vec<String>);

fn main() {
    App::new()
        .insert_resource(SaveRequested(true))
        .init_resource::<SaveBuffer>()
        .add_systems(Startup, setup)
        .add_systems(Update, (save_dirty_records, verify_save).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(CharacterRecord {
        name: "Ari",
        level: 7,
        dirty: true,
    });
    commands.spawn(CharacterRecord {
        name: "Bo",
        level: 4,
        dirty: false,
    });
}

fn save_dirty_records(
    requested: Res<SaveRequested>,
    mut records: RestrictedMut<CharacterRecord>,
    mut save_buffer: ResMut<SaveBuffer>,
) {
    if !requested.0 {
        return;
    }

    for mut record in records.iter_mut() {
        if record.dirty {
            save_buffer
                .0
                .push(format!("{}:level={}", record.name, record.level));
            record.dirty = false;
        }
    }
}

fn verify_save(records: Query<&CharacterRecord>, save_buffer: Res<SaveBuffer>) {
    assert_eq!(save_buffer.0, vec![String::from("Ari:level=7")]);
    assert!(records.iter().all(|record| !record.dirty));
}
