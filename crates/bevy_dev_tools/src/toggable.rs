use crate::DevCommand;
use crate::dev_command::*;
use bevy_ecs::world::{Command, World};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::FromReflect;
use bevy_reflect::Reflect;
use bevy_reflect::TypePath;

pub trait Toggable {
    fn enable(world: &mut World);
    fn disable(world: &mut World);
    fn is_enabled(world: &World) -> bool;
}

#[derive(Reflect, Default)]
#[reflect(DevCommand, Default)]
pub struct Enable<T : Toggable + FromReflect + Send + Sync + 'static + Default> {
    #[reflect(ignore)]
    _phantom: std::marker::PhantomData<T>,
}
impl<T : Toggable + Send + Sync + 'static + TypePath + FromReflect + Default> DevCommand for Enable<T> {}
impl<T: Toggable + FromReflect + Send + Sync + 'static + Default> Command for Enable<T> {
    fn apply(self, world: &mut World) {
        T::enable(world);
    }
}

#[derive(Reflect, Default)]
#[reflect(DevCommand, Default)]
pub struct Disable<T : Toggable + FromReflect + Send + Sync + 'static + Default> {
    #[reflect(ignore)]
    _phantom: std::marker::PhantomData<T>,
}
impl<T : Toggable + Send + Sync + 'static + TypePath + FromReflect + Default> DevCommand for Disable<T> {}
impl<T: Toggable + FromReflect + Send + Sync + 'static + Default> Command for Disable<T> {
    fn apply(self, world: &mut World) {
        T::disable(world);
    }
}