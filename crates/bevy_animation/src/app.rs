use bevy_app::AppBuilder;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use std::{
    any::{type_name, TypeId},
    mem::drop,
};

use crate::{
    custom::{AnimatedAsset, AnimatedComponent, AnimatorRegistry},
    reflect, stage,
};

pub trait AddAnimated {
    fn register_animated_component<T: AnimatedComponent>(&mut self) -> &mut Self;

    fn register_animated_asset<T: AnimatedAsset>(&mut self) -> &mut Self;

    fn register_animated<T: Struct + Component>(&mut self) -> &mut Self;
}

impl AddAnimated for AppBuilder {
    fn register_animated_component<T: AnimatedComponent>(&mut self) -> &mut Self {
        let mut registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorRegistry::default);

        if registry.targets.insert(TypeId::of::<T>()) {
            registry.static_properties.extend(T::PROPERTIES.iter());
            drop(registry);

            self.add_system_to_stage(stage::ANIMATE, T::animator_update_system);
            self
        } else {
            panic!("animator already registered for '{}'", type_name::<T>());
        }
    }

    fn register_animated_asset<T: AnimatedAsset>(&mut self) -> &mut Self {
        let mut registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorRegistry::default);

        if registry.targets.insert(TypeId::of::<T>()) {
            registry.static_properties.extend(T::PROPERTIES.iter());
            drop(registry);

            self.add_system_to_stage(stage::ANIMATE, T::animator_update_system);
            self
        } else {
            panic!("animator already registered for '{}'", type_name::<T>());
        }
    }

    fn register_animated<T: Struct + Component>(&mut self) -> &mut Self {
        let mut registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorRegistry::default);

        if registry.targets.insert(TypeId::of::<T>()) {
            // registry.static_properties.extend(T::PROPERTIES.iter());
            drop(registry);

            self.add_system_to_stage(stage::ANIMATE, reflect::animate_system::<T>);
            self
        } else {
            panic!("animator already registered for '{}'", type_name::<T>());
        }
    }
}
