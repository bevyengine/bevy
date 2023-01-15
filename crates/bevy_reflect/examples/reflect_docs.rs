//! This example illustrates how you can reflect doc comments.
//!
//! There may be cases where you may want to collect a reflected item's documentation.
//! For example, you may want to generate schemas or other external documentation for scripting.
//! Or perhaps you want your custom editor to display tooltips for certain properties that match the documentation.
//!
//! These scenarios can readily be achieved by using `bevy_reflect` with the `documentation` feature.

use bevy_reflect::{Reflect, TypeInfo, Typed};

fn main() {
    //! This function will simply demonstrate how you can access a type's documentation.
    //!
    //! Please note that the code below uses a standard struct with named fields; however, this isn't
    //! exclusive to them. It can work for all kinds of data types including tuple structs and enums too!

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

    // Using `TypeInfo` we can access all of the doc comments on the `Player` struct above:
    let player_info = <Player as Typed>::type_info();

    // From here, we already have access to the struct's docs:
    let player_docs = player_info.docs().unwrap();
    assert_eq!(" The struct that defines our player.\n\n # Example\n\n ```\n let player = Player::new(\"Urist McPlayer\");\n ```", player_docs);
    println!("=====[ Player ]=====\n{player_docs}");

    // We can then iterate through our struct's fields to get their documentation as well:
    if let TypeInfo::Struct(struct_info) = player_info {
        for field in struct_info.iter() {
            let field_name = field.name();
            let field_docs = field.docs().unwrap_or("<NO_DOCUMENTATION>");
            println!("-----[ Player::{field_name} ]-----\n{field_docs}");
        }
    }
}
