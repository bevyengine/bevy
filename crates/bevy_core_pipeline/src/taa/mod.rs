use crate::{
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::Camera3d,
    prepass::{DepthPrepass, VelocityPrepass, ViewPrepassTextures},
};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core::FrameCount;
use bevy_ecs::{
    prelude::{Bundle, Component, Entity},
    query::{QueryState, With},
    schedule::IntoSystemConfig,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::vec2;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    camera::{ExtractedCamera, TemporalJitter},
    prelude::{Camera, Projection},
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
        ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{BevyDefault, CachedTexture, TextureCache},
    view::{prepare_view_uniforms, ExtractedView, Msaa, ViewTarget},
    ExtractSchedule, MainWorld, RenderApp, RenderSet,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

mod draw_3d_graph {
    pub mod node {
        /// Label for the TAA render node.
        pub const TAA: &str = "taa";
    }
}

const TAA_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656865235226276);

/// Plugin for temporal antialiasing. Disables multisample antialiasing (MSAA).
pub struct TemporalAntialiasPlugin;

impl Plugin for TemporalAntialiasPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TAA_SHADER_HANDLE, "taa.wgsl", Shader::from_wgsl);

        app.insert_resource(Msaa::Off)
            .register_type::<TemporalAntialiasSettings>();

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        render_app
            .init_resource::<TAAPipeline>()
            .init_resource::<SpecializedRenderPipelines<TAAPipeline>>()
            .add_system_to_schedule(ExtractSchedule, extract_taa_settings)
            .add_system(
                prepare_taa_jitter
                    .before(prepare_view_uniforms)
                    .in_set(RenderSet::Prepare),
            )
            .add_system(prepare_taa_history_textures.in_set(RenderSet::Prepare))
            .add_system(prepare_taa_pipelines.in_set(RenderSet::Prepare));

        let taa_node = TAANode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(crate::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::TAA, taa_node);
        draw_3d_graph.add_slot_edge(
            draw_3d_graph.input_node().id,
            crate::core_3d::graph::input::VIEW_ENTITY,
            draw_3d_graph::node::TAA,
            TAANode::IN_VIEW,
        );
        // MAIN_PASS -> TAA -> BLOOM -> TONEMAPPING
        draw_3d_graph.add_node_edge(
            crate::core_3d::graph::node::MAIN_PASS,
            draw_3d_graph::node::TAA,
        );
        draw_3d_graph.add_node_edge(draw_3d_graph::node::TAA, crate::core_3d::graph::node::BLOOM);
        draw_3d_graph.add_node_edge(
            draw_3d_graph::node::TAA,
            crate::core_3d::graph::node::TONEMAPPING,
        );
    }
}

/// Bundle to apply temporal antialiasing.
#[derive(Bundle, Default)]
pub struct TemporalAntialiasBundle {
    pub settings: TemporalAntialiasSettings,
    pub jitter: TemporalJitter,
    pub depth_prepass: DepthPrepass,
    pub velocity_prepass: VelocityPrepass,
}

