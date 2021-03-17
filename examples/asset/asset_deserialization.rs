use bevy::{prelude::*, reflect::TypeUuid};
use serde::Deserialize;

#[derive(Debug, Deserialize, TypeUuid)]
#[uuid = "abfa1180-abfa-41fa-80a1-2af49dd778fd"]
pub struct Character {
    name: String,
    hit_points: u32,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_asset::<Character>()
        .add_startup_system(setup.system())
        .add_system(character_asset_event.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let character: Handle<Character> = asset_server.deserialize("data/character.ron");

    // attach the handle to an entity so that it won't be cleaned
    commands.spawn((character,));
}

pub fn character_asset_event(
    mut events: EventReader<AssetEvent<Character>>,
    assets: Res<Assets<Character>>,
) {
    for event in events.iter() {
        let handle = match event {
            AssetEvent::Created { handle }
            | AssetEvent::Removed { handle }
            | AssetEvent::Modified { handle } => handle,
        };
        if let Some(character) = assets.get(handle) {
            info!("{:?}", character);
        }
    }
}
