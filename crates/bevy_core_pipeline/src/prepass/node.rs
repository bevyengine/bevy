use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    camera::ExtractedCamera,
    prelude::Color,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use super::{AlphaMask3dPrepass, Opaque3dPrepass, ViewPrepassTextures};

pub struct PrepassNode {
    main_view_query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Opaque3dPrepass>,
            &'static RenderPhase<AlphaMask3dPrepass>,
            &'static ViewDepthTexture,
            &'static ViewPrepassTextures,
        ),
        With<ExtractedView>,
    >,
}

impl PrepassNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
        }
    }
}

impl Node for PrepassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(PrepassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        if let Ok((
            camera,
            opaque_prepass_phase,
            alpha_mask_prepass_phase,
            view_depth_texture,
            view_prepass_textures,
        )) = self.main_view_query.get_manual(world, view_entity)
        {
            if opaque_prepass_phase.items.is_empty() && alpha_mask_prepass_phase.items.is_empty() {
                return Ok(());
            }

            let mut color_attachments = vec![];
            if let Some(view_normals_texture) = &view_prepass_textures.normals {
                color_attachments.push(Some(RenderPassColorAttachment {
                    view: &view_normals_texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK.into()),
                        store: true,
                    },
                }));
            }

            {
                // Set up the pass descriptor with the depth attachment and optional color attachments
                let pass_descriptor = RenderPassDescriptor {
                    label: Some("prepass"),
                    color_attachments: &color_attachments,
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &view_depth_texture.view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(0.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                };

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);
                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                if let Some(viewport) = camera.viewport.as_ref() {
                    tracked_pass.set_camera_viewport(viewport);
                }

                {
                    // Run the prepass, sorted front-to-back
                    #[cfg(feature = "trace")]
                    let _opaque_prepass_span = info_span!("opaque_prepass").entered();
                    let draw_functions = world.resource::<DrawFunctions<Opaque3dPrepass>>();

                    let mut draw_functions = draw_functions.write();
                    for item in &opaque_prepass_phase.items {
                        let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                        draw_function.draw(world, &mut tracked_pass, view_entity, item);
                    }
                }

                {
                    // Run the prepass, sorted front-to-back
                    #[cfg(feature = "trace")]
                    let _alpha_mask_prepass_span = info_span!("alpha_mask_prepass").entered();
                    let draw_functions = world.resource::<DrawFunctions<AlphaMask3dPrepass>>();

                    let mut draw_functions = draw_functions.write();
                    for item in &alpha_mask_prepass_phase.items {
                        let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                        draw_function.draw(world, &mut tracked_pass, view_entity, item);
                    }
                }
            }

            if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
                // Copy depth buffer to texture
                render_context.command_encoder.copy_texture_to_texture(
                    view_depth_texture.texture.as_image_copy(),
                    prepass_depth_texture.texture.as_image_copy(),
                    view_prepass_textures.size,
                );
            }
        }

        Ok(())
    }
}
