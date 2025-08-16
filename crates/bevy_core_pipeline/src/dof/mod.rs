//! Depth of field, a postprocessing effect that simulates camera focus.
//!
//! By default, Bevy renders all objects in full focus: regardless of depth, all
//! objects are rendered perfectly sharp (up to output resolution). Real lenses,
//! however, can only focus on objects at a specific distance. The distance
//! between the nearest and furthest objects that are in focus is known as
//! [depth of field], and this term is used more generally in computer graphics
//! to refer to the effect that simulates focus of lenses.
//!
//! Attaching [`DepthOfField`] to a camera causes Bevy to simulate the
//! focus of a camera lens. Generally, Bevy's implementation of depth of field
//! is optimized for speed instead of physical accuracy. Nevertheless, the depth
//! of field effect in Bevy is based on physical parameters.
//!
//! [Depth of field]: https://en.wikipedia.org/wiki/Depth_of_field

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::{Camera3d, PhysicalCameraParameters, Projection};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::World,
};
use bevy_image::BevyDefault as _;
use bevy_math::ops;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphContext, RenderGraphExt as _, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer,
        },
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FilterMode, FragmentState, LoadOp,
        Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, StoreOp,
        TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    },
    renderer::{RenderContext, RenderDevice},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, TextureCache},
    view::{
        prepare_view_targets, ExtractedView, Msaa, ViewDepthTexture, ViewTarget, ViewUniform,
        ViewUniformOffset, ViewUniforms,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::{default, once};
use smallvec::SmallVec;
use tracing::{info, warn};

use crate::{
    core_3d::{
        graph::{Core3d, Node3d},
        DEPTH_TEXTURE_SAMPLING_SUPPORTED,
    },
    FullscreenShader,
};

/// A plugin that adds support for the depth of field effect to Bevy.
pub struct DepthOfFieldPlugin;

/// A component that enables a [depth of field] postprocessing effect when attached to a [`Camera3d`],
/// simulating the focus of a camera lens.
///
/// [depth of field]: https://en.wikipedia.org/wiki/Depth_of_field
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct DepthOfField {
    /// The appearance of the effect.
    pub mode: DepthOfFieldMode,

    /// The distance in meters to the location in focus.
    pub focal_distance: f32,

    /// The height of the [image sensor format] in meters.
    ///
    /// Focal length is derived from the FOV and this value. The default is
    /// 18.66mm, matching the [Super 35] format, which is popular in cinema.
    ///
    /// [image sensor format]: https://en.wikipedia.org/wiki/Image_sensor_format
    ///
    /// [Super 35]: https://en.wikipedia.org/wiki/Super_35
    pub sensor_height: f32,

    /// Along with the focal length, controls how much objects not in focus are
    /// blurred.
    pub aperture_f_stops: f32,

    /// The maximum diameter, in pixels, that we allow a circle of confusion to be.
    ///
    /// A circle of confusion essentially describes the size of a blur.
    ///
    /// This value is nonphysical but is useful for avoiding pathologically-slow
    /// behavior.
    pub max_circle_of_confusion_diameter: f32,

    /// Objects are never considered to be farther away than this distance as
    /// far as depth of field is concerned, even if they actually are.
    ///
    /// This is primarily useful for skyboxes and background colors. The Bevy
    /// renderer considers them to be infinitely far away. Without this value,
    /// that would cause the circle of confusion to be infinitely large, capped
    /// only by the `max_circle_of_confusion_diameter`. As that's unsightly,
    /// this value can be used to essentially adjust how "far away" the skybox
    /// or background are.
    pub max_depth: f32,
}

/// Controls the appearance of the effect.
#[derive(Clone, Copy, Default, PartialEq, Debug, Reflect)]
#[reflect(Default, Clone, PartialEq)]
pub enum DepthOfFieldMode {
    /// A more accurate simulation, in which circles of confusion generate
    /// "spots" of light.
    ///
    /// For more information, see [Wikipedia's article on *bokeh*].
    ///
    /// This doesn't work on WebGPU.
    ///
    /// [Wikipedia's article on *bokeh*]: https://en.wikipedia.org/wiki/Bokeh
    Bokeh,

    /// A faster simulation, in which out-of-focus areas are simply blurred.
    ///
    /// This is less accurate to actual lens behavior and is generally less
    /// aesthetically pleasing but requires less video memory bandwidth.
    ///
    /// This is the default.
    ///
    /// This works on native and WebGPU.
    /// If targeting native platforms, consider using [`DepthOfFieldMode::Bokeh`] instead.
    #[default]
    Gaussian,
}

/// Data about the depth of field effect that's uploaded to the GPU.
#[derive(Clone, Copy, Component, ShaderType)]
pub struct DepthOfFieldUniform {
    /// The distance in meters to the location in focus.
    focal_distance: f32,

    /// The focal length. See the comment in `DepthOfFieldParams` in `dof.wgsl`
    /// for more information.
    focal_length: f32,

    /// The premultiplied factor that we scale the circle of confusion by.
    ///
    /// This is calculated as `focal_length² / (sensor_height *
    /// aperture_f_stops)`.
    coc_scale_factor: f32,

    /// The maximum circle of confusion diameter in pixels. See the comment in
    /// [`DepthOfField`] for more information.
    max_circle_of_confusion_diameter: f32,

    /// The depth value that we clamp distant objects to. See the comment in
    /// [`DepthOfField`] for more information.
    max_depth: f32,

    /// Padding.
    pad_a: u32,
    /// Padding.
    pad_b: u32,
    /// Padding.
    pad_c: u32,
}

/// A key that uniquely identifies depth of field pipelines.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepthOfFieldPipelineKey {
    /// Whether we're doing Gaussian or bokeh blur.
    pass: DofPass,
    /// Whether we're using HDR.
    hdr: bool,
    /// Whether the render target is multisampled.
    multisample: bool,
}

