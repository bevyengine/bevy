use bevy::{
    ecs::{
        query::{FilterFetch, WorldQuery},
        system::SystemParam,
    },
    prelude::*,
};

/// This example creates a [`SystemParam`] struct that counts the number of players
fn main() {
    App::new()
        .insert_resource(PlayerCount(0))
        .add_startup_system(spawn)
        .add_system(count_players)
        .run();
}

#[derive(Component)]
pub struct Player;
pub struct PlayerCount(usize);

/// The [`SystemParam`] struct can contain any types that can also be included in a
/// system function signature.
///
/// In this example, it includes a query and a mutable resource.
#[derive(SystemParam)]
struct PlayerCounter<'w, 's> {
    players: Query<'w, 's, &'static Player>,
    count: ResMut<'w, PlayerCount>,
}

#[derive(SystemParam)]
pub struct MySystemParam<
    'w,
    's,
    Q: 'static + WorldQuery + Send + Sync,
    F: 'static + WorldQuery + Send + Sync,
> where
    F::Fetch: FilterFetch,
{
    _query: Query<'w, 's, Q, F>,
}

impl<'w, 's> PlayerCounter<'w, 's> {
    fn count(&mut self) {
        self.count.0 = self.players.iter().len();
    }
}

/// Spawn some players to count
fn spawn(mut commands: Commands) {
    commands.spawn().insert(Player);
    commands.spawn().insert(Player);
    commands.spawn().insert(Player);
}

/// The [`SystemParam`] can be used directly in a system argument.
fn count_players(mut counter: PlayerCounter, _p: MySystemParam<&'static Player, ()>) {
    counter.count();

    println!("{} players in the game", counter.count.0);
}
