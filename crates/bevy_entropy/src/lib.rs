use bevy_app::{App, Plugin};
use bevy_utils::tracing::{debug, trace};
use rand::{rngs::StdRng, RngCore, SeedableRng};

pub mod prelude {
    #[doc(hidden)]
    pub use crate::Entropy;
}

/// Provides a source of entropy.
/// This enables deterministic random number generation.
///
/// See <https://github.com/bevyengine/bevy/discussions/2480> for issues
/// to be mindful of if you desire complete determinism.
#[derive(Default)]
pub struct EntropyPlugin;

impl Plugin for EntropyPlugin {
    fn build(&self, app: &mut App) {
        if !app.world.contains_resource::<Entropy>() {
            trace!("Creating entropy");
            app.init_resource::<Entropy>();
        }
    }
}

/// A resource that provides entropy.
pub struct Entropy(StdRng);

impl Default for Entropy {
    /// The default entropy source is non-deterministic and seeded from the operating system.
    /// For a deterministic source, use [`Entropy::from`].
    fn default() -> Self {
        debug!("Entropy created via the operating system");
        let rng = StdRng::from_entropy();
        Entropy(rng)
    }
}

impl Entropy {
    /// Create a deterministic source of entropy. All random number generators
    /// later seeded from an [`Entropy`] created this way will be deterministic.
    /// If determinism is not required, use [`Entropy::default`].
    pub fn from(seed: [u8; 32]) -> Self {
        debug!("Entropy created via seed: {:?} ", seed);
        let rng = StdRng::from_seed(seed);
        Entropy(rng)
    }

    /// Fill `dest` with entropy data. For an allocating alternative, see [`Entropy::get`].
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }

    /// Allocate and return entropy data. For a non-allocating alternative, see [`Entropy::fill_bytes`].
    pub fn get(&mut self) -> [u8; 32] {
        let mut dest = [0; 32];
        self.0.fill_bytes(&mut dest);
        dest
    }
}

#[cfg(test)]
mod test {
    use bevy_app::AppExit;
    use bevy_ecs::prelude::*;
    use bevy_internal::prelude::*;
    use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
    use std::sync::mpsc;
    use std::sync::mpsc::{Receiver, SyncSender};

    #[test]
    fn is_deterministic() {
        const APP_RUN_COUNT: u8 = 10;
        const CHOOSE_COUNT: u8 = 5;
        const THING_COUNT: u8 = 100;

        #[derive(Component)]
        struct Thing(u8);
        struct ResultChannel(SyncSender<u8>);

        // The result of the app we will check to make sure it is always the same.
        let mut expected_result: Option<Vec<u8>> = None;

        // The seed we will use for the random number generator in all app runs.
        let world_seed: [u8; 32] = [1; 32];

        // Run the app multiple times.
        for runs in 0..APP_RUN_COUNT {
            let (tx, rx): (SyncSender<u8>, Receiver<u8>) = mpsc::sync_channel(CHOOSE_COUNT.into());

            App::new()
                .insert_resource(Entropy::from(world_seed))
                .insert_resource(ResultChannel(tx))
                .add_plugins_with(MinimalPlugins, |group| group.add(super::EntropyPlugin))
                .add_startup_system(spawn_things)
                .add_system(choose_things)
                .run();

            fn spawn_things(mut commands: Commands) {
                for x in 1..THING_COUNT {
                    commands.spawn().insert(Thing(x));
                }
            }

            fn choose_things(
                query: Query<&Thing>,
                mut entropy: ResMut<Entropy>,
                result_channel: Res<ResultChannel>,
                mut app_exit_events: EventWriter<AppExit>,
            ) {
                // Create RNG from global entropy.
                let seed = entropy.get();
                let mut rng = SmallRng::from_seed(seed);

                // Choose some random things.
                for _ in 0..CHOOSE_COUNT {
                    if let Some(thing) = query.iter().choose(&mut rng) {
                        // Send the chosen thing out of the app so it can be inspected
                        // after the app exits.
                        result_channel.0.send(thing.0).expect("result to send");
                    }
                }
                app_exit_events.send(AppExit)
            }

            // The result of running the app.
            let run_result: Vec<u8> = rx.iter().collect();

            // If it is the first run, treat the current result as the expected
            // result we will check future runs against.
            if runs == 0 {
                expected_result = Some(run_result.clone());
            }

            assert_eq!(expected_result, Some(run_result));
        }
    }
}
