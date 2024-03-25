use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
pub use bevy_render_macros::ExtractResource;

use crate::{Extract, ExtractSchedule, RenderApp};

/// Describes how a resource gets extracted for rendering.
///
/// Therefore the resource is transferred from the "main world" into the "render world"
/// in the [`ExtractSchedule`] step.
///
/// See also [`crate::extract_event::ExtractEvent`] for extracting events.
pub trait ExtractResource: Resource {
    type Source: Resource;

    /// Defines how the resource is transferred into the "render world".
    fn extract_resource(source: &Self::Source) -> Self;
}

/// This plugin extracts the resources into the "render world".
///
/// Therefore it sets up the[`ExtractSchedule`] step
/// for the specified [`Resource`].
///
/// See also [`crate::extract_event::ExtractEventPlugin`] for extracting events.
pub struct ExtractResourcePlugin<R: ExtractResource>(PhantomData<R>);

impl<R: ExtractResource> Default for ExtractResourcePlugin<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<R: ExtractResource> Plugin for ExtractResourcePlugin<R> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, extract_resource::<R>);
        }
    }
}

/// This system extracts the resource of the corresponding [`Resource`] type
pub fn extract_resource<R: ExtractResource>(
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
                bevy_utils::warn_once!(
                    "Removing resource {} from render world not expected, adding using `Commands`.
                This may decrease performance",
                    std::any::type_name::<R>()
                );
            }
            commands.insert_resource(R::extract_resource(main_resource));
        }
    }
}
