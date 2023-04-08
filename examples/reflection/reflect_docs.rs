//! Demonstrates the reflection of docmentation .
use bevy::prelude::*;
use bevy::reflect::{TypeInfo, Typed};

/// The struct that defines our player.
///
/// # Example
///
/// ```
/// let player = Player::new("Urist McPlayer");
/// ```
#[derive(Reflect)]
struct Player {
    /// The player's name.
    name: String,
    /// The player's current health points.
    hp: u8,
    // Some regular comment (i.e. not a doc comment)
    max_hp: u8,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // You must manually register each instance of a generic type
        .register_type::<Player>()
        .add_systems(Startup, setup)
        .run();
}

fn setup() {
    let player_info = <Player as Typed>::type_info();

    // From here, we already have access to the struct's docs:
    let player_docs = player_info.docs().unwrap();
    assert_eq!(" The struct that defines our player.\n\n # Example\n\n ```\n let player = Player::new(\"Urist McPlayer\");\n ```", player_docs);
    info!("=====[ Player ]=====\n{player_docs}");

    // We can then iterate through our struct's fields to get their documentation as well:
    if let TypeInfo::Struct(struct_info) = player_info {
        for field in struct_info.iter() {
            let field_name = field.name();
            let field_docs = field.docs().unwrap_or("<NO_DOCUMENTATION>");
            info!("-----[ Player::{field_name} ]-----\n{field_docs}");
        }
    }
}