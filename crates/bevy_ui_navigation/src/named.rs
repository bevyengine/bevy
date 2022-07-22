//! Declare menu navigation through [`Name`].
//!
//! The most difficult part of the API to deal with was giving
//! [`MenuSetting::reachable_from`](crate::MenuSetting::reachable_from) the `Entity` for of
//! the button used to reach it.
//!
//! This forced you to divide the whole menu construction in multiple
//! parts and keep track of intermediary values if you want to make multple menus.
//!
//! *By-name declaration* let you simply add a label to your `Focusable` and
//! refer to it in [`MenuSetting::reachable_from_named`](crate::MenuSetting::reachable_from_named).
//! The runtime then detects labelled stuff and replace the partial
//! [`MenuSetting`](crate::MenuSetting) with the full [`TreeMenu`](crate::resolve::TreeMenu)
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
                        "Tried to spawn a menu with parent focusable {name}, but no\
                         `Focusable` has a `Name` component with that value."
                    );
                    commands.entity(entity).remove::<MenuBuilder>();
                }
            }
        }
    }
}