/// Identifies a specific depth of field render pass.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum DofPass {
    /// The first, horizontal, Gaussian blur pass.
    GaussianHorizontal,
    /// The second, vertical, Gaussian blur pass.
    GaussianVertical,
    /// The first bokeh pass: vertical and diagonal.
    BokehPass0,
    /// The second bokeh pass: two diagonals.
    BokehPass1,
}

impl Plugin for DepthOfFieldPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "dof.wgsl");

        app.add_plugins(UniformComponentPlugin::<DepthOfFieldUniform>::default());

        app.add_plugins(SyncComponentPlugin::<DepthOfField>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<DepthOfFieldPipeline>>()
            .init_resource::<DepthOfFieldGlobalBindGroup>()
            .add_systems(RenderStartup, init_dof_global_bind_group_layout)
            .add_systems(ExtractSchedule, extract_depth_of_field_settings)
            .add_systems(
                Render,
                (
                    configure_depth_of_field_view_targets,
                    prepare_auxiliary_depth_of_field_textures,
                )
                    .after(prepare_view_targets)
                    .in_set(RenderSystems::ManageViews),
            )
            .add_systems(
                Render,
                (
                    prepare_depth_of_field_view_bind_group_layouts,
                    prepare_depth_of_field_pipelines,
                )
                    .chain()
                    .in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Render,
                prepare_depth_of_field_global_bind_group.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_render_graph_node::<ViewNodeRunner<DepthOfFieldNode>>(Core3d, Node3d::DepthOfField)
            .add_render_graph_edges(
                Core3d,
                (Node3d::Bloom, Node3d::DepthOfField, Node3d::Tonemapping),
            );
    }
}

/// The node in the render graph for depth of field.
#[derive(Default)]
pub struct DepthOfFieldNode;

/// The layout for the bind group shared among all invocations of the depth of
/// field shader.
#[derive(Resource, Clone)]
pub struct DepthOfFieldGlobalBindGroupLayout {
    /// The layout.
    layout: BindGroupLayout,
    /// The sampler used to sample from the color buffer or buffers.
    color_texture_sampler: Sampler,
}

/// The bind group shared among all invocations of the depth of field shader,
/// regardless of view.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct DepthOfFieldGlobalBindGroup(Option<BindGroup>);

