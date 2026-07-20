//! Definitions for [`SceneComponent`] reflection.
//! This allows building a [`SceneComponent`] from types only known at runtime.
//!
//! This module exports two types: [`ReflectSceneComponentFns`] and [`ReflectSceneComponent`].
//!
//! Same as [`component`](`super::component`), but for [`SceneComponent`].

use alloc::boxed::Box;
use bevy_ecs::reflect::from_reflect_with_fallback;
use bevy_reflect::{
    CreateTypeData, PartialReflect, Reflect, TypePath, TypeRegistration, TypeRegistry,
};
use std::any::TypeId;

use crate::{Scene, SceneComponent};
use bevy_ecs::world::World;

/// A struct used to operate on the reflected [`SceneComponent`] trait of a type.
///
/// A [`ReflectSceneComponent`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectSceneComponent(ReflectSceneComponentFns);

/// The raw function pointers needed to make up a [`ReflectSceneComponent`].
#[derive(Clone)]
pub struct ReflectSceneComponentFns {
    /// Function pointer implementing [`ReflectSceneComponent::get_props`].
    pub get_props: fn(&TypeRegistry) -> Option<&TypeRegistration>,
    /// Function pointer implementing [`ReflectSceneComponent::get_scene`].
    pub get_scene: fn(&mut World, &dyn PartialReflect, &TypeRegistry) -> Box<dyn Scene>,
}

impl ReflectSceneComponentFns {
    /// Get the default set of [`ReflectSceneComponentFns`] for a specific type using its
    /// [`CreateTypeData`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<R: Reflect + TypePath, T: Reflect + SceneComponent<Props = R> + TypePath>() -> Self {
        <ReflectSceneComponent as CreateTypeData<T>>::create_type_data(()).0
    }
}

impl ReflectSceneComponent {
    /// fetches the Registration for the [`SceneComponent`] Props
    pub fn get_props<'a>(&self, registry: &'a TypeRegistry) -> Option<&'a TypeRegistration> {
        (self.0.get_props)(registry)
    }

    /// fetches the associated scene of a reflected [`SceneComponent`].
    pub fn get_scene(
        &self,
        world: &mut World,
        props: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) -> Box<dyn Scene> {
        (self.0.get_scene)(world, props, registry)
    }

    /// Create a custom implementation of [`ReflectSceneComponent`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(SceneComponent)]` bundle
    /// to generate a [`ReflectSceneComponent`] implementation automatically.
    ///
    /// See [`ReflectSceneComponentFns`] for more information.
    pub fn new(fns: ReflectSceneComponentFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectSceneComponent`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectSceneComponent>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectSceneComponent`] and keeping it
    /// between frames, cloning a `ReflectSceneComponent` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectSceneComponent`,
    /// use `fn_pointers` to get the underlying [`ReflectSceneComponentFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectSceneComponent>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectSceneComponentFns {
        &self.0
    }
}

impl<R: Reflect + TypePath, T: Reflect + SceneComponent<Props = R> + TypePath> CreateTypeData<T>
    for ReflectSceneComponent
{
    fn create_type_data(_input: ()) -> Self {
        ReflectSceneComponent(ReflectSceneComponentFns {
            get_props: |registry: &TypeRegistry| {
                let registration = registry.get(TypeId::of::<T::Props>());

                registration
            },
            get_scene: |world, reflected_props, registry| {
                let props = from_reflect_with_fallback::<R>(reflected_props, world, registry);

                Box::new(T::scene(props))
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::reflect::ReflectSceneComponent;
    use crate::{self as bevy_scene, ScenePlugin, WorldSceneExt};
    use bevy_app::App;
    use bevy_asset::AssetPlugin;
    use bevy_ecs::prelude::Name;
    use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypeRegistry};
    use bevy_scene_macros::{bsn, SceneComponent};
    use std::any::TypeId;

    #[test]
    fn spawn_scene_component() {
        #[derive(SceneComponent, Default, Clone, Reflect, Debug, PartialEq)]
        #[reflect(Default, SceneComponent)]
        #[scene(MySceneComponentProps)]
        struct MySceneComponent;

        #[derive(Reflect)]
        #[reflect(Default)]
        struct MySceneComponentProps {
            name: String,
        }

        impl Default for MySceneComponentProps {
            fn default() -> Self {
                Self {
                    name: "MySceneComponent".to_string(),
                }
            }
        }

        impl MySceneComponent {
            fn scene(props: MySceneComponentProps) -> impl bevy_scene::Scene {
                bsn! {
                    Name({props.name})
                }
            }
        }

        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), ScenePlugin));

        let world = app.world_mut();

        let mut registry = TypeRegistry::empty();

        registry.register::<MySceneComponent>();
        registry.register_type_data::<MySceneComponent, ReflectSceneComponent>();
        registry.register::<MySceneComponentProps>();
        registry.register_type_data::<MySceneComponentProps, ReflectDefault>();

        let my_scene_component_registry = registry.get(TypeId::of::<MySceneComponent>()).unwrap();
        let reflect_scene_component = my_scene_component_registry
            .data::<ReflectSceneComponent>()
            .unwrap();
        let my_scene_component_props_registry =
            reflect_scene_component.get_props(&registry).unwrap();
        let my_scene_component_props_reflect_default = my_scene_component_props_registry
            .data::<ReflectDefault>()
            .unwrap();
        let my_scene_component_props = my_scene_component_props_reflect_default.default();
        let my_scene_component_scene =
            reflect_scene_component.get_scene(world, my_scene_component_props.as_ref(), &registry);

        let entity = world
            .spawn_scene(my_scene_component_scene)
            .expect("Scene should be spawnable")
            .id();

        let query = world
            .query::<(&Name, &MySceneComponent)>()
            .get(world, entity)
            .expect("query should contain entity");

        assert_eq!(query.0, &Name::from("MySceneComponent"));
        assert_eq!(query.1, &MySceneComponent);
    }
}
