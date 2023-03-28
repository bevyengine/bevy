//! This example shows how [`FallibleSystemParam`] can be used to create a flexible system for interacting with a trait object resource.
//!
//! This is fairly advanced and the [`SystemParam`](bevy::ecs::system::SystemParam) derive macro can be used in many cases.
//!
//! This pattern is useful for working with resources where the exact type isn't known.
//! The system param allows for expressing the desired type as a type parameter,
//! which is far more convenient than getting the resource directly and handling it in every system.

use std::ops::{Deref, DerefMut};

use bevy::{
    ecs::{
        component::{ComponentId, Tick},
        system::{FallibleSystemParam, ReadOnlySystemParam, ResError, ResMutError, SystemMeta},
    },
    prelude::*,
};

// Resources simulating game statistics.
#[derive(Resource)]
pub struct GameTime(f32);

#[derive(Resource)]
pub struct GameKills(u32);

fn main() {
    App::new()
        .insert_resource(GameTime(532.1))
        .insert_resource(GameKills(31))
        .insert_resource(CurrentGameMode::new(Deathmatch {
            max_time: 600.0,
            max_kills: 30,
        }))
        .add_systems(Update, update_deathmatch)
        .run();
}

// This struct encapsulates the trait object so it can used as a resource.
#[derive(Resource)]
pub struct CurrentGameMode(Box<dyn GameMode>);

impl CurrentGameMode {
    pub fn new(mode: impl GameMode) -> Self {
        Self(Box::new(mode))
    }

    pub fn to_ref<T: GameMode>(&self) -> Option<&T> {
        GameMode::as_reflect(&*self.0).downcast_ref()
    }

    pub fn to_mut<T: GameMode>(&mut self) -> Option<&mut T> {
        GameMode::as_reflect_mut(&mut *self.0).downcast_mut()
    }

    pub fn type_name(&self) -> String {
        self.0.type_name().to_string()
    }
}

// This is the error type returned when the resource isn't the correct type.
#[derive(Debug)]
pub enum GameError {
    MissingResource,
    WrongType(&'static str, String),
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingResource => write!(f, "Resource `CurrentGameMode` missing"),
            Self::WrongType(expected, found) => {
                write!(f, "Expected game mode `{expected}`, found `{found}`")
            }
        }
    }
}

impl std::error::Error for GameError {}

impl From<ResError<CurrentGameMode>> for GameError {
    fn from(_: ResError<CurrentGameMode>) -> Self {
        Self::MissingResource
    }
}

impl From<ResMutError<CurrentGameMode>> for GameError {
    fn from(_: ResMutError<CurrentGameMode>) -> Self {
        Self::MissingResource
    }
}

// This is the optional system param that abstracts away converting the resource to the actual type.
pub struct Game<'w, T: GameMode> {
    mode: &'w T,
}

// Deref makes it convenient to interact with the actual data.
impl<'w, T: GameMode> Deref for Game<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.mode
    }
}

#[doc(hidden)]
pub struct GameState {
    // The `OptionalSystemParam::State` of `Res<CurrentGameMode>`.
    mode_id: ComponentId,
}

// SAFETY: The resource state is initialized.
unsafe impl<T: GameMode> FallibleSystemParam for Game<'_, T> {
    type State = GameState;
    type Item<'world, 'state> = Game<'world, T>;
    type Error = GameError;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        GameState {
            mode_id: <Res<CurrentGameMode> as FallibleSystemParam>::init_state(world, system_meta),
        }
    }

    unsafe fn get_param<'world>(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: &'world World,
        change_tick: Tick,
    ) -> Result<Game<'world, T>, GameError> {
        let current_mode = <Res<CurrentGameMode> as FallibleSystemParam>::get_param(
            &mut state.mode_id,
            system_meta,
            world,
            change_tick,
        )?
        .into_inner();

        current_mode
            .to_ref()
            .map(|mode| Game { mode })
            .ok_or(GameError::WrongType(
                std::any::type_name::<T>(),
                current_mode.type_name(),
            ))
    }
}

// SAFETY: since this system param only reads the resource, it can be marked read-only to enable shared access.
unsafe impl<'w, T: GameMode> ReadOnlySystemParam for Game<'w, T> {}

// A mutable version of the system param.
// Note: it does not implement `ReadOnlySystemParam`.
pub struct GameMut<'w, T: GameMode> {
    mode: &'w mut T,
}

impl<'w, T: GameMode> Deref for GameMut<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.mode
    }
}

impl<'w, T: GameMode> DerefMut for GameMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mode
    }
}

// SAFETY: The resource state is initialized.
unsafe impl<T: GameMode> FallibleSystemParam for GameMut<'_, T> {
    type State = GameState;
    type Item<'world, 'state> = GameMut<'world, T>;
    type Error = GameError;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        GameState {
            mode_id: <ResMut<CurrentGameMode> as FallibleSystemParam>::init_state(
                world,
                system_meta,
            ),
        }
    }

    unsafe fn get_param<'world>(
        state: &mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'world World,
        change_tick: Tick,
    ) -> Result<GameMut<'world, T>, GameError> {
        let current_mode = <ResMut<CurrentGameMode> as FallibleSystemParam>::get_param(
            &mut state.mode_id,
            system_meta,
            world,
            change_tick,
        )?
        .into_inner();
        let found = current_mode.type_name();
        current_mode
            .to_mut()
            .map(|mode| GameMut { mode })
            .ok_or(GameError::WrongType(std::any::type_name::<T>(), found))
    }
}

// This trait can be used to implement common behavior.
pub trait GameMode: Reflect + Send + Sync + 'static {
    fn as_reflect(&self) -> &dyn Reflect;

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;
}

// This struct implements the trait.
// There could be many structs like this, not necessarily defined in the same library.
#[derive(Reflect)]
pub struct Deathmatch {
    pub max_time: f32,
    pub max_kills: u32,
}

impl GameMode for Deathmatch {
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }
}

fn update_deathmatch(time: Res<GameTime>, kills: Res<GameKills>, game: Option<Game<Deathmatch>>) {
    let Some(game) = game else { return };

    if time.0 >= game.max_time {
        println!("Time ran out!");
    }

    if kills.0 >= game.max_kills {
        println!("Max kills reached!");
    }
}