#[derive(Component)]
pub enum DepthOfFieldPipelines {
    Gaussian {
        horizontal: CachedRenderPipelineId,
        vertical: CachedRenderPipelineId,
    },
    Bokeh {
        pass_0: CachedRenderPipelineId,
        pass_1: CachedRenderPipelineId,
    },
}

struct DepthOfFieldPipelineRenderInfo {
    pass_label: &'static str,
    view_bind_group_label: &'static str,
    pipeline: CachedRenderPipelineId,
    is_dual_input: bool,
    is_dual_output: bool,
}

/// The extra texture used as the second render target for the hexagonal bokeh
/// blur.
///
/// This is the same size and format as the main view target texture. It'll only
/// be present if bokeh is being used.
#[derive(Component, Deref, DerefMut)]
pub struct AuxiliaryDepthOfFieldTexture(CachedTexture);

/// Bind group layouts for depth of field specific to a single view.
#[derive(Component, Clone)]
pub struct ViewDepthOfFieldBindGroupLayouts {
    /// The bind group layout for passes that take only one input.
    single_input: BindGroupLayout,

    /// The bind group layout for the second bokeh pass, which takes two inputs.
    ///
    /// This will only be present if bokeh is in use.
    dual_input: Option<BindGroupLayout>,
}

/// Information needed to specialize the pipeline corresponding to a pass of the
/// depth of field shader.
pub struct DepthOfFieldPipeline {
    /// The bind group layouts specific to each view.
    view_bind_group_layouts: ViewDepthOfFieldBindGroupLayouts,
    /// The bind group layout shared among all invocations of the depth of field
    /// shader.
    global_bind_group_layout: BindGroupLayout,
    /// The asset handle for the fullscreen vertex shader.
    fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    fragment_shader: Handle<Shader>,
}

impl ViewNode for DepthOfFieldNode {
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Read<ViewTarget>,
        Read<ViewDepthTexture>,
        Read<DepthOfFieldPipelines>,
        Read<ViewDepthOfFieldBindGroupLayouts>,
        Read<DynamicUniformIndex<DepthOfFieldUniform>>,
        Option<Read<AuxiliaryDepthOfFieldTexture>>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view_uniform_offset,
            view_target,
            view_depth_texture,
            view_pipelines,
            view_bind_group_layouts,
            depth_of_field_uniform_index,
            auxiliary_dof_texture,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let global_bind_group = world.resource::<DepthOfFieldGlobalBindGroup>();

        let diagnostics = render_context.diagnostic_recorder();

        // We can be in either Gaussian blur or bokeh mode here. Both modes are
        // similar, consisting of two passes each. We factor out the information
        // specific to each pass into
        // [`DepthOfFieldPipelines::pipeline_render_info`].
        for pipeline_render_info in view_pipelines.pipeline_render_info().iter() {
            let (Some(render_pipeline), Some(view_uniforms_binding), Some(global_bind_group)) = (
                pipeline_cache.get_render_pipeline(pipeline_render_info.pipeline),
                view_uniforms.uniforms.binding(),
                &**global_bind_group,
            ) else {
                return Ok(());
            };

            // We use most of the postprocess infrastructure here. However,
            // because the bokeh pass has an additional render target, we have
            // to manage a secondary *auxiliary* texture alongside the textures
            // managed by the postprocessing logic.
            let postprocess = view_target.post_process_write();

            let view_bind_group = if pipeline_render_info.is_dual_input {
                let (Some(auxiliary_dof_texture), Some(dual_input_bind_group_layout)) = (
                    auxiliary_dof_texture,
                    view_bind_group_layouts.dual_input.as_ref(),
                ) else {
                    once!(warn!(
                        "Should have created the auxiliary depth of field texture by now"
                    ));
                    continue;
                };
                render_context.render_device().create_bind_group(
                    Some(pipeline_render_info.view_bind_group_label),
                    dual_input_bind_group_layout,
                    &BindGroupEntries::sequential((
                        view_uniforms_binding,
                        view_depth_texture.view(),
                        postprocess.source,
                        &auxiliary_dof_texture.default_view,
                    )),
                )
            } else {
                render_context.render_device().create_bind_group(
                    Some(pipeline_render_info.view_bind_group_label),
                    &view_bind_group_layouts.single_input,
                    &BindGroupEntries::sequential((
                        view_uniforms_binding,
                        view_depth_texture.view(),
                        postprocess.source,
                    )),
                )
            };

            // Push the first input attachment.
            let mut color_attachments: SmallVec<[_; 2]> = SmallVec::new();
            color_attachments.push(Some(RenderPassColorAttachment {
                view: postprocess.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(default()),
                    store: StoreOp::Store,
                },
            }));