/// Component to apply temporal antialiasing to a 3d perspective camera.
///
/// Temporal antialiasing (TAA) is a form of image smoothing/filtering, like
/// multisample antialiasing (MSAA), or fast approximate antialiasing (FXAA).
/// TAA works by blending (averaging) each frame with the past few frames.
///
/// # Tradeoffs
///
/// Pros:
/// * Cost scales with screen/view resolution, unlike MSAA which scales with number of triangles
/// * Filters more types of aliasing than MSAA, such as textures and singular bright pixels
/// * Greatly increases the quality of stochastic rendering techniques, such as SSAO and SSR
///
/// Cons:
/// * Chance of "ghosting" - ghostly trails left behind moving objects
/// * Thin geometry, lighting detail, or texture lines may flicker or disappear
/// * Slightly blurs the image, leading to a softer look (using an additional sharpening pass can reduce this)
///
/// Because TAA blends past frames with the current frame, when the frames differ too much
/// (such as with fast moving objects or camera cuts), ghosting artifacts may occur.
///
/// Artifacts tend to be reduced at higher framerates and rendering resolution.
///
/// # Usage Notes
///
/// Requires that you add [`TemporalAntialiasPlugin`] to your app,
/// and add the [`DepthPrepass`], [`VelocityPrepass`], and [`TemporalJitter`]
/// components to your camera.
///
/// Cannot be used with [`bevy_render::camera::OrthographicProjection`].
///
/// Currently not compatible with skinned meshes. There will probably be ghosting artifacts.
///
/// It is very important that correct velocity vectors are written for everything on screen.
/// Failure to do so will lead to ghosting artifacts. For instance, if particle effects
/// are added using a third party library, the library must either:
/// 1. Write particle velocity to the velocity prepass texture
/// 2. Render particles after TAA
#[derive(Component, Reflect, Clone)]
pub struct TemporalAntialiasSettings {
    /// Set to true to delete the saved temporal history (past frames).
    ///
    /// Useful for preventing ghosting when the history is no longer
    /// representive of the current frame, such as in sudden camera cuts.
    ///
    /// After setting this to true, it will automatically be toggled
    /// back to false after one frame.
    pub reset: bool,
}

impl Default for TemporalAntialiasSettings {
    fn default() -> Self {
        Self { reset: true }
    }
}

struct TAANode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static TAAHistoryTextures,
        &'static ViewPrepassTextures,
        &'static TAAPipelineId,
    )>,
}

impl TAANode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for TAANode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        #[cfg(feature = "trace")]
        let _taa_span = info_span!("taa").entered();

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (
            Ok((camera, view_target, taa_history_textures, prepass_textures, taa_pipeline_id)),
            Some(pipelines),
            Some(pipeline_cache),
        ) = (
            self.view_query.get_manual(world, view_entity),
            world.get_resource::<TAAPipeline>(),
            world.get_resource::<PipelineCache>(),
        ) else {
            return Ok(());
        };
        let (
            Some(taa_pipeline),
            Some(prepass_velocity_texture),
            Some(prepass_depth_texture),
        ) = (
            pipeline_cache.get_render_pipeline(taa_pipeline_id.0),
            &prepass_textures.velocity,
            &prepass_textures.depth,
        ) else {
            return Ok(());
        };
        let view_target = view_target.post_process_write();

        let taa_bind_group =
            render_context
                .render_device()
                .create_bind_group(&BindGroupDescriptor {
                    label: Some("taa_bind_group"),
                    layout: &pipelines.taa_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(view_target.source),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(
                                &taa_history_textures.read.default_view,
                            ),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::TextureView(
                                &prepass_velocity_texture.default_view,
                            ),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(
                                &prepass_depth_texture.default_view,
                            ),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::Sampler(&pipelines.nearest_sampler),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: BindingResource::Sampler(&pipelines.linear_sampler),
                        },
                    ],
                });

        {
            let mut taa_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("taa_pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: view_target.destination,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                    Some(RenderPassColorAttachment {
                        view: &taa_history_textures.write.default_view,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                ],
                depth_stencil_attachment: None,
            });
            taa_pass.set_render_pipeline(taa_pipeline);
            taa_pass.set_bind_group(0, &taa_bind_group, &[]);
            if let Some(viewport) = camera.viewport.as_ref() {
                taa_pass.set_camera_viewport(viewport);
            }
            taa_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

#[derive(Resource)]
struct TAAPipeline {
    taa_bind_group_layout: BindGroupLayout,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
}

