//! Definitions for [`Template`] reflection.
//! This allows building a [`Template`] from types only known at runtime.
//!
//! This module exports two types: [`ReflectTemplateFns`] and [`ReflectTemplate`].
//!
//! Same as [`component`](`super::component`), but for [`Template`].

use alloc::boxed::Box;
use bevy_ecs::reflect::from_reflect_with_fallback;
use bevy_reflect::{
    CreateTypeData, PartialReflect, Reflect, TypePath, TypeRegistration, TypeRegistry,
};
use std::any::TypeId;

use crate::error::Result;
use crate::template::{Template, TemplateContext};

/// A struct used to operate on the reflected [`Template`] trait of a type.
///
/// A [`ReflectTemplate`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`] or [`ReflectFromTemplate::get_template`](`super::ReflectFromTemplate::get_template`).
#[derive(Clone)]
pub struct ReflectTemplate(ReflectTemplateFns);

/// The raw function pointers needed to make up a [`ReflectTemplate`].
#[derive(Clone)]
pub struct ReflectTemplateFns {
    /// Function pointer implementing [`ReflectTemplate::get_output`].
    pub get_output: fn(&TypeRegistry) -> Option<&TypeRegistration>,
    /// Function pointer implementing [`ReflectTemplate::build`].
    pub build:
        fn(&mut TemplateContext, &dyn PartialReflect, &TypeRegistry) -> Result<Box<dyn Reflect>>,
}

impl ReflectTemplateFns {
    /// Get the default set of [`ReflectTemplateFns`] for a specific type using its
    /// [`CreateTypeData`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Reflect + Template<Output: Reflect> + TypePath>() -> Self {
        <ReflectTemplate as CreateTypeData<T>>::create_type_data(()).0
    }
}

impl ReflectTemplate {
    /// fetches the Registration for the [`Template`] Output
    pub fn get_output<'a>(&self, registry: &'a TypeRegistry) -> Option<&'a TypeRegistration> {
        (self.0.get_output)(registry)
    }

    /// builds a reflected [`Template`] into it's Output like [`build_template`](Template::build_template).
    pub fn build(
        &self,
        context: &mut TemplateContext,
        reflected_template: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) -> Result<Box<dyn Reflect>> {
        (self.0.build)(context, reflected_template, registry)
    }

    /// Create a custom implementation of [`ReflectTemplate`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Template)]` bundle
    /// to generate a [`ReflectTemplate`] implementation automatically.
    ///
    /// See [`ReflectTemplateFns`] for more information.
    pub fn new(fns: ReflectTemplateFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectTemplate`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectTemplate>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectTemplate`] and keeping it
    /// between frames, cloning a `ReflectTemplate` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectTemplate`,
    /// use `fn_pointers` to get the underlying [`ReflectTemplateFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectTemplate>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectTemplateFns {
        &self.0
    }
}

impl<T: Reflect + Template<Output: Reflect> + TypePath> CreateTypeData<T> for ReflectTemplate {
    fn create_type_data(_input: ()) -> Self {
        ReflectTemplate(ReflectTemplateFns {
            get_output: |registry: &TypeRegistry| {
                let registration = registry.get(TypeId::of::<T::Output>());

                registration
            },
            build: |context, reflected_template, registry| {
                let template = context.entity.world_scope(|world| {
                    from_reflect_with_fallback::<T>(reflected_template, world, registry)
                });

                template
                    .build_template(context)
                    .map(|result| Box::new(result) as Box<dyn Reflect>)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::reflect::from_template::ReflectFromTemplate;
    use crate::reflect::template::ReflectTemplate;
    use crate::template::{FromTemplate, SceneEntityReferences, Template};
    use bevy_ecs::prelude::World;
    use bevy_ecs::template::TemplateContext;
    use bevy_reflect::prelude::ReflectDefault;
    use bevy_reflect::{Reflect, TypeRegistry};
    use std::any::TypeId;

    #[test]
    fn build_template() {
        #[derive(Reflect, Default, FromTemplate, Clone, Debug, Eq, PartialEq)]
        #[reflect(Default, FromTemplate)]
        struct MyStruct {
            foo: i32,
        }

        let mut world = World::new();

        let mut registry = TypeRegistry::empty();

        registry.register::<MyStruct>();
        registry.register_type_data::<MyStruct, ReflectFromTemplate>();
        registry.register::<MyStructTemplate>();
        registry.register_type_data::<MyStructTemplate, ReflectTemplate>();
        registry.register_type_data::<MyStructTemplate, ReflectDefault>();

        let my_struct_registration = registry.get(TypeId::of::<MyStruct>()).unwrap();
        let reflect_from_template = my_struct_registration
            .data::<ReflectFromTemplate>()
            .unwrap();

        let my_struct_template_registration =
            reflect_from_template.get_template(&registry).unwrap();
        let reflect_template = my_struct_template_registration
            .data::<ReflectTemplate>()
            .unwrap();
        let reflect_default = my_struct_template_registration
            .data::<ReflectDefault>()
            .unwrap();

        let template = reflect_default.default();

        let mut entity = world.spawn_empty();
        let mut scene_entity_references = SceneEntityReferences::default();
        let mut template_context = TemplateContext::new(&mut entity, &mut scene_entity_references);
        let my_struct_reflect = reflect_template
            .build(&mut template_context, template.as_ref(), &registry)
            .expect("Should be able to build Template");
        let my_struct = my_struct_reflect
            .downcast::<MyStruct>()
            .expect("Should be MyStruct");

        assert_eq!(*my_struct, MyStruct { foo: 0 })
    }
}
