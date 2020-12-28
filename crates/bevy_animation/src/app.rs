use bevy_app::AppBuilder;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    mem::drop,
};

use crate::{custom::AnimatorRegistry, reflect, stage};

pub trait AddAnimated {
    fn register_animated_component<T: Default + Struct + Component>(&mut self) -> &mut Self;
}

impl AddAnimated for AppBuilder {
    fn register_animated_component<T: Default + Struct + Component>(&mut self) -> &mut Self {
        let mut registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorRegistry::default);

        if registry.targets.insert(TypeId::of::<T>()) {
            let component = T::default();
            let descriptor = reflect::AnimatorDescriptor::<T>::from_component(&component);

            registry.static_properties.extend(
                descriptor
                    .properties()
                    .map(|(name, type_id)| (Cow::Owned(name.to_string()), type_id)),
            );

            drop(registry);

            self.add_resource(descriptor);
            self.add_system_to_stage(stage::ANIMATE, reflect::animate_component_system::<T>);
            self
        } else {
            panic!("animator already registered for '{}'", type_name::<T>());
        }
    }
}