            // The first pass of the bokeh shader has two color outputs, not
            // one. Handle this case by attaching the auxiliary texture, which
            // should have been created by now in
            // `prepare_auxiliary_depth_of_field_textures``.
            if pipeline_render_info.is_dual_output {
                let Some(auxiliary_dof_texture) = auxiliary_dof_texture else {
                    once!(warn!(
                        "Should have created the auxiliary depth of field texture by now"
                    ));
                    continue;
                };
                color_attachments.push(Some(RenderPassColorAttachment {
                    view: &auxiliary_dof_texture.default_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(default()),
                        store: StoreOp::Store,
                    },
                }));
            }

            let render_pass_descriptor = RenderPassDescriptor {
                label: Some(pipeline_render_info.pass_label),
                color_attachments: &color_attachments,
                ..default()
            };

            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&render_pass_descriptor);
            let pass_span =
                diagnostics.pass_span(&mut render_pass, pipeline_render_info.pass_label);

            render_pass.set_pipeline(render_pipeline);
            // Set the per-view bind group.
            render_pass.set_bind_group(0, &view_bind_group, &[view_uniform_offset.offset]);
            // Set the global bind group shared among all invocations of the shader.
            render_pass.set_bind_group(
                1,
                global_bind_group,
                &[depth_of_field_uniform_index.index()],
            );
            // Render the full-screen pass.
            render_pass.draw(0..3, 0..1);

            pass_span.end(&mut render_pass);
        }

        Ok(())
    }
}

impl Default for DepthOfField {
    fn default() -> Self {
        let physical_camera_default = PhysicalCameraParameters::default();
        Self {
            focal_distance: 10.0,
            aperture_f_stops: physical_camera_default.aperture_f_stops,
            sensor_height: physical_camera_default.sensor_height,
            max_circle_of_confusion_diameter: 64.0,
            max_depth: f32::INFINITY,
            mode: DepthOfFieldMode::Bokeh,
        }
    }
}

impl DepthOfField {
    /// Initializes [`DepthOfField`] from a set of
    /// [`PhysicalCameraParameters`].
    ///
    /// By passing the same [`PhysicalCameraParameters`] object to this function
    /// and to [`bevy_camera::Exposure::from_physical_camera`], matching
    /// results for both the exposure and depth of field effects can be
    /// obtained.
    ///
    /// All fields of the returned [`DepthOfField`] other than
    /// `focal_length` and `aperture_f_stops` are set to their default values.
    pub fn from_physical_camera(camera: &PhysicalCameraParameters) -> DepthOfField {
        DepthOfField {
            sensor_height: camera.sensor_height,
            aperture_f_stops: camera.aperture_f_stops,
            ..default()
        }
    }
}

pub fn init_dof_global_bind_group_layout(mut commands: Commands, render_device: Res<RenderDevice>) {
    // Create the bind group layout that will be shared among all instances
    // of the depth of field shader.
    let layout = render_device.create_bind_group_layout(
        Some("depth of field global bind group layout"),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // `dof_params`
                uniform_buffer::<DepthOfFieldUniform>(true),
                // `color_texture_sampler`
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    // Create the color texture sampler.
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("depth of field sampler"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });

    commands.insert_resource(DepthOfFieldGlobalBindGroupLayout {
        color_texture_sampler: sampler,
        layout,
    });
}

