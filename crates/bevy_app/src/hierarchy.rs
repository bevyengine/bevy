use core::marker::PhantomData;

use bevy_ecs::{
    change_detection::MaybeLocation,
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    intern::Interned,
    lifecycle::Insert,
    message::{Message, MessageReader, MessageWriter},
    name::Name,
    observer::On,
    query::{With, Without},
    schedule::{common_conditions::on_message, IntoScheduleConfigs, ScheduleLabel, SystemSet},
    system::Query,
};
use bevy_platform::prelude::format;
use bevy_utils::prelude::DebugName;
use log::warn;

use crate::{Last, Plugin};

/// A plugin that verifies that [`Component`] `C` has parents that also have that component.
pub struct ValidateParentHasComponentPlugin<C: Component> {
    schedule: Interned<dyn ScheduleLabel>,
    marker: PhantomData<fn() -> C>,
}

impl<C: Component> Default for ValidateParentHasComponentPlugin<C> {
    fn default() -> Self {
        Self::in_schedule(Last)
    }
}

impl<C: Component> ValidateParentHasComponentPlugin<C> {
    /// Creates an instance of this plugin that inserts systems in the provided schedule.
    pub fn in_schedule(label: impl ScheduleLabel) -> Self {
        Self {
            schedule: label.intern(),
            marker: PhantomData,
        }
    }
}

impl<C: Component> Plugin for ValidateParentHasComponentPlugin<C> {
    fn build(&self, app: &mut crate::App) {
        app.add_message::<CheckParentHasComponent<C>>()
            .add_observer(validate_parent_has_component::<C>)
            .add_systems(
                self.schedule,
                check_parent_has_component::<C>
                    .run_if(on_message::<CheckParentHasComponent<C>>)
                    .in_set(ValidateParentHasComponentSystems),
            );
    }
}

/// System set for systems added by [`ValidateParentHasComponentPlugin`].
#[derive(SystemSet, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ValidateParentHasComponentSystems;

/// An `Insert` observer that when run, will validate that the parent of a given entity contains
/// component `C`. If the parent does not contain `C`, a warning will be logged later in the frame.
fn validate_parent_has_component<C: Component>(
    event: On<Insert, C>,
    child: Query<&ChildOf>,
    with_component: Query<(), With<C>>,
    mut writer: MessageWriter<CheckParentHasComponent<C>>,
) {
    let Ok(child_of) = child.get(event.entity) else {
        return;
    };
    if with_component.contains(child_of.parent()) {
        return;
    }
    // This entity may be configured incorrectly, or the parent may just not have been populated
    // yet. Send a message to check again later.
    writer.write(CheckParentHasComponent::<C> {
        entity: event.entity,
        caller: event.caller(),
        marker: PhantomData,
    });
}

/// A message to indicate that this entity should be checked if its parent has a component.
///
/// While we initially check when emitting these messages, we want to do a second check later on in
/// case the parent eventually gets populated.
#[derive(Message)]
struct CheckParentHasComponent<C: Component> {
    /// The entity
    entity: Entity,
    caller: MaybeLocation,
    marker: PhantomData<fn() -> C>,
}

/// System to handle "check parent" messages and log out any entities that still violate the
/// component hierarchy.
fn check_parent_has_component<C: Component>(
    mut messages: MessageReader<CheckParentHasComponent<C>>,
    children: Query<(&ChildOf, Option<&Name>), With<C>>,
    components: Query<Option<&Name>, Without<C>>,
) {
    for CheckParentHasComponent {
        entity,
        caller,
        marker: _,
    } in messages.read()
    {
        let Ok((child_of, name)) = children.get(*entity) else {
            // Either the entity has been despawned, no longer has `C`, or is no longer a child. In
            // any case, we can say that this situation is no longer relevant.
            continue;
        };
        let parent = child_of.0;
        let Ok(parent_name) = components.get(parent) else {
            // This can only fail if the parent now has the `C` component. If the parent was
            // despawned, the child entity would also be despawned.
            continue;
        };
        let debug_name = DebugName::type_name::<C>();
        warn!(
            "warning[B0004]: {}{name} with the {ty_name} component has a parent ({parent_name}) without {ty_name}.\n\
            This will cause inconsistent behaviors! See: https://bevy.org/learn/errors/b0004",
            caller.map(|c| format!("{c}: ")).unwrap_or_default(),
            ty_name = debug_name.shortname(),
            name = name.map_or_else(
                || format!("Entity {entity}"),
                |s| format!("The {s} entity")
            ),
            parent_name = parent_name.map_or_else(
                || format!("{parent} entity"),
                |s| format!("the {s} entity")
            ),
        );
    }
}
