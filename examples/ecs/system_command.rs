use bevy::{prelude::*, utils::Duration};

/// This example triggers a system from a command
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(spawn)
        .add_system(trigger_sync)
        .run();
}

pub struct Player;

/// Spawn some players to sync
fn spawn(mut commands: Commands) {
    commands.spawn().insert(Player);
    commands.spawn().insert(Player);
}

fn trigger_sync(mut commands: Commands, mut last_sync: Local<f64>, time: Res<Time>) {
    if time.seconds_since_startup() - *last_sync > 5.0 {
        commands
            .run_system(sync_system.config(|config| config.1 = Some(time.time_since_startup())));
        *last_sync = time.seconds_since_startup();
    }
}

/// As this system is run through a command, it will run at the end of the current stage.
fn sync_system(players: Query<&Player>, since_startup: Local<Duration>) {
    for _player in players.iter() {
        // do the sync
    }
    info!(
        "synced: {} players ({:?})",
        players.iter().len(),
        *since_startup
    );
}
