use core::marker::PhantomData;

use bevy_app::{App, AppLabel, InternedAppLabel, Plugin};
use bevy_ecs::prelude::*;
pub use bevy_extract_macros::ExtractResource;
use bevy_utils::once;

use crate::{Extract, ExtractSchedule};

/// Describes how a resource gets extracted for rendering.
///
/// Therefore the resource is transferred from the "main world" into the "render world"
/// in the [`ExtractSchedule`] step.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractResourcePlugin`].
pub trait ExtractResource<F = ()>: Resource {
    type Source: Resource;

    /// Defines how the resource is transferred into the "render world".
    fn extract_resource(source: &Self::Source) -> Self;
}

/// This plugin extracts the resources into the "render world".
///
/// Therefore it sets up the[`ExtractSchedule`] step
/// for the specified [`Resource`].
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractResourcePlugin`].
pub struct ExtractResourcePlugin<R: ExtractResource<F>, F = ()> {
    marker: PhantomData<(R, F)>,

    /// The [`AppLabel`](bevy_app::AppLabel) of the [`SubApp`] to set up with extraction.
    pub app_label: InternedAppLabel,
}

impl <R: ExtractResource<F>, F> ExtractResourcePlugin<R, F> {
    pub fn new<L: AppLabel>(app: L) -> Self {
        Self {
            marker: PhantomData,
            app_label: app.intern(),
        }
    }
}

impl<R: ExtractResource<F>, F: 'static + Send + Sync> Plugin for ExtractResourcePlugin<R, F> {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(self.app_label) {
            render_app.add_systems(ExtractSchedule, extract_resource::<R, F>);
        } else {
            once!(bevy_log::error!(
                "Render app did not exist when trying to add `extract_resource` for <{}>.",
                core::any::type_name::<R>()
            ));
        }
    }
}

/// This system extracts the resource of the corresponding [`Resource`] type
pub fn extract_resource<R: ExtractResource<F>, F>(
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
                    "Removing resource {} from render world not expected, adding using `Commands`.
                This may decrease performance",
                    core::any::type_name::<R>()
                ));
            }

            commands.insert_resource(R::extract_resource(main_resource));
        }
    }
}
