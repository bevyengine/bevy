use crate::{
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    renderer::{RenderContext, RenderDevice},
    texture_blitter::{TextureBlitter, TextureBlitterBuilder, TextureBlitterRenderPass},
    view::{Msaa, ViewTarget},
    Render, RenderApp, RenderSystems,
};

/// This enables "msaa writeback" support for the `core_2d` and `core_3d` pipelines, which can be enabled on cameras
/// using [`bevy_render::camera::Camera::msaa_writeback`]. See the docs on that field for more information.
pub struct MsaaWritebackPlugin;

impl Plugin for MsaaWritebackPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_msaa_writeback_pipelines.in_set(RenderSystems::Prepare),
        );
        {
            render_app
                .add_render_graph_node::<ViewNodeRunner<MsaaWritebackNode>>(
                    Core2d,
                    Node2d::MsaaWriteback,
                )
                .add_render_graph_edge(Core2d, Node2d::MsaaWriteback, Node2d::StartMainPass);
        }
        {
            render_app
                .add_render_graph_node::<ViewNodeRunner<MsaaWritebackNode>>(
                    Core3d,
                    Node3d::MsaaWriteback,
                )
                .add_render_graph_edge(Core3d, Node3d::MsaaWriteback, Node3d::StartMainPass);
        }
    }
}

#[derive(Default)]
pub struct MsaaWritebackNode;

impl ViewNode for MsaaWritebackNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static MsaaWritebackRequired,
        &'static MsaaWritebackTextureBlitter,
        &'static Msaa,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (target, _writeback_enabled, texture_blitter, msaa): QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        if *msaa == Msaa::Off {
            return Ok(());
        }

        // The current "main texture" needs to be bound as an input resource, and we need the "other"
        // unused target to be the "resolve target" for the MSAA write. Therefore this is the same
        // as a post process write!
        let post_process = target.post_process_write();
        render_context.blit_with_render_pass(
            &texture_blitter.texture_blitter,
            post_process.source,
            // If MSAA is enabled, then the sampled texture will always exist
            target.sampled_main_texture_view().unwrap(),
            &TextureBlitterRenderPass {
                clear_color: Some(wgpu::Color::BLACK),
                // The target's "resolve target" is the "destination" in post_process.
                // We will indirectly write the results to the "destination" using
                // the MSAA resolve step.
                resolve_target: Some(post_process.destination),
                ..Default::default()
            },
        );

        Ok(())
    }
}

#[derive(Component)]
pub struct MsaaWritebackTextureBlitter {
    // The values used to create the texture blitter
    // We need to keep them around to check if they changed between frames
    //
    // (msaa samples, main texture format)
    descriptor: (u32, wgpu::TextureFormat),
    // The texture blitter that will be used to do the writeback
    // It should be created based on the descriptor
    texture_blitter: TextureBlitter,
}

#[derive(Component)]
pub struct MsaaWritebackRequired;

fn prepare_msaa_writeback_pipelines(
    mut commands: Commands,
    view_targets: Query<(
        Entity,
        &ViewTarget,
        &ExtractedCamera,
        &Msaa,
        Option<&MsaaWritebackTextureBlitter>,
    )>,
    render_device: Res<RenderDevice>,
) {
    for (entity, view_target, camera, msaa, maybe_texture_blitter) in view_targets.iter() {
        // only do writeback if writeback is enabled for the camera and this isn't the first camera in the target,
        // as there is nothing to write back for the first camera.
        if msaa.samples() > 1 && camera.msaa_writeback && camera.sorted_camera_index_for_target > 0
        {
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(MsaaWritebackRequired);

            let new_descriptor = (msaa.samples(), view_target.main_texture_format());

            // We want to only create a new texture blitter if there isn't one or if the descriptor
            // has changed.
            //
            // We can't currently rely on change detection because those components update every
            // frame even if they haven't changed
            let mut create_new_texture_blitter = maybe_texture_blitter.is_none();
            if let Some(MsaaWritebackTextureBlitter { descriptor, .. }) = maybe_texture_blitter {
                if *descriptor != new_descriptor {
                    create_new_texture_blitter = true;
                }
            }

            if create_new_texture_blitter {
                // Create a new texture blitter based on the descriptor
                let texture_blitter =
                    TextureBlitterBuilder::new(render_device.wgpu_device(), new_descriptor.1)
                        .target_sample_count(new_descriptor.0)
                        .build();

                entity_commands.insert(MsaaWritebackTextureBlitter {
                    // Make sure to store the new descriptor
                    descriptor: new_descriptor,
                    texture_blitter,
                });
            }
        } else {
            commands
                .entity(entity)
                .remove::<MsaaWritebackRequired>()
                // Technically we could keep it around, but if writeback is never needed for this
                // camera in the future it's better to completely remove it
                .remove::<MsaaWritebackTextureBlitter>();
        }
    }
}
