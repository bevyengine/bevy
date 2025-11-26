//! Demonstrates how to write a custom mesh pass
//!
//! The `MeshPass` in Bevy is designed for creating render passes that draw meshes with a set of new shaders in `Material`.
//!
//! This is useful for creating custom prepasses or implementing techniques like Inverted Hull Outline.

use bevy::{
    camera::{MainPassResolutionOverride, Viewport},
    core_pipeline::core_3d::{
        graph::{Core3d, Node3d},
        Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey,
    },
    ecs::query::QueryItem,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{
        BinnedPhaseFamily, DrawMaterial, ExtendedMaterial, MainPass, MaterialExtension,
        MaterialExtensionKey, MaterialExtensionPipeline, MaterialPipelineSpecializer, MeshPass,
        MeshPassPlugin, NoExtractCondition, PIEPhase, PassShaders, PhaseContext, PhaseItemExt,
        RenderPhaseType, ShaderSet,
    },
    prelude::*,
    render::{
        camera::ExtractedCamera,
        diagnostic::RecordDiagnostics,
        extract_component::ExtractComponent,
        render_graph::{RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner},
        render_phase::{
            BinnedPhaseItem, BinnedRenderPhaseType, TrackedRenderPass, ViewBinnedRenderPhases,
        },
        render_resource::{
            AsBindGroup, CommandEncoderDescriptor, Face, RenderPassDescriptor,
            RenderPipelineDescriptor, SpecializedMeshPipelineError, StoreOp,
        },
        renderer::RenderContext,
        view::{ExtractedView, ViewDepthTexture, ViewTarget},
        RenderApp,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/custom_mesh_pass_material.wgsl";

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, OutlineExtension>>::default(),
        MeshPassPlugin::<OutlinePass>::default(),
    ))
    // You can use `register_required_components` to add our `OutlinePass` to all cameras.
    // Example: .register_required_components::<Camera3d, OutlinePass>()
    .add_systems(Startup, setup);

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .add_render_graph_node::<ViewNodeRunner<OutlinePassNode>>(Core3d, OutlinePassLabel)
        .add_render_graph_edges(
            Core3d,
            (
                Node3d::MainOpaquePass,
                OutlinePassLabel,
                Node3d::MainTransmissivePass,
            ),
        );

    app.run();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct OutlinePassLabel;

#[derive(Clone, Copy, Default, Component, ExtractComponent)]
struct OutlinePass;

impl MeshPass for OutlinePass {
    type ViewKeySource = MainPass;
    type Specializer = MaterialPipelineSpecializer;
    type PhaseItems = OutlineOpaque3d;
    type RenderCommand = DrawMaterial;
}

#[derive(BinnedPhaseItem)]
struct OutlineOpaque3d(Opaque3d);

impl PhaseItemExt for OutlineOpaque3d {
    type PhaseFamily = BinnedPhaseFamily<Self>;
    type ExtractCondition = NoExtractCondition;
    const PHASE_TYPES: RenderPhaseType = RenderPhaseType::Opaque;

    fn queue(render_phase: &mut PIEPhase<Self>, context: &PhaseContext) {
        let (vertex_slab, index_slab) = context
            .mesh_allocator
            .mesh_slabs(&context.mesh_instance.mesh_asset_id);

        render_phase.add(
            Opaque3dBatchSetKey {
                pipeline: context.pipeline_id,
                draw_function: context.draw_function,
                material_bind_group_index: Some(context.material.binding.group.0),
                vertex_slab: vertex_slab.unwrap_or_default(),
                index_slab,
                lightmap_slab: context
                    .mesh_instance
                    .shared
                    .lightmap_slab_index
                    .map(|index| *index),
            },
            Opaque3dBinKey {
                asset_id: context.mesh_instance.mesh_asset_id.into(),
            },
            (context.entity, context.main_entity),
            context.mesh_instance.current_uniform_index,
            BinnedRenderPhaseType::mesh(
                context.mesh_instance.should_batch(),
                &context.gpu_preprocessing_support,
            ),
            context.current_change_tick,
        );
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct OutlineExtension {
    #[uniform(100)]
    outline_color: LinearRgba,
}

impl MaterialExtension for OutlineExtension {
    fn shaders() -> PassShaders {
        let mut pass_shaders = PassShaders::default();
        pass_shaders.extend([
            (MainPass::id(), ShaderSet::default()),
            (
                OutlinePass::id(),
                ShaderSet {
                    vertex: SHADER_ASSET_PATH.into(),
                    fragment: SHADER_ASSET_PATH.into(),
                },
            ),
        ]);
        pass_shaders
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.pass_id == OutlinePass::id() {
            descriptor.primitive.cull_mode = Some(Face::Front);
        }
        Ok(())
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, OutlineExtension>>>,
) {
    // Cube
    commands.spawn((
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.75),
                // opaque_render_method: OpaqueRendererMethod::Forward,
                ..default()
            },
            extension: OutlineExtension {
                outline_color: Color::srgb(0.6, 0.9, 0.70).to_linear(),
            },
        })),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh())),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        // We are not using `register_required_components`, so let's manually
        // mark the camera for rendering the custom pass.
        OutlinePass,
    ));

    // Light
    commands.spawn((
        SpotLight::default(),
        Transform::from_xyz(4.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Default)]
struct OutlinePassNode;

impl ViewNode for OutlinePassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        Option<&'static MainPassResolutionOverride>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, extracted_view, target, depth, resolution_override): QueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), bevy_render::render_graph::NodeRunError> {
        let Some(opaque_phases) = world.get_resource::<ViewBinnedRenderPhases<OutlineOpaque3d>>()
        else {
            return Ok(());
        };

        let Some(opaque_phase) = opaque_phases.get(&extracted_view.retained_view_entity) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let color_attachments = [Some(target.get_color_attachment())];
        let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

        let view_entity = graph.view_entity();
        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _main_opaque_pass_3d_span = info_span!("outline_opaque_pass_3d").entered();

            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("outline_opaque_pass_3d_command_encoder"),
                });

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("outline_opaque_pass_3d"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
            let pass_span = diagnostics.pass_span(&mut render_pass, "outline_opaque_pass_3d");

            if let Some(viewport) =
                Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
            {
                render_pass.set_camera_viewport(&viewport);
            }

            // Opaque draws
            if !opaque_phase.is_empty() {
                #[cfg(feature = "trace")]
                let _opaque_main_pass_3d_span = info_span!("opaque_outline_pass_3d").entered();
                if let Err(err) = opaque_phase.render(&mut render_pass, world, view_entity) {
                    error!("Error encountered while rendering the outline opaque phase {err:?}");
                }
            }

            pass_span.end(&mut render_pass);
            drop(render_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}
