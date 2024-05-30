//! Show how to use DevCommands, DevTools and cli dev console

use std::any::Any;

use bevy::dev_tools::cli_deserialize::CliDeserializer;
use bevy::dev_tools::dev_command::{DevCommand, ReflectDevCommand};
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::dev_tools::DevCommand;
use bevy::ecs::world::Command;
use bevy::prelude::*;
use bevy::dev_tools::console_reader_plugin::{ConsoleInput, ConsoleReaderPlugin};
use bevy::dev_tools::prelude::*;
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

//We can create toggable dev tool
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default, Reflect)]
enum ShowGold {
    #[default]
    Show,
    Hide,
}

impl Toggable for ShowGold {
    fn enable(world: &mut World) {
        world.resource_mut::<NextState<ShowGold>>().set(ShowGold::Show);
    }

    fn disable(world: &mut World) {
        world.resource_mut::<NextState<ShowGold>>().set(ShowGold::Hide);
    }

    fn is_enabled(world: &World) -> bool {
        *world.resource::<State<ShowGold>>() == ShowGold::Show
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ConsoleReaderPlugin)
        .add_plugins(FpsOverlayPlugin::default())

        //register dev commands as usual types
        .register_type::<SetGold>() 
        .register_type::<PrintGold>()
        .register_type::<ShowGold>()
        .register_type::<Enable<ShowGold>>()
        .register_type::<Disable<ShowGold>>()

        .init_resource::<Gold>()
        .init_state::<ShowGold>()

        .add_systems(Update, parse_command)

        //dev tool example
        .add_systems(Update, show_gold_system.run_if(in_state(ShowGold::Show)))
        .add_systems(OnEnter(ShowGold::Show), create_gold_node)
        .add_systems(OnExit(ShowGold::Show), destroy_gold_node)

        .add_systems(Startup, setup)

        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct ShowGoldNode;

fn create_gold_node(mut commands: Commands) {
    commands.spawn(ShowGoldNode);
}

fn destroy_gold_node(mut commands: Commands, q_node: Query<Entity, With<ShowGoldNode>>) {
    if let Ok(node) = q_node.get_single() {
        commands.entity(node).despawn();
    }
}

fn show_gold_system(
    mut commands: Commands,
    mut q_node: Query<Entity, With<ShowGoldNode>>,
    gold : Res<Gold>,
) {
    if let Ok(node) = q_node.get_single() {
        commands.entity(node).insert(TextBundle::from_section(format!("Gold: {}", gold.0), TextStyle::default()));
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
