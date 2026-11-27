use bevy_ecs::world::World;

use crate::erased::schedule::BeforeOrAfter;

pub type Order = Option<BeforeOrAfter<RegistrationOrder>>;

pub struct ErasedDataless<O> {
    ordering: O,
    func: fn(&mut World),
}

pub enum RegistrationOrder {
    Messages,
    Events,
    Observers,
    Schedules,
    Resources,
    Systems,
}
