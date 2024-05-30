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
        .add_plugins(CLIToolbox)
        .add_plugins(FpsOverlayPlugin::default())

        //register dev commands as usual types
        .register_type::<SetGold>() 
        .register_type::<PrintGold>()
        .register_type::<ShowGold>()
        .register_type::<Enable<ShowGold>>()
        .register_type::<Disable<ShowGold>>()

        .init_resource::<Gold>()
        .init_state::<ShowGold>()

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
        commands.entity(node).insert(TextBundle::from_section(format!("Gold: {}", gold.0), TextStyle::default()))
            .insert(Style {
                position_type: PositionType::Absolute,
                right: Val::Px(10.),
                ..default()
            });
    }
}
