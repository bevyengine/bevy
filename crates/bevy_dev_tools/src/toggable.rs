use crate::dev_command::*;
use bevy_ecs::world::{Command, World};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::FromReflect;
use bevy_reflect::Reflect;
use bevy_reflect::TypePath;

/// Trait that represents a toggable dev tool
pub trait Toggable {
    /// Enables the dev tool in the given `world`.
    fn enable(world: &mut World);
    /// Disables the dev tool in the given `world`.
    fn disable(world: &mut World);
    /// Checks if the dev tool is enabled in the given `world`.
    fn is_enabled(world: &World) -> bool;
}

/// Command to enable a `Toggable` component or system.
#[derive(Reflect, Default)]
#[reflect(DevCommand, Default)]
pub struct Enable<T: Toggable + FromReflect + Send + Sync + 'static + Default> {
    /// PhantomData to hold the type `T`.
    #[reflect(ignore)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Toggable + Send + Sync + 'static + TypePath + FromReflect + Default> DevCommand for Enable<T> {}

impl<T: Toggable + FromReflect + Send + Sync + 'static + Default> Command for Enable<T> {
    /// Applies the enable command, enabling the `Toggable` dev tool in the `world`.
    fn apply(self, world: &mut World) {
        T::enable(world);
    }
}

/// Command to disable a `Toggable` dev tool.
#[derive(Reflect, Default)]
#[reflect(DevCommand, Default)]
pub struct Disable<T: Toggable + FromReflect + Send + Sync + 'static + Default> {
    /// PhantomData to hold the type `T`.
    #[reflect(ignore)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Toggable + Send + Sync + 'static + TypePath + FromReflect + Default> DevCommand for Disable<T> {}

impl<T: Toggable + FromReflect + Send + Sync + 'static + Default> Command for Disable<T> {
    /// Applies the disable command, disabling the `Toggable` dev tool in the `world`.
    fn apply(self, world: &mut World) {
        T::disable(world);
    }
}