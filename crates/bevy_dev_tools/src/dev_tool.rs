use std::any::Any;

use bevy_dev_tools_macros::DevCommand;
use bevy_ecs::{reflect::AppTypeRegistry, system::Resource, world::Command};
use bevy_log::{error, info};
use bevy_reflect::{serde::ReflectDeserializer, DynamicStruct, FromReflect, GetPath, GetTypeRegistration, Reflect, ReflectFromReflect, TypePath};
use serde::de::DeserializeSeed;
use crate::{dev_command::{DevCommand, ReflectDevCommand}, toggable::{Disable, Enable, Toggable}};



pub trait DevTool : Reflect + FromReflect + GetTypeRegistration  {

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

        let SetField { field_path, val, _marker } = self;

        let Ok(field) = target.reflect_path_mut(field_path.as_str()) else {
            error!("Field {} not found", field_path);
            return;
        };


        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let Ok(value) = ron::from_str::<ron::Value>(&val) else {
            error!("Failed to parse value {}", val);
            return;
        };

        info!("Set value {}", val);

        match field.reflect_mut() {
            bevy_reflect::ReflectMut::Struct(s) => {
                
            },
            bevy_reflect::ReflectMut::TupleStruct(_) => todo!(),
            bevy_reflect::ReflectMut::Tuple(_) => todo!(),
            bevy_reflect::ReflectMut::List(_) => todo!(),
            bevy_reflect::ReflectMut::Array(_) => todo!(),
            bevy_reflect::ReflectMut::Map(_) => todo!(),
            bevy_reflect::ReflectMut::Enum(_) => todo!(),
            bevy_reflect::ReflectMut::Value(v) => {
                if let Some(v) = v.downcast_mut::<usize>() {
                    if let Ok(input_value) = ron::de::from_str::<usize>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as usize", val);
                    }
                } else if let Some(v) = v.downcast_mut::<bool>() {
                    if let Ok(input_value) = ron::de::from_str::<bool>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as bool", val);
                    }
                } else if let Some(v) = v.downcast_mut::<String>() {
                    *v = val;
                } else if let Some(v) = v.downcast_mut::<f32>() {
                    if let Ok(input_value) = ron::de::from_str::<f32>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as f32", val);
                    }
                } else if let Some(v) = v.downcast_mut::<f64>() {
                    if let Ok(input_value) = ron::de::from_str::<f64>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as f64", val);
                    }
                } else if let Some(v) = v.downcast_mut::<i8>() {
                    if let Ok(input_value) = ron::de::from_str::<i8>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as i8", val);
                    }
                } else if let Some(v) = v.downcast_mut::<i16>() {
                    if let Ok(input_value) = ron::de::from_str::<i16>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as i16", val);
                    }
                } else if let Some(v) = v.downcast_mut::<i32>() {
                    if let Ok(input_value) = ron::de::from_str::<i32>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as i32", val);
                    }
                } else if let Some(v) = v.downcast_mut::<i64>() {
                    if let Ok(input_value) = ron::de::from_str::<i64>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as i64", val);
                    }
                } else if let Some(v) = v.downcast_mut::<i128>() {
                    if let Ok(input_value) = ron::de::from_str::<i128>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as i128", val);
                    }
                } else if let Some(v) = v.downcast_mut::<u8>() {
                    if let Ok(input_value) = ron::de::from_str::<u8>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as u8", val);
                    }
                } else if let Some(v) = v.downcast_mut::<u16>() {
                    if let Ok(input_value) = ron::de::from_str::<u16>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as u16", val);
                    }
                } else if let Some(v) = v.downcast_mut::<u32>() {
                    if let Ok(input_value) = ron::de::from_str::<u32>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as u32", val);
                    }
                } else if let Some(v) = v.downcast_mut::<u64>() {
                    if let Ok(input_value) = ron::de::from_str::<u64>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as u64", val);
                    }
                } else if let Some(v) = v.downcast_mut::<u128>() {
                    if let Ok(input_value) = ron::de::from_str::<u128>(val.to_string().as_str()) {
                        *v = input_value;
                    } else {
                        error!("Failed to parse value {} as u128", val);
                    }
                } else {
                    error!("Failed to set value {} to {}", val, std::any::type_name::<T>());
                }
            },
        }
        
    }
}


pub trait AppDevTool {
    fn register_toggable_dev_tool<T: DevTool + Resource + std::default::Default + bevy_reflect::TypePath + Toggable>(&mut self) -> &mut Self;
}


impl AppDevTool for bevy_app::App {
    fn register_toggable_dev_tool<T: DevTool + Resource + std::default::Default + bevy_reflect::TypePath + Toggable>(&mut self) -> &mut Self {
        self.register_type::<T>();
        self.register_type::<SetTool<T>>();
        self.register_type::<SetField<T>>();
        self.register_type::<Enable<T>>();
        self.register_type::<Disable<T>>();
        self
    }
}