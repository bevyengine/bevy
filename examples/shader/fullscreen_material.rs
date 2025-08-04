//! Demonstrates how to write a custom fullscreen shader

use bevy::{prelude::*, shader::ShaderRef};
use bevy_render::{extract_component::ExtractComponent, render_resource::ShaderType};
use plugin::{FullscreenMaterial, FullscreenMaterialPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FullscreenMaterialPlugin::<MyPostProcessing>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::default(), Vec3::Y),
        MyPostProcessing { data: 0.005 },
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn(DirectionalLight {
        illuminance: 1_000.,
        ..default()
    });
}

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
struct MyPostProcessing {
    data: f32,
}

impl FullscreenMaterial for MyPostProcessing {
    fn fragment_shader() -> ShaderRef {
        "shaders/my_post_processing.wgsl".into()
    }
}

mod plugin {
    use std::marker::PhantomData;

    use bevy::{
        core_pipeline::{
            core_3d::graph::{Core3d, Node3d},
            FullscreenShader,
        },
        prelude::*,
        shader::ShaderRef,
    };
    use bevy_ecs::query::QueryItem;
    use bevy_render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            encase::internal::WriteInto,
            AsBindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, Operations,
            PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp, RenderStartup,
    };
    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    struct PostProcessLabel;

    #[derive(Default)]
    pub struct FullscreenMaterialPlugin<T: FullscreenMaterial> {
        _marker: PhantomData<T>,
    }
    impl<T: FullscreenMaterial> Plugin for FullscreenMaterialPlugin<T> {
        fn build(&self, app: &mut App) {
            app.add_plugins((
                ExtractComponentPlugin::<T>::default(),
                UniformComponentPlugin::<T>::default(),
            ));

            let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
                return;
            };
            render_app.add_systems(RenderStartup, init_pipeline::<T>);
            render_app
                .add_render_graph_node::<ViewNodeRunner<FullscreenMaterialNode<T>>>(
                    Core3d,
                    PostProcessLabel,
                )
                .add_render_graph_edges(
                    Core3d,
                    (
                        Node3d::Tonemapping,
                        // TODO make this configurable
                        PostProcessLabel,
                        Node3d::EndMainPassPostProcessing,
                    ),
                );
        }
    }

    pub trait FullscreenMaterial:
        Component + ExtractComponent + Clone + Copy + ShaderType + WriteInto + Default
    {
        fn fragment_shader() -> ShaderRef;
    }

    #[derive(Resource)]
    struct FullscreenMaterialPipeline {
        layout: BindGroupLayout,
        sampler: Sampler,
        pipeline_id: CachedRenderPipelineId,
    }

    fn init_pipeline<T: FullscreenMaterial>(
        mut commands: Commands,
        render_device: Res<RenderDevice>,
        asset_server: Res<AssetServer>,
        fullscreen_shader: Res<FullscreenShader>,
        pipeline_cache: Res<PipelineCache>,
    ) {
        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "post_process_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::Filtering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<T>(true),
                ),
            ),
        );
        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let shader = match T::fragment_shader() {
            ShaderRef::Default => {
                unimplemented!("No default fallback for FullscreenMaterial shader")
            }
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => asset_server.load(path),
        };
        // This will setup a fullscreen triangle for the vertex state.
        let vertex_state = fullscreen_shader.to_vertex_state();
        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("post_process_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: vertex_state,
            fragment: Some(FragmentState {
                shader,
                // TODO handle HDR
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
        commands.insert_resource(FullscreenMaterialPipeline {
            layout,
            sampler,
            pipeline_id,
        });
    }

    #[derive(Default)]
    struct FullscreenMaterialNode<T: FullscreenMaterial> {
        _marker: PhantomData<T>,
    }

    impl<T: FullscreenMaterial> ViewNode for FullscreenMaterialNode<T> {
        // The node needs a query to gather data from the ECS in order to do its rendering,
        // but it's not a normal system so we need to define it manually.
        //
        // This query will only run on the view entity
        type ViewQuery = (
            &'static ViewTarget,
            // This makes sure the node only runs on cameras with the PostProcessSettings component
            &'static T,
            // As there could be multiple post processing components sent to the GPU (one per camera),
            // we need to get the index of the one that is associated with the current view.
            &'static DynamicUniformIndex<T>,
        );

        fn run<'w>(
            &self,
            _graph: &mut RenderGraphContext,
            render_context: &mut RenderContext,
            (view_target, _post_process_settings, settings_index): QueryItem<Self::ViewQuery>,
            world: &World,
        ) -> Result<(), NodeRunError> {
            let post_process_pipeline = world.resource::<FullscreenMaterialPipeline>();

            let pipeline_cache = world.resource::<PipelineCache>();

            let Some(pipeline) =
                pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id)
            else {
                return Ok(());
            };

            let settings_uniforms = world.resource::<ComponentUniforms<T>>();
            let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
                return Ok(());
            };

            let post_process = view_target.post_process_write();

            let bind_group = render_context.render_device().create_bind_group(
                "post_process_bind_group",
                &post_process_pipeline.layout,
                // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
                &BindGroupEntries::sequential((
                    // Make sure to use the source view
                    post_process.source,
                    // Use the sampler created for the pipeline
                    &post_process_pipeline.sampler,
                    // Set the settings binding
                    settings_binding.clone(),
                )),
            );

            // Begin the render pass
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("post_process_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    // We need to specify the post process destination view here
                    // to make sure we write to the appropriate texture.
                    view: post_process.destination,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // This is mostly just wgpu boilerplate for drawing a fullscreen triangle,
            // using the pipeline/bind_group created above
            render_pass.set_render_pipeline(pipeline);
            // By passing in the index of the post process settings on this view, we ensure
            // that in the event that multiple settings were sent to the GPU (as would be the
            // case with multiple cameras), we use the correct one.
            render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
            render_pass.draw(0..3, 0..1);

            Ok(())
        }
    }
}
