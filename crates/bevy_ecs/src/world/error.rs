//! Contains error types returned by methods on [`World`].
//!
//! [`World`]: crate::world::World

use std::any::TypeId;

use thiserror::Error;

use crate::{component::ComponentId, prelude::*, schedule::InternedScheduleLabel};

/// The error type returned by [`World::try_run_schedule`] if the provided schedule does not exist.
///
/// [`World::try_run_schedule`]: crate::world::World::try_run_schedule
#[derive(Error, Debug)]
#[error("The schedule with the label {0:?} was not found.")]
pub struct TryRunScheduleError(pub InternedScheduleLabel);

/// The error type returned by [`World::get_reflect`] and [`World::get_reflect_mut`].
#[cfg(feature = "bevy_reflect")]
#[derive(Error, Debug)]
pub enum GetComponentReflectError {
    /// There is no [`ComponentId`] corresponding to the given [`TypeId`].
    ///
    /// This is usually handled by calling [`App::register_type`] for the type corresponding to
    /// the given [`TypeId`].
    ///
    /// See the documentation for [`bevy_reflect`] for more information.
    ///
    /// [`App::register_type`]: ../../../bevy_app/struct.App.html#method.register_type
    #[error("No `ComponentId` corresponding to {0:?} found (did you call App::register_type()?)")]
    NoCorrespondingComponentId(TypeId),

    /// The given [`Entity`] does not have a [`Component`] corresponding to the given [`TypeId`].
    #[error("The given `Entity` {entity:?} does not have a `{component_name:?}` component ({component_id:?}, which corresponds to {type_id:?})")]
    EntityDoesNotHaveComponent {
        /// The given [`Entity`].
        entity: Entity,
        /// The given [`TypeId`].
        type_id: TypeId,
        /// The [`ComponentId`] corresponding to the given [`TypeId`].
        component_id: ComponentId,
        /// The name corresponding to the [`Component`] with the given [`TypeId`], or `None`
        /// if not available.
        component_name: Option<String>,
    },

    /// The [`World`] was missing the [`AppTypeRegistry`] resource.
    #[error("The `World` was missing the `AppTypeRegistry` resource")]
    MissingAppTypeRegistry,

    /// The [`World`]'s [`TypeRegistry`] did not contain [`TypeData`] for [`ReflectFromPtr`] for the given [`TypeId`].
    ///
    /// This is usually handled by calling [`App::register_type`] for the type corresponding to
    /// the given [`TypeId`].
    ///
    /// See the documentation for [`bevy_reflect`] for more information.
    ///
    /// [`TypeData`]: bevy_reflect::TypeData
    /// [`TypeRegistry`]: bevy_reflect::TypeRegistry
    /// [`ReflectFromPtr`]: bevy_reflect::ReflectFromPtr
    /// [`App::register_type`]: ../../../bevy_app/struct.App.html#method.register_type
    #[error("The `World`'s `TypeRegistry` did not contain `TypeData` for `ReflectFromPtr` for the given {0:?} (did you call `App::register_type()`?)")]
    MissingReflectFromPtrTypeData(TypeId),
}
