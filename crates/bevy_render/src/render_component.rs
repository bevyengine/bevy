use crate::RenderSet::Cleanup;
use crate::{Render, RenderApp};
use bevy_app::{App, Plugin};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Component;
use bevy_ecs::query::With;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::system::{Commands, Query};

pub use bevy_render_macros::RenderComponent;


/// A plugin that registers a component used to indicate that an entity should be rendered using
/// a particular render pipeline. These components are automatically removed from entities every
/// frame and must be re-added if the entity should continue to be rendered using the given
/// pipeline.
pub struct RenderComponentPlugin<C>(std::marker::PhantomData<C>);

impl<C: RenderComponent> Plugin for RenderComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(Render, cleanup_render_component::<C>.in_set(Cleanup));
        }
    }
}

impl <C> Default for RenderComponentPlugin<C> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

/// Marker trait for components that are used to indicate that an entity should be rendered using a
/// particular render pipeline.
pub trait RenderComponent: Component {}

fn cleanup_render_component<C: RenderComponent>(
    mut commands: Commands,
    components: Query<Entity, With<C>>,
) {
    for entity in components.iter() {
        commands.entity(entity).remove::<C>();
    }
}
