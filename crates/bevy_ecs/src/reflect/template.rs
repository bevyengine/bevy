use alloc::boxed::Box;
use bevy_ecs::reflect::from_reflect_with_fallback;
use bevy_reflect::{CreateTypeData, PartialReflect, Reflect, TypePath, TypeRegistry};

use crate::error::Result;
use crate::template::{Template, TemplateContext};

#[derive(Clone)]
pub struct ReflectTemplate(ReflectTemplateFns);

#[derive(Clone)]
pub struct ReflectTemplateFns {
    pub build: fn(&mut TemplateContext, &dyn PartialReflect, &TypeRegistry) -> Result<Box<dyn Reflect>>
}

impl ReflectTemplateFns {
    pub fn new<T: Reflect + Template<Output=impl Reflect> + TypePath>() -> Self {
        <ReflectTemplate as CreateTypeData<T>>::create_type_data(()).0
    }
}

impl ReflectTemplate {
    pub fn build(&self, context: &mut TemplateContext, reflected_template: &dyn PartialReflect, registry: &TypeRegistry) -> Result<Box<dyn Reflect>> {
        (self.0.build)(context, reflected_template, registry)
    }

    pub fn new(fns: ReflectTemplateFns) -> Self {
        Self(fns)
    }

    pub fn fn_pointer(&self) -> &ReflectTemplateFns {
        &self.0
    }
}

impl<T: Reflect + Template<Output=impl Reflect> + TypePath> CreateTypeData<T> for ReflectTemplate {
    fn create_type_data(_input: ()) -> Self {
        ReflectTemplate(ReflectTemplateFns {
            build: |context, reflected_template, registry| {
                let template = context.entity.world_scope(|world| {
                    from_reflect_with_fallback::<T>(reflected_template, world, registry)
                });

                template.build_template(context).map(|result| Box::new(result) as Box<dyn Reflect>)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::ops::{Deref};
    use bevy_ecs::prelude::World;
    use bevy_ecs::template::TemplateContext;
    use bevy_reflect::{Reflect, TypeRegistry};
    use bevy_reflect::prelude::ReflectDefault;
    use crate::reflect::from_template::ReflectFromTemplate;
    use crate::reflect::template::ReflectTemplate;
    use crate::template::{FromTemplate, SceneEntityReferences, Template};

    #[test]
    fn build_template() {
        #[derive(Reflect, Default, Debug, Eq, PartialEq)]
        #[reflect(Default, FromTemplate)]
        struct MyStruct {
            foo: i32
        }

        impl FromTemplate for MyStruct {
            type Template = MyStructTemplate;
        }

        #[derive(Reflect, Default)]
        #[reflect(Default, Template)]
        struct MyStructTemplate {
            foo: i32
        }

        impl Template for MyStructTemplate {
            type Output = MyStruct;

            fn build_template(&self, _context: &mut TemplateContext) -> bevy_ecs::error::Result<Self::Output> {
                Ok(MyStruct {
                    foo: self.foo
                })
            }

            fn clone_template(&self) -> Self {
                Self {
                    foo: self.foo.clone()
                }
            }
        }

        let mut world = World::new();

        let mut registry = TypeRegistry::empty();
        #[cfg(feature = "reflect_auto_register")]
        registry.register_derived_types();
        #[cfg(not(feature = "reflect_auto_register"))]
        {
            registry.register::<MyStruct>();
            registry.register_type_data::<MyStruct, ReflectFromTemplate>();
            registry.register::<MyStructTemplate>();
            registry.register_type_data::<MyStructTemplate, ReflectTemplate>();
            registry.register_type_data::<MyStructTemplate, ReflectDefault>();
        }

        let my_struct_registration = registry.get(TypeId::of::<MyStruct>()).unwrap();
        let reflect_from_template = my_struct_registration.data::<ReflectFromTemplate>().unwrap();

        let my_struct_template_registration = reflect_from_template.get_template(&registry).unwrap();
        let reflect_template = my_struct_template_registration.data::<ReflectTemplate>().unwrap();
        let reflect_default = my_struct_template_registration.data::<ReflectDefault>().unwrap();

        let template = reflect_default.default();

        let mut entity = world.spawn_empty();
        let mut scene_entity_references = SceneEntityReferences::default();
        let mut template_context = TemplateContext::new(
            &mut entity,
            &mut scene_entity_references
        );
        let my_struct_reflect = reflect_template.build(
            &mut template_context,
            template.as_partial_reflect(),
            &registry
        ).expect("Should be able to build Template");
        let my_struct = my_struct_reflect.downcast::<MyStruct>().expect("Should be MyStruct");

        assert_eq!(
            *my_struct.deref(),
            MyStruct {
                foo: 0
            }
        )
    }
}