use bevy_dev_tools_macros::DevCommand;
use bevy_ecs::{reflect::AppTypeRegistry, system::Resource, world::Command};
use bevy_log::error;
use bevy_reflect::{serde::ReflectDeserializer, FromReflect, GetPath, GetTypeRegistration, Reflect, TypePath};
use serde::de::DeserializeSeed;
use crate::dev_command::{DevCommand, ReflectDevCommand};



pub trait DevTool : Reflect + FromReflect + GetTypeRegistration  {

}

pub struct ReflectDevTool {

}

#[derive(Default, Reflect)]
#[reflect(DevCommand)]
pub struct SetTool<T: DevTool + Resource + Default + Reflect + FromReflect + TypePath> {
    val: T
}

impl<T: DevTool + Default + Reflect + FromReflect + Resource + TypePath> DevCommand for SetTool<T> {}
impl<T: DevTool + Default + Reflect + FromReflect + Resource + TypePath> Command for SetTool<T> {
    fn apply(self, world: &mut bevy_ecs::world::World) {
        world.insert_resource(self.val);
    }
}

#[derive(Default, Reflect)]
#[reflect(DevCommand)]
pub struct SetField<T: Resource + Default + Reflect + FromReflect + TypePath> {
    field_path: String,
    val: String,

    #[reflect(ignore)]
    _marker: std::marker::PhantomData<T>
}

impl<T: Default + Reflect + FromReflect + Resource + TypePath> DevCommand for SetField<T> {}
impl<T: Default + Reflect + FromReflect + Resource + TypePath> Command for SetField<T> {
    fn apply(self, world: &mut bevy_ecs::world::World) {

        let app_registry = world.resource::<AppTypeRegistry>().clone();
        let registry = app_registry.read();
        let Some(mut target) = world.get_resource_mut::<T>() else {
            error!("Resource {} not found", std::any::type_name::<T>());
            return;
        };

        let SetField {
            field_path,
            val,
            _marker
        } = self;

        let Ok(mut field) = 
            target.as_mut().reflect_path_mut(
                &*field_path) else {
            error!("Field {} not found", field_path);
            return;
        };

        let ref_des = ReflectDeserializer::new(&registry);
        let Ok(mut ron_des) = ron::Deserializer::from_str(&val) else {
            error!("Failed to deserialize: {}", val);
            return;
        };
        let val = ref_des.deserialize(&mut ron_des).unwrap();
        field.apply(val.as_ref());

    }
}