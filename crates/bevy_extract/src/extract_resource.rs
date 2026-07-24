use core::marker::PhantomData;

use bevy_app::{App, AppLabel, Plugin};
use bevy_ecs::{component::Mutable, prelude::*};
pub use bevy_extract_macros::ExtractResource;
use bevy_utils::once;

use crate::{Extract, ExtractSchedule};

/// Describes how a resource gets extracted for processing.
///
/// Therefore the resource is transferred from the "main world" into the "sub world"
/// in the [`ExtractSchedule`] step.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractResourcePlugin`].
pub trait ExtractResource<L: AppLabel, F = ()>: Resource {
    type Source: Resource;

    /// Defines how the resource is transferred into the "sub world".
    fn extract_resource(source: &Self::Source) -> Self;
}

/// This plugin extracts the resources into the "sub world".
///
/// Therefore it sets up the[`ExtractSchedule`] step
/// for the specified [`Resource`].
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractResourcePlugin`].
pub struct ExtractResourcePlugin<R: ExtractResource<L, F>, L: AppLabel, F = ()>(
    PhantomData<(R, L, F)>,
);

impl<R: ExtractResource<L, F>, L: AppLabel, F> Default for ExtractResourcePlugin<R, L, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<
        R: ExtractResource<L, F, Mutability = Mutable>,
        L: AppLabel + Default,
        F: 'static + Send + Sync,
    > Plugin for ExtractResourcePlugin<R, L, F>
{
    fn build(&self, app: &mut App) {
        if let Some(sub_app) = app.get_sub_app_mut(L::default()) {
            sub_app.add_systems(ExtractSchedule, extract_resource::<R, L, F>);
        } else {
            once!(bevy_log::error!(
                "Sub app did not exist when trying to add `extract_resource` for <{}>.",
                core::any::type_name::<R>()
            ));
        }
    }
}

/// This system extracts the resource of the corresponding [`Resource`] type
pub fn extract_resource<R: ExtractResource<L, F, Mutability = Mutable>, L: AppLabel, F>(
    mut commands: Commands,
    main_resource: Extract<Option<Res<R::Source>>>,
    target_resource: Option<ResMut<R>>,
) {
    if let Some(main_resource) = main_resource.as_ref() {
        if let Some(mut target_resource) = target_resource {
            if main_resource.is_changed() {
                *target_resource = R::extract_resource(main_resource);
            }
        } else {
            #[cfg(debug_assertions)]
            if !main_resource.is_added() {
                once!(bevy_log::warn!(
                    "Removing resource {} from sub world not expected, adding using `Commands`.
                This may decrease performance",
                    core::any::type_name::<R>()
                ));
            }

            commands.insert_resource(R::extract_resource(main_resource));
        }
    }
}
