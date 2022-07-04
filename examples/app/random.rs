use bevy::prelude::*;
use rand::{prelude::IteratorRandom, rngs::SmallRng, Rng, SeedableRng};

// This example illustrates how to use entropy to control randomness in bevy.
// We randomly choose a coin to toss (which is itself random) and record the result.
//
// Because all the random number generators are seeded from the same "world seed",
// the chosen coin and results of the coin tosses are deterministic across runs.
//
// There are many different random number generators with different tradeoffs.
// We use a `SmallRng`, which is an insecure random number generator designed to
// be fast, simple, require little memory, and have good output quality.
// This would be an inappropriate choice for cryptographic randomness.
//
// See <https://docs.rs/rand/0.8.4/rand/rngs/index.html> for more details

#[derive(Component)]
struct Coin;

impl Coin {
    // Toss the coin and return the `Face` that lands up.
    // Coin tosses are independent so each toss needs its own
    // random number generator.
    fn toss(&self, seed: [u8; 32]) -> Face {
        let mut rng = SmallRng::from_seed(seed);
        if rng.gen_bool(0.5) {
            Face::Heads
        } else {
            Face::Tails
        }
    }
}

struct CoinChooser(SmallRng);

#[derive(Component, Debug, Eq, PartialEq)]
enum Face {
    Heads,
    Tails,
}

fn main() {
    // We create a random seed for the world. Random number generators created with
    // or derived from this seed will appear random during execution but will be
    // deterministic across multiple executions.
    // See <https://en.wikipedia.org/wiki/Random_seed> and
    // <https://blog.unity.com/technology/a-primer-on-repeatable-random-numbers>
    // for more details.
    //
    // The seed you choose may have security implications or influence the
    // distribution of the random numbers generated.
    // See <https://rust-random.github.io/book/guide-seeding.html> for more details
    // about how to pick a "good" random seed for your needs.
    //
    // Normally you would do one of the following:
    //   1. Get a good random seed out-of-band and hardcode it in the source.
    //   2. Dynamically call to the OS and print the seed so the user can rerun
    //      deterministically.
    //   3. Dynamically call to the OS and share the seed with a server so the
    //      client and server deterministically execute together.
    //   4. Load the seed from a server so the client and server deterministically
    //      execute together.
    let world_seed: [u8; 32] = [1; 32];

    // Create a source of entropy for the world using the random seed.
    let mut world_entropy = Entropy::from(world_seed);

    // Create a coin chooser, seeded from the world's entropy.
    // We do this at the start of the world so the random number generator backing
    // the coin chooser is not influenced by coin tosses.
    let seed = world_entropy.get();
    let coin_chooser = CoinChooser(SmallRng::from_seed(seed));

    App::new()
        // Delete the following line to use the default OS-provided entropy source.
        // Note that doing so introduces non-determinism.
        .insert_resource(world_entropy)
        .insert_resource(coin_chooser)
        .add_plugins_with(MinimalPlugins, |group| {
            group.add(bevy::entropy::EntropyPlugin);
            group.add(bevy::log::LogPlugin)
        })
        .add_startup_system(spawn_coins)
        .add_system(toss_coin)
        .run();
}

// System to spawn coins into the world.
fn spawn_coins(mut commands: Commands) {
    for _ in 1..100 {
        commands.spawn().insert(Coin).insert(Face::Heads);
    }
    info!("Spawned coins");
}

// System to toss a random coin.
fn toss_coin(
    mut query: Query<(Entity, &Coin, &mut Face), With<Coin>>,
    coin_chooser: ResMut<CoinChooser>,
    mut entropy: ResMut<Entropy>,
) {
    // Pick a random coin.
    if let Some((ent, coin, mut face)) = query.iter_mut().choose(&mut coin_chooser.into_inner().0) {
        // Toss it to determine the resulting [Face].
        // Tosses are seeded from the world's entropy and are deterministic.
        let seed = entropy.get();
        let new_face = coin.toss(seed);

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