/// Creates the bind group layouts for the depth of field effect that are
/// specific to each view.
pub fn prepare_depth_of_field_view_bind_group_layouts(
    mut commands: Commands,
    view_targets: Query<(Entity, &DepthOfField, &Msaa)>,
    render_device: Res<RenderDevice>,
) {
    for (view, depth_of_field, msaa) in view_targets.iter() {
        // Create the bind group layout for the passes that take one input.
        let single_input = render_device.create_bind_group_layout(
            Some("depth of field bind group layout (single input)"),
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    if *msaa != Msaa::Off {
                        texture_depth_2d_multisampled()
                    } else {
                        texture_depth_2d()
                    },
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
            ),
        );

        // If needed, create the bind group layout for the second bokeh pass,
        // which takes two inputs. We only need to do this if bokeh is in use.
        let dual_input = match depth_of_field.mode {
            DepthOfFieldMode::Gaussian => None,
            DepthOfFieldMode::Bokeh => Some(render_device.create_bind_group_layout(
                Some("depth of field bind group layout (dual input)"),
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        uniform_buffer::<ViewUniform>(true),
                        if *msaa != Msaa::Off {
                            texture_depth_2d_multisampled()
                        } else {
                            texture_depth_2d()
                        },
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                    ),
                ),
            )),
        };

        commands
            .entity(view)
            .insert(ViewDepthOfFieldBindGroupLayouts {
                single_input,
                dual_input,
            });
    }
}

/// Configures depth textures so that the depth of field shader can read from
/// them.
///
/// By default, the depth buffers that Bevy creates aren't able to be bound as
/// textures. The depth of field shader, however, needs to read from them. So we
/// need to set the appropriate flag to tell Bevy to make samplable depth
/// buffers.
pub fn configure_depth_of_field_view_targets(
    mut view_targets: Query<&mut Camera3d, With<DepthOfField>>,
) {
    for mut camera_3d in view_targets.iter_mut() {
        let mut depth_texture_usages = TextureUsages::from(camera_3d.depth_texture_usages);
        depth_texture_usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = depth_texture_usages.into();
    }
}

/// Creates depth of field bind group 1, which is shared among all instances of
/// the depth of field shader.
pub fn prepare_depth_of_field_global_bind_group(
    global_bind_group_layout: Res<DepthOfFieldGlobalBindGroupLayout>,
    mut dof_bind_group: ResMut<DepthOfFieldGlobalBindGroup>,
    depth_of_field_uniforms: Res<ComponentUniforms<DepthOfFieldUniform>>,
    render_device: Res<RenderDevice>,
) {
    let Some(depth_of_field_uniforms) = depth_of_field_uniforms.binding() else {
        return;
    };

    **dof_bind_group = Some(render_device.create_bind_group(
        Some("depth of field global bind group"),
        &global_bind_group_layout.layout,
        &BindGroupEntries::sequential((
            depth_of_field_uniforms,                         // `dof_params`
            &global_bind_group_layout.color_texture_sampler, // `color_texture_sampler`
        )),
    ));
}

