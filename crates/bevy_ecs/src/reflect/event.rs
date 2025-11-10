use crate::{event::Event, reflect::from_reflect_with_fallback, world::World};
use bevy_reflect::{FromType, PartialReflect, Reflect, TypePath, TypeRegistry};

#[derive(Clone)]
pub struct ReflectEvent {
    trigger: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
}

impl ReflectEvent {
    pub fn trigger(&self, world: &mut World, event: &dyn PartialReflect, registry: &TypeRegistry) {
        (self.trigger)(world, event, registry)
    }
}

impl<'a, E: Reflect + Event + TypePath> FromType<E> for ReflectEvent
where
    <E as Event>::Trigger<'a>: Default,
{
    fn from_type() -> Self {
        ReflectEvent {
            trigger: |world, reflected_event, registry| {
                let event = from_reflect_with_fallback::<E>(reflected_event, world, registry);
                world.trigger(event);
            },
        }
    }
}
