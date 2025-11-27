//! Low-level API for custom rendering tasks (graphics or compute).
//!
//! See [`RenderTask`] for more details.

/// Types for binding buffers or textures to your render passes.
pub mod bind;

mod compute_builder;
mod node;
mod plugin;
mod resource_cache;

pub use compute_builder::ComputeCommandBuilder;
pub use node::RenderTaskContext;
pub use plugin::RenderTaskPlugin;

use crate::{
    extract_component::ExtractComponent,
    render_graph::{IntoRenderNodeArray, RenderLabel, RenderSubGraph},
    settings::{WgpuFeatures, WgpuLimits},
};
use bevy_app::{App, SubApp};
use bevy_ecs::{component::Component, entity::Entity, world::World};

/// Low-level API for custom rendering tasks (graphics or compute).
///
/// # Introduction
///
/// [`RenderTask`] is a low-level API for adding custom rendering work to your Bevy app.
///
/// It provides some convenient helpers to reduce CPU-code boilerplate of common [`bevy_render`] API usages within Bevy.
///
/// # Use cases
///
/// Bevy provides several different APIs, depending on how deeply you want to customize rendering:
/// - **High level** - `Material` - Intended for artists writing shaders to customize the visual appearance of a `Mesh`.
/// - **Mid level** - [`crate::render_phase::PhaseItem`] and [`crate::render_phase::RenderCommand`] - Intended for rendering engineers to customize how specific entities render, beyond Bevy's default of issuing a draw call with a vertex and index buffer.
/// - **Low level** - [`RenderTask`] (this trait, and [`bevy_render`] in general) - Intended for rendering engineers to add completely from-scratch rendering tasks associated with a `Camera`, e.g. a compute shader-based weather simulation.
/// - **Lowest level** - Writing your own renderer on top of bevy_mesh, bevy_camera, etc, without using [`bevy_render`] / [`wgpu`].
///
/// # What this trait does
///
/// This trait wraps several common pieces of functionality for creating a new rendering feature:
/// * Checking that the user's GPU supports the required [`WgpuFeatures`] and [`WgpuLimits`] to use the feature.
/// * Setting up a [`crate::render_graph::Node`].
/// * Syncing (extracting) a camera component in the main world to the render world.
/// * Creating and caching textures, buffers, bind groups, and pipelines.
/// * Binding resources and encoding draw/dispatch commands in render/compute passes.
/// * Adding profiling spans to passes.
///
/// # Usage
///
/// ## 1) Define your camera component
/// In Bevy, almost all rendering work is driven by cameras.
///
/// To add a new rendering task to your app, write a new component for a camera:
///
/// ```rust
/// #[derive(Component, ExtractComponent)]
/// struct MyRenderingFeature { /* ... */ }
///
/// impl RenderTask for MyRenderingFeature {
///     // ...
/// }
/// ```
///
/// ## 2) (Optional) Set required GPU features and limits
/// Tasks can optionally require certain GPU features and limits in order to run.
///
/// If the defaults (no required features, [`WgpuLimits::downlevel_webgl2_defaults()`]) are sufficient for your task, you may skip this step.
///
/// ```rust
/// const REQUIRED_FEATURES: WgpuFeatures = WgpuFeatures::SHADER_F64;
/// const REQUIRED_LIMITS: WgpuLimits = WgpuLimits { max_sampled_textures_per_shader_stage: 32, ..WgpuLimits::downlevel_webgl2_defaults() };
/// ```
///
/// ## 3) Setup a render node
/// Render nodes control the order that each piece of rendering work runs in, relative to other rendering work.
///
/// ```rust
/// #[derive(RenderLabel, Default)]
/// struct MyRenderingFeatureNode;
///
/// type RenderNodeSubGraph = Core3d; // Run as part of the Core3d render graph
///
/// fn render_node_label() -> impl RenderLabel {
///    MyRenderingFeatureNode
/// }
///
/// fn render_node_ordering() -> impl IntoRenderNodeArray {
///     (
///        Node3d::EndPrepasses,
///        Self::render_node_label(), // Run sometime after the end of the prepass rendering, and before the end of the main pass rendering
///        Node3d::EndMainPass,
///     )
/// }
/// ```
///
/// ## 4) (Optional) Add additional plugin setup code
/// If you need additional resources, systems, etc as part of your plugin, you can add them like this:
///
/// ```rust
/// fn plugin_app_build(app: &mut App) {
///     app.insert_resource(/* ... */);
/// }
///
/// fn plugin_render_app_build(render_app: &mut SubApp) {
///     render_app.add_systems(Render, /* ... */);
/// }
/// ```
///
/// ## 5) Encode commands
/// With the setup out of the way, you can now define the actual render work your task will do.
///
/// Create resources and encode render commands as follows:
///
/// ```rust
/// fn encode_commands(&self, mut ctx: RenderTaskContext, camera_entity: Entity, world: &World) -> Option<(); {
///     let (component_a, component_b) = world
///         .entity(entity)
///         .get_components::<(&ComponentA, &ComponentB)>()?;
///
///     let resource = world.get_resource::<ResourceC>()?;
///
///     if self.foo {
///         // ...
///     }
///
///     let texture = ctx.texture(TextureDescriptor { /* ... */ });
///     let buffer = ctx.buffer(BufferDescriptor { /* ... */ });
///
///     ctx.compute_pass("my_pass")
///         .shader(load_embedded_asset!(world, "my_shader.wgsl"))
///         .bind_resources((
///             SampledTexture(&texture),
///             StorageTextureReadWrite(&buffer),
///         ))
///         .dispatch_2d(10, 20)?;
///
///     Some(())
/// }
/// ```
///
/// ## 6) Use the plugin
/// Finally, you can add the task plugin to your app, and use it with your camera:
///
/// ```rust
/// app.add_plugins(RenderTaskPlugin::<MyRenderingFeature>::default());
///
/// commands.spawn((
///     Camera3d::default(),
///     MyRenderingFeature::new(),
/// ));
/// ```
pub trait RenderTask: Component + ExtractComponent {
    /// What render graph the task should run it.
    type RenderNodeSubGraph: RenderSubGraph + Default;

    /// Render node label for the task.
    fn render_node_label() -> impl RenderLabel;

    /// Ordering to run render nodes in.
    fn render_node_ordering() -> impl IntoRenderNodeArray;

    /// Required GPU features for the task.
    ///
    /// Defaults to [`WgpuFeatures::empty()`].
    const REQUIRED_FEATURES: WgpuFeatures = WgpuFeatures::empty();

    /// Required GPU limits for the task.
    ///
    /// Defaults to [`WgpuLimits::downlevel_webgl2_defaults()`].
    const REQUIRED_LIMITS: WgpuLimits = WgpuLimits::downlevel_webgl2_defaults();

    /// Optional additional plugin setup for the main app.
    #[expect(unused_variables)]
    fn plugin_app_build(app: &mut App) {}

    /// Optional additional plugin setup for the render app.
    #[expect(unused_variables)]
    fn plugin_render_app_build(render_app: &mut SubApp) {}

    /// Function to encode render commands for the task.
    ///
    /// This is where you create textures, run shaders, etc.
    fn encode_commands(
        &self,
        ctx: RenderTaskContext,
        camera_entity: Entity,
        world: &World,
    ) -> Option<()>;
}
