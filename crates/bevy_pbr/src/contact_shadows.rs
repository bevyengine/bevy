//! Contact shadows implemented via screenspace raymarching.

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{DynamicUniformBuffer, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    view::ExtractedView,
    Render, RenderApp, RenderSystems,
};
use bevy_utils::default;

/// Enables contact shadows for a camera.
pub struct ContactShadowsPlugin;

/// Add this component to a camera to enable contact shadows.
///
/// Contact shadows are a screen-space technique that adds small-scale shadows
/// in areas where traditional shadow maps may lack detail, such as where
/// objects touch the ground.
///
/// This can be used in forward or deferred rendering, but the depth prepass is required.
#[derive(Clone, Copy, Component, Reflect)]
#[reflect(Component, Default, Clone)]
#[require(bevy_core_pipeline::prepass::DepthPrepass)]
pub struct ContactShadows {
    /// The number of steps to be taken at regular intervals to find an initial
    /// intersection.
    pub linear_steps: u32,
    /// When marching the depth buffer, we only have 2.5D information and don't
    /// know how thick surfaces are. We shall assume that the depth buffer
    /// fragments are cuboids with a constant thickness defined by this
    /// parameter.
    pub thickness: f32,
    /// The length of the contact shadow ray in world space.
    pub length: f32,
}

impl Default for ContactShadows {
    fn default() -> Self {
        Self {
            linear_steps: 16,
            thickness: 0.1,
            length: 0.3,
        }
    }
}

/// A version of [`ContactShadows`] for upload to the GPU.
#[derive(Clone, Copy, Component, ShaderType, Default)]
pub struct ContactShadowsUniform {
    pub linear_steps: u32,
    pub thickness: f32,
    pub length: f32,
    #[cfg(feature = "webgl")]
    pub _padding: f32,
}

impl From<ContactShadows> for ContactShadowsUniform {
    fn from(settings: ContactShadows) -> Self {
        Self {
            linear_steps: settings.linear_steps,
            thickness: settings.thickness,
            length: settings.length,
            #[cfg(feature = "webgl")]
            _padding: 0.0,
        }
    }
}

impl ExtractComponent for ContactShadows {
    type QueryData = &'static ContactShadows;
    type QueryFilter = ();
    type Out = ContactShadows;

    fn extract_component(settings: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(*settings)
    }
}

/// A GPU buffer that stores the contact shadow settings for each view.
#[derive(Resource, Default)]
pub struct ContactShadowsBuffer(pub DynamicUniformBuffer<ContactShadowsUniform>);

impl Plugin for ContactShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ContactShadows>()
            .add_plugins(ExtractComponentPlugin::<ContactShadows>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ContactShadowsBuffer>()
            .add_systems(
                Render,
                prepare_contact_shadows_settings.in_set(RenderSystems::PrepareResources),
            );
    }
}

fn prepare_contact_shadows_settings(
    mut commands: Commands,
    views: Query<(Entity, Option<&ContactShadows>), With<ExtractedView>>,
    mut contact_shadows_buffer: ResMut<ContactShadowsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    contact_shadows_buffer.0.clear();
    for (entity, settings) in &views {
        let uniform = if let Some(settings) = settings {
            ContactShadowsUniform::from(*settings)
        } else {
            ContactShadowsUniform {
                linear_steps: 0,
                ..default()
            }
        };
        let offset = contact_shadows_buffer.0.push(&uniform);
        commands
            .entity(entity)
            .insert(ViewContactShadowsUniformOffset(offset));
    }
    contact_shadows_buffer
        .0
        .write_buffer(&render_device, &render_queue);
}

/// A component that stores the offset within the [`ContactShadowsBuffer`] for
/// each view.
#[derive(Component, Default, Deref, DerefMut)]
pub struct ViewContactShadowsUniformOffset(pub u32);
