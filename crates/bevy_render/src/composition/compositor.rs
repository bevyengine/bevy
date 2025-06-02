use bevy_asset::Assets;
use bevy_color::{Color, LinearRgba};
use bevy_ecs::{
    component::{Component, HookContext},
    entity::{ContainsEntity, Entity, EntityHashSet},
    event::Event,
    observer::Trigger,
    query::{Has, QueryEntityError, QueryState, With, Without},
    system::{lifetimeless::Read, Commands, Local, Query, Res, Single},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_platform::sync::Arc;
use bevy_reflect::Reflect;
use bevy_window::{PrimaryWindow, Window};
use core::{iter::Copied, ops::Deref};

use crate::{
    render_graph::{
        InternedRenderSubGraph, Node, NodeRunError, RenderGraphContext, RenderLabel,
        RenderSubGraph, ViewNode,
    },
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp, Texture,
        TextureView,
    },
    renderer::RenderContext,
    sync_world::{RenderEntity, SyncToRenderWorld},
    Extract,
};

use super::{
    render_target::{ExtractedWindows, NormalizedRenderTarget, RenderTarget, RenderTargetInfo},
    ExtractedView, ManualTextureViews, RenderGraphDriver, SubView, View, ViewTarget,
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
        self.0.iter().copied()
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

pub(super) fn handle_compositor_events(
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
        }
    }
}

// -----------------------------------------------------------------------------
// Extraction / Render World Logic

#[derive(Component)]
pub struct ExtractedCompositor {
    views: Vec<Entity>,
    target: Arc<(NormalizedRenderTarget, RenderTargetInfo)>,
}

pub(super) fn extract_compositors(
    compositors: Extract<Query<(RenderEntity, &Compositor, &Views)>>,
    mapper: Extract<Query<RenderEntity, With<View>>>,
    mut commands: Commands,
) {
    for (render_entity, compositor, views) in &compositors {
        let extracted_views: Vec<Entity> = views
            .iter()
            .filter_map(|view| mapper.get(view).ok())
            .collect();

        let extracted_compositor = compositor
            .target
            .clone()
            .filter(|_| !extracted_views.is_empty())
            .map(|target| ExtractedCompositor {
                views: extracted_views,
                target,
            });

        if let Some(extracted_compositor) = extracted_compositor {
            commands.entity(render_entity).insert(extracted_compositor);
        } else {
            commands
                .entity(render_entity)
                .remove::<ExtractedCompositor>();
        }
    }
}

pub struct MainCompositorTexture(Texture);

// -----------------------------------------------------------------------------
// Render Graph

pub struct RunCompositorsNode {
    compositors: QueryState<(Entity, Read<ExtractedCompositor>, Read<RenderGraphDriver>)>,
}

impl Node for RunCompositorsNode {
    fn update(&mut self, world: &mut World) {
        self.compositors.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let mut rendered_windows = EntityHashSet::default();
        for (entity, compositor, compositor_render_graph) in self.compositors.query_manual(world) {
            /* TODO: include this logic in main world, while processing targets on the compositor
            if let Some(NormalizedRenderTarget::Window(window_ref)) = camera.target {
                let window_entity = window_ref.entity();
                if windows
                    .windows
                    .get(&window_entity)
                    .is_some_and(|w| w.physical_width > 0 && w.physical_height > 0)
                {
                    camera_windows.insert(window_entity);
                } else {
                    // The window doesn't exist anymore or zero-sized so we don't need to run the graph
                    run_graph = false;
                }
            }
            */
            if let NormalizedRenderTarget::Window(window_ref) = compositor.target.0 {
                rendered_windows.insert(window_ref.entity());
            }

            if let Err(err) =
                graph.run_sub_graph(*compositor_render_graph.deref(), vec![], Some(entity))
            {
                return Err(err.into());
            }
        }

        // wgpu (and some backends) require doing work for swap chains if you call `get_current_texture()` and `present()`
        // This ensures that Bevy doesn't crash, even when there are no cameras (and therefore no work submitted).
        for (entity, window) in world.resource::<ExtractedWindows>().iter() {
            if rendered_windows.contains(entity) {
                continue;
            }

            let Some(swap_chain_texture) = &window.swap_chain_texture_view else {
                continue;
            };

            #[cfg(feature = "trace")]
            let _span = tracing::info_span!("no_camera_clear_pass").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("window"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: swap_chain_texture,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(LinearRgba::BLACK.into()), // TODO: use ClearColor resource still?
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, RenderSubGraph)]
pub struct CompositorGraph;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, RenderLabel)]
pub enum CompositorNodes {
    RunViews,
    BlitToSurface,
}

pub struct RunViewsNode {
    compositors: QueryState<Read<ExtractedCompositor>>,
    views: QueryState<Read<RenderGraphDriver>, With<ExtractedView>>,
}

impl FromWorld for RunViewsNode {
    fn from_world(world: &mut World) -> Self {
        todo!()
    }
}

impl Node for RunViewsNode {
    fn update(&mut self, world: &mut World) {
        self.compositors.update_archetypes(world);
        self.views.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let compositor_query = self.compositors.query_manual(world);
        let view_query = self.views.query_manual(world);

        let Ok(compositor) = compositor_query.get(graph.view_entity()) else {
            return Ok(());
        };

        for view in &compositor.views {
            let Ok(view_render_graph) = view_query.get(*view) else {
                continue;
            };
            if let Err(err) = graph.run_sub_graph(*view_render_graph.deref(), vec![], Some(*view)) {
                return Err(err.into());
            }
        }

        Ok(())
    }
}

pub struct BlitToSurfaceNode {
    blitter: wgpu::util::TextureBlitter,
    //TODO: setup new types for managing and passing around surfaces
    // compositors: QueryState<(Read<MainCompositorTexture>, Read<RenderSurface>), With<ExtractedCompositor>>,
    compositors: QueryState<(), With<ExtractedCompositor>>,
}

impl FromWorld for BlitToSurfaceNode {
    fn from_world(world: &mut World) -> Self {
        todo!()
    }
}

impl Node for BlitToSurfaceNode {
    fn update(&mut self, world: &mut World) {
        self.compositors.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let compositor_query = self.compositors.query_manual(world);

        self.blitter.copy(
            render_context.render_device().wgpu_device(),
            render_context.command_encoder(),
            todo!(),
            todo!(),
        );

        Ok(())
    }
}