/// Creates the second render target texture that the first pass of the bokeh
/// effect needs.
pub fn prepare_auxiliary_depth_of_field_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut view_targets: Query<(Entity, &ViewTarget, &DepthOfField)>,
) {
    for (entity, view_target, depth_of_field) in view_targets.iter_mut() {
        // An auxiliary texture is only needed for bokeh.
        if depth_of_field.mode != DepthOfFieldMode::Bokeh {
            continue;
        }

        // The texture matches the main view target texture.
        let texture_descriptor = TextureDescriptor {
            label: Some("depth of field auxiliary texture"),
            size: view_target.main_texture().size(),
            mip_level_count: 1,
            sample_count: view_target.main_texture().sample_count(),
            dimension: TextureDimension::D2,
            format: view_target.main_texture_format(),
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = texture_cache.get(&render_device, texture_descriptor);

        commands
            .entity(entity)
            .insert(AuxiliaryDepthOfFieldTexture(texture));
    }
}

/// Specializes the depth of field pipelines specific to a view.
pub fn prepare_depth_of_field_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DepthOfFieldPipeline>>,
    global_bind_group_layout: Res<DepthOfFieldGlobalBindGroupLayout>,
    view_targets: Query<(
        Entity,
        &ExtractedView,
        &DepthOfField,
        &ViewDepthOfFieldBindGroupLayouts,
        &Msaa,
    )>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    for (entity, view, depth_of_field, view_bind_group_layouts, msaa) in view_targets.iter() {
        let dof_pipeline = DepthOfFieldPipeline {
            view_bind_group_layouts: view_bind_group_layouts.clone(),
            global_bind_group_layout: global_bind_group_layout.layout.clone(),
            fullscreen_shader: fullscreen_shader.clone(),
            fragment_shader: load_embedded_asset!(asset_server.as_ref(), "dof.wgsl"),
        };

        // We'll need these two flags to create the `DepthOfFieldPipelineKey`s.
        let (hdr, multisample) = (view.hdr, *msaa != Msaa::Off);

        // Go ahead and specialize the pipelines.
        match depth_of_field.mode {
            DepthOfFieldMode::Gaussian => {
                commands
                    .entity(entity)
                    .insert(DepthOfFieldPipelines::Gaussian {
                        horizontal: pipelines.specialize(
                            &pipeline_cache,
                            &dof_pipeline,
                            DepthOfFieldPipelineKey {
                                hdr,
                                multisample,
                                pass: DofPass::GaussianHorizontal,
                            },
                        ),
                        vertical: pipelines.specialize(
                            &pipeline_cache,
                            &dof_pipeline,
                            DepthOfFieldPipelineKey {
                                hdr,
                                multisample,
                                pass: DofPass::GaussianVertical,
                            },
                        ),
                    });
            }

            DepthOfFieldMode::Bokeh => {
                commands
                    .entity(entity)
                    .insert(DepthOfFieldPipelines::Bokeh {
                        pass_0: pipelines.specialize(
                            &pipeline_cache,
                            &dof_pipeline,
                            DepthOfFieldPipelineKey {
                                hdr,
                                multisample,
                                pass: DofPass::BokehPass0,
                            },
                        ),
                        pass_1: pipelines.specialize(
                            &pipeline_cache,
                            &dof_pipeline,
                            DepthOfFieldPipelineKey {
                                hdr,
                                multisample,
                                pass: DofPass::BokehPass1,
                            },
                        ),
                    });
            }
        }
    }
}

impl SpecializedRenderPipeline for DepthOfFieldPipeline {
    type Key = DepthOfFieldPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Build up our pipeline layout.
        let (mut layout, mut shader_defs) = (vec![], vec![]);
        let mut targets = vec![Some(ColorTargetState {
            format: if key.hdr {
                ViewTarget::TEXTURE_FORMAT_HDR
            } else {
                TextureFormat::bevy_default()
            },
            blend: None,
            write_mask: ColorWrites::ALL,
        })];

        // Select bind group 0, the view-specific bind group.
        match key.pass {
            DofPass::GaussianHorizontal | DofPass::GaussianVertical => {
                // Gaussian blurs take only a single input and output.
                layout.push(self.view_bind_group_layouts.single_input.clone());
            }
            DofPass::BokehPass0 => {
                // The first bokeh pass takes one input and produces two outputs.
                layout.push(self.view_bind_group_layouts.single_input.clone());
                targets.push(targets[0].clone());
            }
            DofPass::BokehPass1 => {
                // The second bokeh pass takes the two outputs from the first
                // bokeh pass and produces a single output.
                let dual_input_bind_group_layout = self
                    .view_bind_group_layouts
                    .dual_input
                    .as_ref()
                    .expect("Dual-input depth of field bind group should have been created by now")
                    .clone();
                layout.push(dual_input_bind_group_layout);
                shader_defs.push("DUAL_INPUT".into());
            }
        }

        // Add bind group 1, the global bind group.
        layout.push(self.global_bind_group_layout.clone());

        if key.multisample {
            shader_defs.push("MULTISAMPLED".into());
        }

        RenderPipelineDescriptor {
            label: Some("depth of field pipeline".into()),
            layout,
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                entry_point: Some(match key.pass {
                    DofPass::GaussianHorizontal => "gaussian_horizontal".into(),
                    DofPass::GaussianVertical => "gaussian_vertical".into(),
                    DofPass::BokehPass0 => "bokeh_pass_0".into(),
                    DofPass::BokehPass1 => "bokeh_pass_1".into(),
                }),
                targets,
            }),
            ..default()
        }
    }
}

