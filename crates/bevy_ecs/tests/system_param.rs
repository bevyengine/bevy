use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;

#[allow(dead_code)]
#[derive(Debug, Resource)]
struct PlayerName(String);

#[derive(Debug, Component)]
struct Companion;

#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct UsualLifetime<'w> {
    player_name: Res<'w, PlayerName>,
}

#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct UsualLifetimes<'w, 's> {
    player_name: Res<'w, PlayerName>,
    companions: Query<'w, 's, &'static Companion>,
}

#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct UnusualLifetime<'world> {
    player_name: Res<'world, PlayerName>,
}

#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct UnusualLifetimes<'world, 'state> {
    player_name: Res<'world, PlayerName>,
    companions: Query<'world, 'state, &'static Companion>,
}

#[allow(dead_code)]
#[derive(Debug, SystemParam)]
struct MixLifetimes<'w, 'state> {
    player_name: Res<'w, PlayerName>,
    companions: Query<'w, 'state, &'static Companion>,
}

// must fail to compile
// #[allow(dead_code)]
// #[derive(Debug, SystemParam)]
// struct DuplicateWorldLifetime<'w, 'world> {
//     player_name: Res<'w, PlayerName>
// }

// must fail to compile
// #[allow(dead_code)]
// #[derive(Debug, SystemParam)]
// struct DuplicateStateLifetime<'s, 'cache> {
//     player_name: Local<'cache, String>
// }