impl FromWorld for TAAPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("taa_nearest_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..SamplerDescriptor::default()
        });
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("taa_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let taa_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("taa_bind_group_layout"),
                entries: &[
                    // View target (read)
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // TAA History (read)
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Velocity
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Depth
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Nearest sampler
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // Linear sampler
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        TAAPipeline {
            taa_bind_group_layout,
            nearest_sampler,
            linear_sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct TAAPipelineKey {
    hdr: bool,
    reset: bool,
}

impl SpecializedRenderPipeline for TAAPipeline {
    type Key = TAAPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];

        let format = if key.hdr {
            shader_defs.push("TONEMAP".into());
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if key.reset {
            shader_defs.push("RESET".into());
        }

        RenderPipelineDescriptor {
            label: Some("taa_pipeline".into()),
            layout: Some(vec![self.taa_bind_group_layout.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TAA_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "taa".into(),
                targets: vec![
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

fn extract_taa_settings(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d = main_world
        .query_filtered::<(Entity, &Camera, &Projection, &mut TemporalAntialiasSettings), (
            With<Camera3d>,
            With<TemporalJitter>,
            With<DepthPrepass>,
            With<VelocityPrepass>,
        )>();

    for (entity, camera, camera_projection, mut taa_settings) in
        cameras_3d.iter_mut(&mut main_world)
    {
        let has_perspective_projection = matches!(camera_projection, Projection::Perspective(_));
        if camera.is_active && has_perspective_projection {
            commands.get_or_spawn(entity).insert(taa_settings.clone());
            taa_settings.reset = false;
        }
    }
}

fn prepare_taa_jitter(
    frame_count: Res<FrameCount>,
    mut query: Query<&mut TemporalJitter, With<TemporalAntialiasSettings>>,
) {
    // Halton sequence (2, 3)
    let halton_sequence = [
        vec2(0.5, 0.33333334),
        vec2(0.25, 0.6666667),
        vec2(0.75, 0.111111104),
        vec2(0.125, 0.44444445),
        vec2(0.625, 0.7777778),
        vec2(0.375, 0.22222221),
        vec2(0.875, 0.5555556),
        vec2(0.0625, 0.8888889),
        vec2(0.5625, 0.037037045),
        vec2(0.3125, 0.3703704),
        vec2(0.8125, 0.7037037),
        vec2(0.1875, 0.14814815),
    ];

    let offset = halton_sequence[frame_count.0 as usize % halton_sequence.len()];

    for mut jitter in &mut query {
        jitter.offset = offset;
    }
}

#[derive(Component)]
struct TAAHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
}

fn prepare_taa_history_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView), With<TemporalAntialiasSettings>>,
) {
    for (entity, camera, view) in &views {
        if let Some(physical_viewport_size) = camera.physical_viewport_size {
            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_viewport_size.x,
                    height: physical_viewport_size.y,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            texture_descriptor.label = Some("taa_history_1_texture");
            let history_1_texture = texture_cache.get(&render_device, texture_descriptor.clone());

            texture_descriptor.label = Some("taa_history_2_texture");
            let history_2_texture = texture_cache.get(&render_device, texture_descriptor);

            let textures = if frame_count.0 % 2 == 0 {
                TAAHistoryTextures {
                    write: history_1_texture,
                    read: history_2_texture,
                }
            } else {
                TAAHistoryTextures {
                    write: history_2_texture,
                    read: history_1_texture,
                }
            };

            commands.entity(entity).insert(textures);
        }
    }
}

#[derive(Component)]
struct TAAPipelineId(CachedRenderPipelineId);

fn prepare_taa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TAAPipeline>>,
    pipeline: Res<TAAPipeline>,
    views: Query<(Entity, &ExtractedView, &TemporalAntialiasSettings)>,
) {
    for (entity, view, taa_settings) in &views {
        let mut pipeline_key = TAAPipelineKey {
            hdr: view.hdr,
            reset: taa_settings.reset,
        };
        let pipeline_id = pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key.clone());

        // Prepare non-reset pipeline anyways - it will be necessary next frame
        if pipeline_key.reset {
            pipeline_key.reset = false;
            pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key);
        }

        commands.entity(entity).insert(TAAPipelineId(pipeline_id));
    }
}