/// Extracts all [`DepthOfField`] components into the render world.
fn extract_depth_of_field_settings(
    mut commands: Commands,
    mut query: Extract<Query<(RenderEntity, &DepthOfField, &Projection)>>,
) {
    if !DEPTH_TEXTURE_SAMPLING_SUPPORTED {
        once!(info!(
            "Disabling depth of field on this platform because depth textures aren't supported correctly"
        ));
        return;
    }

    for (entity, depth_of_field, projection) in query.iter_mut() {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Depth of field entity wasn't synced.");

        // Depth of field is nonsensical without a perspective projection.
        let Projection::Perspective(ref perspective_projection) = *projection else {
            // TODO: needs better strategy for cleaning up
            entity_commands.remove::<(
                DepthOfField,
                DepthOfFieldUniform,
                // components added in prepare systems (because `DepthOfFieldNode` does not query extracted components)
                DepthOfFieldPipelines,
                AuxiliaryDepthOfFieldTexture,
                ViewDepthOfFieldBindGroupLayouts,
            )>();
            continue;
        };

        let focal_length =
            calculate_focal_length(depth_of_field.sensor_height, perspective_projection.fov);

        // Convert `DepthOfField` to `DepthOfFieldUniform`.
        entity_commands.insert((
            *depth_of_field,
            DepthOfFieldUniform {
                focal_distance: depth_of_field.focal_distance,
                focal_length,
                coc_scale_factor: focal_length * focal_length
                    / (depth_of_field.sensor_height * depth_of_field.aperture_f_stops),
                max_circle_of_confusion_diameter: depth_of_field.max_circle_of_confusion_diameter,
                max_depth: depth_of_field.max_depth,
                pad_a: 0,
                pad_b: 0,
                pad_c: 0,
            },
        ));
    }
}

/// Given the sensor height and the FOV, returns the focal length.
///
/// See <https://photo.stackexchange.com/a/97218>.
pub fn calculate_focal_length(sensor_height: f32, fov: f32) -> f32 {
    0.5 * sensor_height / ops::tan(0.5 * fov)
}

impl DepthOfFieldPipelines {
    /// Populates the information that the `DepthOfFieldNode` needs for the two
    /// depth of field render passes.
    fn pipeline_render_info(&self) -> [DepthOfFieldPipelineRenderInfo; 2] {
        match *self {
            DepthOfFieldPipelines::Gaussian {
                horizontal: horizontal_pipeline,
                vertical: vertical_pipeline,
            } => [
                DepthOfFieldPipelineRenderInfo {
                    pass_label: "depth of field pass (horizontal Gaussian)",
                    view_bind_group_label: "depth of field view bind group (horizontal Gaussian)",
                    pipeline: horizontal_pipeline,
                    is_dual_input: false,
                    is_dual_output: false,
                },
                DepthOfFieldPipelineRenderInfo {
                    pass_label: "depth of field pass (vertical Gaussian)",
                    view_bind_group_label: "depth of field view bind group (vertical Gaussian)",
                    pipeline: vertical_pipeline,
                    is_dual_input: false,
                    is_dual_output: false,
                },
            ],

            DepthOfFieldPipelines::Bokeh {
                pass_0: pass_0_pipeline,
                pass_1: pass_1_pipeline,
            } => [
                DepthOfFieldPipelineRenderInfo {
                    pass_label: "depth of field pass (bokeh pass 0)",
                    view_bind_group_label: "depth of field view bind group (bokeh pass 0)",
                    pipeline: pass_0_pipeline,
                    is_dual_input: false,
                    is_dual_output: true,
                },
                DepthOfFieldPipelineRenderInfo {
                    pass_label: "depth of field pass (bokeh pass 1)",
                    view_bind_group_label: "depth of field view bind group (bokeh pass 1)",
                    pipeline: pass_1_pipeline,
                    is_dual_input: true,
                    is_dual_output: false,
                },
            ],
        }
    }
}
