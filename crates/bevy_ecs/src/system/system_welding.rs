use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    system::{IntoSystem, System},
    world::World,
};
use std::borrow::Cow;

/// A [`System`] that welds two systems together, creating a new system that routes the output of
/// the first system into the input of the second system, yielding the output of the second system.
///
/// Given two systems A and B, B may be welded onto A as `A.weld(B)`
/// if the output type of A is the same as the input type of B.
///
/// The output type is the return value of the function.
/// The input type is the first [`SystemParam`](crate::system::SystemParam)
/// if it is wrapped by [`In`](crate::system::In).
/// If no input parameter is designated, the input type is `()`.
///
/// Systems can only be welded in a one-to-one fashion, and
/// welding systems causes them to be treated as a single cohesive unit by the scheduler.
/// System welding is a specialized tool designed to be used when handling the output of systems;
/// for general purpose inter-system communication you should use [`Events`](crate::event::Events).
///
/// # Examples
///
/// ```
/// use std::num::ParseIntError;
///
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut world = World::default();
///     world.insert_resource(Message("42".to_string()));
///
///     // Weld the `parse_message_system`'s output into the `filter_system`s input
///     let mut welded_system = parse_message_system.weld(filter_system);
///     welded_system.initialize(&mut world);
///     assert_eq!(welded_system.run((), &mut world), Some(42));
/// }
///
/// struct Message(String);
///
/// fn parse_message_system(message: Res<Message>) -> Result<usize, ParseIntError> {
///     message.0.parse::<usize>()
/// }
///
/// fn filter_system(In(result): In<Result<usize, ParseIntError>>) -> Option<usize> {
///     result.ok().filter(|&n| n < 100)
/// }
/// ```
pub struct WeldSystem<SystemA, SystemB> {
    system_a: SystemA,
    system_b: SystemB,
    name: Cow<'static, str>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

impl<SystemA: System, SystemB: System<In = SystemA::Out>> System for WeldSystem<SystemA, SystemB> {
    type In = SystemA::In;
    type Out = SystemB::Out;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        self.system_a.new_archetype(archetype);
        self.system_b.new_archetype(archetype);

        self.archetype_component_access
            .extend(self.system_a.archetype_component_access());
        self.archetype_component_access
            .extend(self.system_b.archetype_component_access());
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn is_send(&self) -> bool {
        self.system_a.is_send() && self.system_b.is_send()
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        let out = self.system_a.run_unsafe(input, world);
        self.system_b.run_unsafe(out, world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.system_a.apply_buffers(world);
        self.system_b.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.system_a.initialize(world);
        self.system_b.initialize(world);
        self.component_access
            .extend(self.system_a.component_access());
        self.component_access
            .extend(self.system_b.component_access());
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.system_a.check_change_tick(change_tick);
        self.system_b.check_change_tick(change_tick);
    }
}

/// An extension trait providing the [`IntoWeldSystem::weld`] method
/// for convenient [`System`] welding.
///
/// This trait is blanket implemented for all system pairs that fulfill the welding requirement.
///
/// See [`WeldSystem`].
pub trait IntoWeldSystem<ParamA, Payload, SystemB, ParamB, Out>:
    IntoSystem<(), Payload, ParamA> + Sized
where
    SystemB: IntoSystem<Payload, Out, ParamB>,
{
    /// Weld this system `A` with another system `B` creating a new system that feeds system A's
    /// output into system `B`, returning the output of system `B`.
    fn weld(self, system: SystemB) -> WeldSystem<Self::System, SystemB::System>;
}

impl<SystemA, ParamA, Payload, SystemB, ParamB, Out>
    IntoWeldSystem<ParamA, Payload, SystemB, ParamB, Out> for SystemA
where
    SystemA: IntoSystem<(), Payload, ParamA>,
    SystemB: IntoSystem<Payload, Out, ParamB>,
{
    fn weld(self, system: SystemB) -> WeldSystem<SystemA::System, SystemB::System> {
        let system_a = self.system();
        let system_b = system.system();
        WeldSystem {
            name: Cow::Owned(format!("Weld({}, {})", system_a.name(), system_b.name())),
            system_a,
            system_b,
            archetype_component_access: Default::default(),
            component_access: Default::default(),
        }
    }
}
