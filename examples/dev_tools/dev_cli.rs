//! Show how to use DevCommands, DevTools and cli dev console

use std::any::Any;

use bevy::dev_tools::cli_deserialize::CliDeserializer;
use bevy::dev_tools::dev_command::{DevCommand, ReflectDevCommand};
use bevy::dev_tools::DevCommand;
use bevy::ecs::world::Command;
use bevy::prelude::*;
use bevy::dev_tools::console_reader_plugin::{ConsoleInput, ConsoleReaderPlugin};
use bevy::reflect::serde::*;
use serde::de::DeserializeSeed;

#[derive(Resource, Default)]
pub struct Gold(pub usize);

#[derive(Reflect, Default, DevCommand)]
#[reflect(DevCommand, Default)]
pub struct SetGold {
    pub gold: usize,
}
impl Command for SetGold {
    fn apply(self, world: &mut World) {
        world.insert_resource(Gold(self.gold));
    }
}

#[derive(Reflect, Default, DevCommand)]
#[reflect(DevCommand, Default)]
pub struct PrintGold {}

impl Command for PrintGold {
    fn apply(self, world: &mut World) {
        let gold = world.get_resource::<Gold>().unwrap();
        info!("Gold: {}", gold.0);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ConsoleReaderPlugin)

        .register_type::<SetGold>()
        .register_type::<PrintGold>()

        .init_resource::<Gold>()

        .add_systems(Update, parse_command)

        .run();
}

fn parse_command(
    mut commands: Commands,
    mut console_input: EventReader<ConsoleInput>,
    app_registry: Res<AppTypeRegistry>
) {
    for input in console_input.read() {
        match input {
            ConsoleInput::Text(text) => {
                let registry = app_registry.read();
                let des = CliDeserializer::from_str(text.as_str(), &registry).unwrap();
                let refl_des = ReflectDeserializer::new(&registry);

                if let Ok(boxed_cmd) = refl_des.deserialize(des) {
                    // println!("Deserialized command: {:?}", boxed_cmd);
                    // println!("Type path: {:?}", boxed_cmd.get_represented_type_info().unwrap().type_path());
                    let Some(type_info) = registry.get_with_type_path(boxed_cmd.get_represented_type_info().unwrap().type_path()) else {
                        println!("Failed to get type info");
                        continue;
                    };

                    let Some(dev_command_data) = registry.get_type_data::<ReflectDevCommand>(type_info.type_id()) else {
                        println!("Failed to get dev command metadata");
                        continue;
                    };

                    (dev_command_data.metadata.self_to_commands)(boxed_cmd.as_ref(), &mut commands);
                } else {
                    println!("Failed to deserialize command");
                }
            }
            _ => {}
        }
    }
    console_input.clear();
}
