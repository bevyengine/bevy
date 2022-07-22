//! Declare menu navigation through
//! [`Name`](https://docs.rs/bevy/0.8.0/bevy/core/struct.Name.html).
//!
//! The most difficult part of the API to deal with was giving
//! [`NavMenu::reachable_from`](crate::NavMenu::reachable_from) the `Entity` for of
//! the button used to reach it.
//!
//! This forced you to divide the whole menu construction in multiple
//! parts and keep track of intermediary values if you want to make multple menus.
//!
//! *By-name declaration* let you simply add a label to your `Focusable` and
//! refer to it in [`NavMenu::reachable_from_named`](crate::NavMenu::reachable_from_named).
//! The runtime then detects labelled stuff and replace the partial
//! [`NavMenu`](crate::NavMenu) with the full [`TreeMenu`](crate::resolve::TreeMenu)
//! with the proper entity id reference. This saves you from pre-spawning your
//! buttons so that you can associate their `id` with the proper submenu.

use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    prelude::With,
    system::{Commands, Query},
};

use crate::{seeds::MenuBuilder, Focusable};

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
                        "Tried to spawn a `NavMenu` with parent focusable {name}, but no\
                         `Focusable` has a `Name` component with that value."
                    );
                    commands.entity(entity).remove::<MenuBuilder>();
                }
            }
        }
    }
}
