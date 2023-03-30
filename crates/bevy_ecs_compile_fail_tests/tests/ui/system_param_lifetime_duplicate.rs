use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;

#[allow(dead_code)]
#[derive(Debug, Resource)]
struct PlayerName(String);

// must fail to compile
#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct DuplicateWorldLifetime<'w, 'world> {
    player_name: Res<'w, PlayerName>,
}

// must fail to compile
#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct DuplicateStateLifetime<'s, 'cache> {
    player_name: Local<'cache, String>,
}
