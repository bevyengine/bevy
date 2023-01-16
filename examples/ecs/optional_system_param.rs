//! This example shows how an [`OptionalSystemParam`] can be used to create a flexible system for handling a trait object resource.
//!
//! This pattern is useful for libraries that work with user-defined types, where alternatives like enums would be cumbersome.

use std::ops::{Deref, DerefMut};

use bevy::{
    ecs::{
        component::ComponentId,
        system::{OptionalSystemParam, ReadOnlySystemParam},
    },
    prelude::*,
};

// Resources simulating game statistics
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
        .add_system(update_deathmatch)
        .run();
}

// This resource encapsulates the trait object.
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
}

// This is our optional system param that abstracts away converting the resource to the actual type.
pub struct Game<'w, T: GameMode> {
    mode: &'w T,
}

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

unsafe impl<'w, T: GameMode> OptionalSystemParam for Game<'w, T> {
    type State = GameState;

    type Item<'world, 'state> = Game<'world, T>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        GameState {
            mode_id: <Res<CurrentGameMode> as OptionalSystemParam>::init_state(world, system_meta),
        }
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Option<Self::Item<'world, 'state>> {
        let current_mode = <Res<CurrentGameMode> as OptionalSystemParam>::get_param(
            &mut state.mode_id,
            system_meta,
            world,
            change_tick,
        )?
        .into_inner();
        current_mode.to_ref().map(|mode| Game { mode })
    }
}

// Marks the non-mut system param as read-only.
unsafe impl<'w, T: GameMode> ReadOnlySystemParam for Game<'w, T> {}

// A mutable version of the system param.
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

unsafe impl<'w, T: GameMode> OptionalSystemParam for GameMut<'w, T> {
    type State = GameState;

    type Item<'world, 'state> = GameMut<'world, T>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        GameState {
            mode_id: <ResMut<CurrentGameMode> as OptionalSystemParam>::init_state(world, system_meta),
        }
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Option<Self::Item<'world, 'state>> {
        let current_mode = <<ResMut<CurrentGameMode> as OptionalSystemParam>::Item<'world, 'state> as OptionalSystemParam>::get_param(&mut state.mode_id, system_meta, world, change_tick)?.into_inner();
        current_mode.to_mut().map(|mode| GameMut { mode })
    }
}

// This trait can be used to implement common behavior.
pub trait GameMode: Reflect + Send + Sync + 'static {
    fn as_reflect(&self) -> &dyn Reflect;

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;
}

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
        println!("Max kills reached!")
    }
}
