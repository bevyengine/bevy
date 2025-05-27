use bevy_asset::Assets;
use bevy_ecs::{
    component::{Component, HookContext},
    entity::{ContainsEntity, Entity},
    event::Event,
    observer::Trigger,
    query::{Has, QueryEntityError, With, Without},
    system::{Commands, Local, Query, Res, Single},
};
use bevy_image::Image;
use bevy_platform::sync::Arc;
use bevy_window::{PrimaryWindow, Window};
use core::iter::Copied;
use tracing::warn;

use crate::{
    render_graph::{InternedRenderSubGraph, RenderSubGraph},
    sync_world::{RenderEntity, SyncToRenderWorld},
    Extract,
};

use super::{
    render::CompositorGraph, ManualTextureViews, NormalizedRenderTarget, RenderGraphDriver,
    RenderTarget, RenderTargetInfo, SubView, View, ViewTarget,
};

// -----------------------------------------------------------------------------
// Core Compositor Types

#[derive(Component, Default)]
#[require(
    RenderTarget,
    Views,
    RenderGraphDriver::new(CompositorGraph),
    SyncToRenderWorld
)]
pub struct Compositor {
    target: Option<Arc<(NormalizedRenderTarget, RenderTargetInfo)>>,
}

#[derive(Component)]
#[relationship(relationship_target = Views)]
pub struct CompositedBy(pub Entity);

impl ContainsEntity for CompositedBy {
    fn entity(&self) -> Entity {
        self.0
    }
}

//TODO: need to modify relationship hooks to trigger compositor events
//TODO: make an analogue of `children!` that works for views
#[derive(Component, Default)]
#[relationship_target(relationship = CompositedBy)]
pub struct Views(Vec<Entity>);

impl<'a> IntoIterator for &'a Views {
    type Item = Entity;

    type IntoIter = Copied<<&'a Vec<Entity> as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter().copied()
    }
}

impl FromIterator<Entity> for Views {
    fn from_iter<T: IntoIterator<Item = Entity>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl Views {
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        self.into_iter()
    }
}

// -----------------------------------------------------------------------------
// Compositor Events

#[derive(Event)]
#[event(auto_propagate, traversal = &'static CompositedBy)]
pub(super) enum CompositorEvent {
    CompositorChanged,
    ViewChanged(Entity),
}

//TODO: handle window events

fn handle_compositor_events(
    trigger: Trigger<CompositorEvent>,
    mut compositors: Query<(&mut Compositor, &RenderTarget, &Views), Without<CompositedBy>>,
    mut views: Query<(&View, Option<&SubView>)>,
    primary_window: Option<Single<Entity, With<PrimaryWindow>>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut commands: Commands,
) {
    let Ok((mut compositor, render_target, composited_views)) =
        compositors.get_mut(trigger.target())
    else {
        // events propagate up the compositor's tree, so the target may not be a compositor yet
        return;
    };

    fn update_compositor<'a>(
        compositor: &mut Compositor,
        render_target: &RenderTarget,
        primary_window: Option<Entity>,
        windows: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) {
        compositor.target = render_target
            .normalize(primary_window)
            .and_then(|normalized_target| {
                Some(normalized_target.clone()).zip(normalized_target.get_render_target_info(
                    windows,
                    images,
                    manual_texture_views,
                ))
            })
            .map(Arc::new);
    }

    fn update_view(
        compositor: &Compositor,
        view: Entity,
        mut views: Query<(&View, Option<&SubView>)>,
        mut commands: Commands,
    ) {
        let Some(target) = &compositor.target else {
            //todo: warn about invalid compositor state;
            return;
        };

        match views.get_mut(view) {
            Ok((View::Enabled, sub_view)) => {
                let viewport =
                    sub_view.map(|sub_view| sub_view.get_viewport(target.1.physical_size));
                let new_target = ViewTarget {
                    target: target.clone(),
                    viewport,
                };
                commands.entity(view).insert(new_target);
            }
            Ok((View::Disabled, ..)) => {
                // view was disabled, remove its target
                commands.entity(view).remove::<ViewTarget>();
            }
            Err(QueryEntityError::QueryDoesNotMatch(..)) => {
                // if entity is not a view, we should remove it from the relationship
                commands.entity(view).remove::<(ViewTarget, CompositedBy)>();
            }
            // view was despawned, ignore.
            _ => {}
        }
    }

    match *trigger.event() {
        CompositorEvent::CompositorChanged => {
            update_compositor(
                &mut compositor,
                render_target,
                primary_window.as_deref().copied(),
                windows,
                &images,
                &manual_texture_views,
            );

            composited_views.iter().for_each(|view| {
                update_view(&compositor, view, views.reborrow(), commands.reborrow());
            });
        }
        CompositorEvent::ViewChanged(view) => {
            update_view(&compositor, view, views, commands);
        } // this compositor
    }
}

// -----------------------------------------------------------------------------
// Extraction / Render World Logic

#[derive(Component)]
pub struct ExtractedCompositor {
    views: Vec<Entity>,
    target: Arc<(NormalizedRenderTarget, RenderTargetInfo)>,
    sub_graph: InternedRenderSubGraph,
}

pub(super) fn extract_compositors(
    main_compoitors: Extract<Query<(RenderEntity, &Compositor, &Views, &RenderGraphDriver)>>,
    render_compositors: Query<&mut ExtractedCompositor>,
    main_views: Extract<Query<RenderEntity, With<View>>>,
    mut commands: Commands,
) {
}
