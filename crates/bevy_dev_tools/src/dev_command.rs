use bevy_ecs::world::Command;
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect};


trait DevCommand : Command + Default + Reflect + FromReflect + GetTypeRegistration {

}

struct ReflectDevCommand {
    pub name: String,
    
}