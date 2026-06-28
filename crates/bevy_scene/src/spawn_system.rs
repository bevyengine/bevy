use crate::{Scene, SceneList, WorldSceneExt};
use bevy_ecs::{error::Result, world::World};

/// Returns a system that spawns the given [`Scene`]. This should generally only be added to
/// schedules that run once, such as [`Startup`](bevy_app::Startup).
pub trait SpawnSystem {
    /// Returns a system that spawns the given [`Scene`]. This should generally only be added to
    /// schedules that run once, such as [`Startup`](bevy_app::Startup).
    fn spawn(self) -> impl FnMut(&mut World) -> Result;
}

impl<F: FnMut() -> S + Send + Sync + 'static, S: Scene> SpawnSystem for F {
    fn spawn(mut self) -> impl FnMut(&mut World) -> Result {
        move |world: &mut World| -> Result {
            world.spawn_scene(self())?;
            Ok(())
        }
    }
}

/// Returns a system that spawns the given [`SceneList`]. This should generally only be added to
/// schedules that run once, such as [`Startup`](bevy_app::Startup).
pub trait SpawnListSystem {
    /// Returns a system that spawns the given [`SceneList`]. This should generally only be added to
    /// schedules that run once, such as [`Startup`](bevy_app::Startup).
    fn spawn(self) -> impl FnMut(&mut World) -> Result;
}
impl<F: FnMut() -> S + Send + Sync + 'static, S: SceneList> SpawnListSystem for F {
    fn spawn(mut self) -> impl FnMut(&mut World) -> Result {
        move |world: &mut World| -> Result {
            world.spawn_scene_list(self())?;
            Ok(())
        }
    }
}
