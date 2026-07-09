use core::any::TypeId;
use bevy_reflect::{CreateTypeData, Reflect, TypeRegistry};
use crate::reflect::template::ReflectTemplate;
use crate::template::FromTemplate;

#[derive(Clone)]
pub struct ReflectFromTemplate(ReflectFromTemplateFns);

#[derive(Clone)]
pub struct ReflectFromTemplateFns {
    pub get_template: fn(&TypeRegistry) -> Option<&ReflectTemplate>
}

impl ReflectFromTemplateFns {
    pub fn new<T: Reflect + FromTemplate>() -> Self {
        <ReflectFromTemplate as CreateTypeData<T>>::create_type_data(()).0
    }
}

impl ReflectFromTemplate {
    pub fn get_template<'a>(&self, registry: &'a TypeRegistry) -> Option<&'a ReflectTemplate> {
        (self.0.get_template)(registry)
    }

    pub fn new(fns: ReflectFromTemplateFns) -> Self {
        Self(fns)
    }

    pub fn fn_pointer(&self) -> &ReflectFromTemplateFns {
        &self.0
    }
}

impl<B: Reflect + FromTemplate> CreateTypeData<B> for ReflectFromTemplate {
    fn create_type_data(_input: ()) -> Self {
        ReflectFromTemplate(ReflectFromTemplateFns {
            get_template: |registry: &TypeRegistry| {
                let registration = registry.get(TypeId::of::<B::Template>());

                let Some(registration) = registration else {
                    return None
                };

                registration.data::<ReflectTemplate>()
            },
        })
    }
}