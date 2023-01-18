//! This example shows how [`ResultfulSystemParam`] can be used to encapsulate simple behavior for use across systems.
//!
//! This is fairly advanced and the [`SystemParam`] derive macro can be used in many cases.

use bevy::{
    ecs::system::{ReadOnlySystemParam, ResultfulSystemParam, SystemParam},
    prelude::*,
};

fn main() {
    App::new()
        .add_startup_system(setup)
        .add_system(display_statistics)
        .run();
}

// A component associated with each player representing game progress.
#[derive(Component)]
pub struct Score(pub u32);

// The system param that represents the average score of all players.
// It fails when there aren't any `Score` components in the world.
pub struct AverageScore(f32);

#[doc(hidden)]
pub struct AverageScoreState {
    query_state: QueryState<&'static Score>,
}

unsafe impl ResultfulSystemParam for AverageScore {
    type State = AverageScoreState;

    type Item<'world, 'state> = AverageScore;

    type Error = AverageScoreError;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        AverageScoreState {
            query_state: <Query<&Score> as SystemParam>::init_state(world, system_meta),
        }
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Result<
        Self::Item<'world, 'state>,
        <Self::Item<'world, 'state> as ResultfulSystemParam>::Error,
    > {
        let query = <Query<&Score> as SystemParam>::get_param(
            &mut state.query_state,
            system_meta,
            world,
            change_tick,
        );

        if query.is_empty() {
            return Err(AverageScoreError::Empty);
        }

        let total = query.iter().map(|p| p.0).sum::<u32>() as f32;

        let size = query.iter().len() as f32;

        Ok(AverageScore(total / size))
    }
}

// Since the system param only reads the query, it can be marked read-only.
unsafe impl ReadOnlySystemParam for AverageScore {}

#[derive(Debug)]
pub enum AverageScoreError {
    Empty,
}

impl std::fmt::Display for AverageScoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AverageScoreError::Empty => write!(f, "No players found!"),
        }
    }
}

impl std::error::Error for AverageScoreError {}

fn setup(mut commands: Commands) {
    commands.spawn(Score(30));
    commands.spawn(Score(5));
    commands.spawn(Score(25));
}

fn display_statistics(avg_score: Result<AverageScore, AverageScoreError>) {
    match avg_score {
        Ok(AverageScore(avg)) => println!("Average Score: {avg}"),
        Err(AverageScoreError::Empty) => println!("Average Score: Not Available"),
    }
}
