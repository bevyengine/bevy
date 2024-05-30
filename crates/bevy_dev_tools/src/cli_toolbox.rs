use bevy_app::{Plugin, PreUpdate, Update};
use bevy_ecs::{event::EventReader, reflect::AppTypeRegistry, system::{Commands, Res}};
use bevy_reflect::serde::ReflectDeserializer;
use serde::de::DeserializeSeed;
use crate::{cli_deserialize::CliDeserializer, console_reader_plugin::{ConsoleInput, ConsoleReaderPlugin}, dev_command::ReflectDevCommand};


pub struct CLIToolbox;

impl Plugin for CLIToolbox {
    fn build(&self, app: &mut bevy_app::App) {
        if !app.is_plugin_added::<ConsoleReaderPlugin>() {
            app.add_plugins(ConsoleReaderPlugin);
        }
        app.add_systems(PreUpdate, parse_command);
    }
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
