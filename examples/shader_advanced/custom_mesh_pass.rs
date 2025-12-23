//! Demonstrates how to write a custom mesh pass
//!
//! The `MeshPass` in Bevy is designed for creating render passes that draw meshes with a set of new shaders in `Material`.
//!
//! This is useful for creating custom prepasses or implementing techniques like Inverted Hull Outline.

use bevy::{
    camera::{MainPassResolutionOverride, Viewport},
    core_pipeline::core_3d::{
        graph::{Core3d, Node3d},
        Opaque3d, Transparent3d,
    },
    ecs::query::QueryItem,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{
        BinnedPhaseItem, DrawMaterial, ExtendedMaterial, MainPass, MaterialExtension,
        MaterialExtensionKey, MaterialExtensionPipeline, MaterialPipelineSpecializer, MeshPass,
        MeshPassPlugin, NoExtractCondition, PassShaders, PhaseContext, PhaseItemExt,
        QueueSortedPhaseItem, RenderPhaseType, ShaderSet, SortedPhaseFamily, SortedPhaseItem,
    },
    prelude::*,
    render::{
        camera::ExtractedCamera,
        diagnostic::RecordDiagnostics,
        extract_component::ExtractComponent,
        render_graph::{RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner},
        render_phase::{BinnedRenderPhase, SortedRenderPhase, TrackedRenderPass},
        render_resource::{
            AsBindGroup, CommandEncoderDescriptor, Face, RenderPassDescriptor,
            RenderPipelineDescriptor, SpecializedMeshPipelineError, StoreOp,
        },
        renderer::RenderContext,
        view::{ViewDepthTexture, ViewTarget},
        RenderApp,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/custom_mesh_pass_material.wgsl";

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, OutlineExtension>>::default(),
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, TransparentOutlineExtension>>::default(
        ),
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
    type PhaseItems = (OutlineOpaque3d, OutlineTransparent3d);
}

// Single-field tuple structs can automatically forward all phase item traits from the inner type.
#[derive(BinnedPhaseItem)]
struct OutlineOpaque3d(Opaque3d);

// Other struct forms have some limitations:
// - `#[derive(BinnedPhaseItem)]` cannot derive `BinnedPhaseItem` automatically
// - `#[derive(SortedPhaseItem)]` cannot derive `QueueSortedPhaseItem` automatically
// - We have to skip those unsupported traits explicitly.
#[derive(SortedPhaseItem)]
struct OutlineTransparent3d {
    // For demonstration, we also skip `PhaseItemExt`.
    #[phase_item(skip(QueueSortedPhaseItem, PhaseItemExt))]
    inner: Transparent3d,
}

// The APIs of `QueueBinnedPhaseItem` and `QueueSortedPhaseItem` are quite different.
// For more details, check their definitions.
impl QueueSortedPhaseItem for OutlineTransparent3d {
    fn get_item(context: &PhaseContext) -> Option<Self> {
        Transparent3d::get_item(context).map(|inner| Self { inner })
    }
}

impl PhaseItemExt for OutlineTransparent3d {
    type PhaseFamily = SortedPhaseFamily;
    type ExtractCondition = NoExtractCondition;
    type RenderCommand = DrawMaterial;
    const PHASE_TYPES: RenderPhaseType = RenderPhaseType::Transparent;
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

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct TransparentOutlineExtension {
    #[uniform(100)]
    outline_color: LinearRgba,
}

impl MaterialExtension for TransparentOutlineExtension {
    fn shaders() -> PassShaders {
        let mut pass_shaders = PassShaders::default();
        pass_shaders.extend([
            (MainPass::id(), ShaderSet::default()),
            (
                OutlinePass::id(),
                // For simplicity, we are using the same shader for both opaque and transparent passes.
                ShaderSet {
                    vertex: SHADER_ASSET_PATH.into(),
                    fragment: SHADER_ASSET_PATH.into(),
                },
            ),
        ]);
        pass_shaders
    }

    fn alpha_mode() -> Option<AlphaMode> {
        Some(AlphaMode::Blend)
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, OutlineExtension>>>,
    mut transparent_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, TransparentOutlineExtension>>,
    >,
) {
    // Cube
    commands.spawn((
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.75),
                ..default()
            },
            extension: OutlineExtension {
                outline_color: Color::srgb(0.6, 0.9, 0.70).to_linear(),
            },
        })),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh())),
        Transform::from_xyz(0.0, 0.5, -0.5),
    ));

    // Sphere
    commands.spawn((
        MeshMaterial3d(transparent_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgba(0.75, 0.75, 1.0, 0.5),
                alpha_mode: AlphaMode::Blend,
                ..default()
            },
            extension: TransparentOutlineExtension {
                outline_color: Color::srgb(0.75, 0.6, 0.2).to_linear(),
            },
        })),
        Mesh3d(meshes.add(Sphere::new(0.5).mesh())),
        Transform::from_xyz(0.0, 0.5, 1.0),
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
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static BinnedRenderPhase<OutlineOpaque3d>,
        &'static SortedRenderPhase<OutlineTransparent3d>,
        Option<&'static MainPassResolutionOverride>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, target, depth, opaque_phase, transparent_phase, resolution_override): QueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), bevy_render::render_graph::NodeRunError> {
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
            let pass_span = diagnostics.pass_span(&mut render_pass, "outline_pass");

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

            // Transparent draws
            if !transparent_phase.items.is_empty() {
                #[cfg(feature = "trace")]
                let _transparent_main_pass_3d_span =
                    info_span!("transparent_outline_pass_3d").entered();
                if let Err(err) = transparent_phase.render(&mut render_pass, world, view_entity) {
                    error!(
                        "Error encountered while rendering the outline transparent phase {err:?}"
                    );
                }
            }

            pass_span.end(&mut render_pass);
            drop(render_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}
