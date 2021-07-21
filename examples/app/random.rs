use bevy::prelude::*;
use rand::{prelude::IteratorRandom, rngs::SmallRng, Rng, SeedableRng};

/// This example illustrates how to use entropy to control randomness in bevy.
/// We randomly choose a coin to toss and record the result.
///
/// Because all random number generators are seeded from the same "world seed",
/// this program is deterministic.
struct Coin;

struct Toss(SmallRng);

#[derive(Debug, PartialEq)]
enum Face {
    Heads,
    Tails,
}

fn main() {
    // This is a bad seed but is used for illustrative purposes.
    //
    // Normally you would do one of the following:
    //   1. Get a good random seed out-of-band and hardcode it in the source.
    //   2. Dynamically call to the OS and print the seed so the user can rerun
    //      deterministically.
    //   3. Dynamically call to the OS and share the seed with the server so the
    //      client and server deterministically execute together.
    //   4. Load the seed from a server so the client and server deterministically
    //      execute together.
    let world_seed: [u8; 32] = [1; 32];

    App::build()
        // Delete this to use the default os-provided entropy source.
        // Note that doing so introduces non-determinism.
        .insert_resource(Entropy::from(world_seed))
        .add_plugins(DefaultPlugins)
        .add_startup_system(coin_chooser.system())
        .add_startup_system(spawn_coins.system())
        .add_system(toss_coin.system())
        .run();
}

// System to create a single random coin chooser.
fn coin_chooser(mut commands: Commands, mut entropy: ResMut<Entropy>) {
    let seed = entropy.get();
    let rng = SmallRng::from_seed(seed);
    commands.insert_resource(rng);
    info!("Initialized coin chooser resource with seed: {:02X?}", seed);
}

// System to spawn coins into the world.
fn spawn_coins(mut commands: Commands, mut entropy: ResMut<Entropy>) {
    for _ in 1..100 {
        // Coin tosses are independent so they need their own rngs.
        // They are seeded from the world's entropy so they are deterministic.
        let seed = entropy.get();
        let rng = SmallRng::from_seed(seed);
        commands
            .spawn()
            .insert(Coin)
            .insert(Face::Heads)
            .insert(Toss(rng));
    }
    info!("Spawned coins")
}

// System to toss a random coin.
fn toss_coin(
    mut query: Query<(Entity, &mut Face, &mut Toss), With<Coin>>,
    coin_chooser: ResMut<SmallRng>,
) {
    // Pick a random coin.
    // Note that this uses the global random number generator.
    if let Some((ent, mut face, mut toss)) = query.iter_mut().choose(&mut coin_chooser.into_inner())
    {
        // Toss it to determine the resulting Face.
        // Note that this uses the coin-specific random number generator.
        let new_face = if toss.0.gen_bool(0.5) {
            Face::Heads
        } else {
            Face::Tails
        };

        info!(
            "Tossed entity {:?} - old: {:?} new: {:?}",
            ent, *face, new_face
        );

        // If the face has changed, update it.
        if *face != new_face {
            info!("    - Updating face for entity {:?} to {:?}", ent, new_face);
            let f = face.as_mut();
            *f = new_face;
        }
    }
}
