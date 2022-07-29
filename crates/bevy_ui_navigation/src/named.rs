//! Declare menu navigation through [`Name`].
//!
//! The most difficult part of the API when creating UI scenes,
//! was using [`MenuBuilder::EntityParent`],
//! providing the [`Entity`] for the [`Focusable`] the menu is reachable from,
//! forced users to separate and order the creation of their menus.
//!
//! *By-name declaration* let you simply add a [`Name`] to your [`Focusable`]
//! and refer to it in [`MenuBuilder::NamedParent`].
//!
//! The runtime then detects labelled stuff
//! and replace the partial [`MenuBuilder`]
//! with the full [`TreeMenu`] with the proper entity id reference.
//! This saves you from pre-spawning your buttons
//! so that you can associate their `id` with the proper submenu.
//!
//! [`TreeMenu`]: crate::resolve::TreeMenu

use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    prelude::With,
    system::{Commands, Query},
};

use crate::{menu::MenuBuilder, resolve::Focusable};

pub(crate) fn resolve_named_menus(
    mut commands: Commands,
    mut unresolved: Query<(Entity, &mut MenuBuilder)>,
    named: Query<(Entity, &Name), With<Focusable>>,
) {
    for (entity, mut builder) in &mut unresolved {
        if let MenuBuilder::NamedParent(parent_name) = builder.clone() {
            match named.iter().find(|(_, n)| **n == parent_name) {
                Some((focus_parent, _)) => {
                    *builder = MenuBuilder::EntityParent(focus_parent);
                }
                None => {
                    let name = parent_name.as_str();
                    bevy_log::warn!(
                        "Tried to spawn a menu with parent focusable {name}, but no\
                         `Focusable` has a `Name` component with that value."
                    );
                    commands.entity(entity).remove::<MenuBuilder>();
                }
            }
        }
    }
}
