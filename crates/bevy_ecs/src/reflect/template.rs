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
    use bevy_reflect::{Reflect, TypeInfo, TypeRegistry};
    use bevy_reflect::structs::DynamicStruct;
    use crate::reflect::from_template::ReflectFromTemplate;
    use crate::reflect::template::ReflectTemplate;
    use crate::reflect::PartialReflect;
    use crate::template::{FromTemplate, SceneEntityReferences, Template};

    #[derive(Reflect, Default, Debug, Eq, PartialEq)]
    struct MyStruct {
        foo: i32
    }

    impl FromTemplate for MyStruct {
        type Template = MyStructTemplate;
    }

    #[derive(Reflect, Default)]
    struct MyStructTemplate {
        foo: i32
    }

    impl Template for MyStructTemplate {
        type Output = MyStruct;

        fn build_template(&self, context: &mut TemplateContext) -> bevy_ecs::error::Result<Self::Output> {
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

    #[test]
    fn build_template() {
        let mut world = World::new();

        let mut registry = TypeRegistry::empty();
        registry.register::<MyStruct>();
        registry.register_type_data::<MyStruct, ReflectFromTemplate>();
        registry.register::<MyStructTemplate>();
        registry.register_type_data::<MyStructTemplate, ReflectTemplate>();

        let my_struct_registration = registry.get(TypeId::of::<MyStruct>()).unwrap();
        let reflect_from_template = my_struct_registration.data::<ReflectFromTemplate>().unwrap();
        let reflect_template = reflect_from_template.get_template(&registry).unwrap();

        let my_struct_template_registration = registry.get(TypeId::of::<MyStructTemplate>()).unwrap();
        let type_info = my_struct_template_registration.type_info();
        let TypeInfo::Struct(info) = type_info else {
            panic!("TypeInfo should be Struct");
        };
        let foo = info.field("foo").expect("Should have foo field");
        let mut template = DynamicStruct::default();
        template.insert("foo", 0);
        template.set_represented_type(Some(type_info));

        let mut entity = world.spawn(());
        let mut scene_entity_references = SceneEntityReferences::default();
        let mut template_context = TemplateContext::new(
            &mut entity,
            &mut scene_entity_references
        );
        let my_struct_reflect = reflect_template.build(
            &mut template_context,
            &template as &dyn PartialReflect,
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