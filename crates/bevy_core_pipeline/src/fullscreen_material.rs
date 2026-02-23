//! This is mostly a pluginified version of the `custom_post_processing` example
//!
//! The plugin will create a new system that runs a fullscreen triangle.
//!
//! Users need to use the [`FullscreenMaterial`] trait to define the parameters like ordering.

use core::any::type_name;
use core::marker::PhantomData;

use crate::{schedule::Core3d, Core3dSystems, FullscreenShader};
use bevy_app::{App, Plugin};
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, SystemSet},
    system::{Commands, Query, Res},
};
use bevy_image::BevyDefault;
use bevy_render::{
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        encase::internal::WriteInto,
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, Operations,
        PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
        Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureFormat,
        TextureSampleType, TextureView, TextureViewId,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    view::ViewTarget,
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::ShaderRef;
use bevy_utils::default;

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

        render_app
            .add_systems(RenderStartup, init_pipeline::<T>)
            .add_systems(
                Render,
                prepare_bind_groups::<T>.in_set(RenderSystems::PrepareBindGroups),
            );

        let mut system = fullscreen_material_system::<T>.in_set(T::run_in());
        if let Some(run_after) = T::run_after() {
            system = system.after(run_after);
        }
        if let Some(run_before) = T::run_before() {
            system = system.before(run_before);
        }
        render_app.add_systems(T::schedule(), system);
    }
}

/// A trait to define a material that will render to the entire screen using a fullscreen triangle.
pub trait FullscreenMaterial:
    Component + ExtractComponent + Clone + Copy + ShaderType + WriteInto + Default
{
    /// The shader that will run on the entire screen using a fullscreen triangle.
    fn fragment_shader() -> ShaderRef;

    /// The schedule this effect runs in.
    ///
    /// Defaults to [`Core3d`] for 3D post-processing effects.
    fn schedule() -> impl ScheduleLabel + Clone {
        Core3d
    }

    /// The system set this effect belongs to.
    ///
    /// Defaults to [`Core3dSystems::PostProcess`].
    fn run_in() -> impl SystemSet {
        Core3dSystems::PostProcess
    }

    /// The system set this effect runs after.
    ///
    /// Defaults to `None`.
    fn run_after() -> Option<Core3dSystems> {
        None
    }

    /// The system set this effect runs before.
    ///
    /// Defaults to `None`.
    fn run_before() -> Option<Core3dSystems> {
        None
    }
}

#[derive(Resource)]
struct FullscreenMaterialPipeline<T: FullscreenMaterial> {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
    pipeline_id_hdr: CachedRenderPipelineId,
    _marker: PhantomData<T>,
}

fn init_pipeline<T: FullscreenMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "fullscreen_material_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<T>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = match T::fragment_shader() {
        ShaderRef::Default => {
            unimplemented!(
                "FullscreenMaterial::fragment_shader() must not return ShaderRef::Default"
            )
        }
        ShaderRef::Handle(handle) => handle,
        ShaderRef::Path(path) => asset_server.load(path),
    };

    let vertex_state = fullscreen_shader.to_vertex_state();
    let mut desc = RenderPipelineDescriptor {
        label: Some(format!("fullscreen_material_pipeline<{}>", type_name::<T>()).into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    };
    let pipeline_id = pipeline_cache.queue_render_pipeline(desc.clone());
    desc.fragment.as_mut().unwrap().targets[0]
        .as_mut()
        .unwrap()
        .format = ViewTarget::TEXTURE_FORMAT_HDR;
    let pipeline_id_hdr = pipeline_cache.queue_render_pipeline(desc);

    commands.insert_resource(FullscreenMaterialPipeline::<T> {
        layout,
        sampler,
        pipeline_id,
        pipeline_id_hdr,
        _marker: PhantomData,
    });
}

/// Holds the bind groups for both main textures
///
/// We can't know ahead of time which one is the source or destination so we create a bind group
/// for both
#[derive(Component)]
struct FullscreenMaterialBindGroup<T: FullscreenMaterial> {
    a: (TextureViewId, BindGroup),
    b: (TextureViewId, BindGroup),
    // This is in case someone wants multiple `FullscreenMaterial` per camera
    _marker: PhantomData<T>,
}

/// Prepare the bind groups for both main textures for all views that have a [`FullscreenMaterial`]
fn prepare_bind_groups<T: FullscreenMaterial>(
    mut commands: Commands,
    mut view: Query<(
        Entity,
        &ViewTarget,
        Option<&mut FullscreenMaterialBindGroup<T>>,
    )>,
    fullscreen_pipeline: Option<Res<FullscreenMaterialPipeline<T>>>,
    pipeline_cache: Res<PipelineCache>,
    data_uniforms: Res<ComponentUniforms<T>>,
    render_device: Res<RenderDevice>,
) {
    let Some(fullscreen_pipeline) = fullscreen_pipeline else {
        return;
    };
    let Some(settings_binding) = data_uniforms.uniforms().binding() else {
        return;
    };

    for (entity, view_target, mut maybe_bind_groups) in &mut view {
        let main_texture_view = view_target.main_texture_view();
        let main_texture_other_view = view_target.main_texture_other_view();

        let create_bind_group = |texture: &TextureView| {
            (
                texture.id(),
                render_device.create_bind_group(
                    "fullscreen_material_bind_group",
                    &pipeline_cache.get_bind_group_layout(&fullscreen_pipeline.layout),
                    &BindGroupEntries::sequential((
                        texture,
                        &fullscreen_pipeline.sampler,
                        settings_binding.clone(),
                    )),
                ),
            )
        };

        if let Some(bind_groups) = &mut maybe_bind_groups {
            if bind_groups.a.0 != main_texture_view.id() {
                bind_groups.a = create_bind_group(main_texture_view);
            }
            if bind_groups.b.0 != main_texture_other_view.id() {
                bind_groups.b = create_bind_group(main_texture_other_view);
            }
        } else {
            commands.entity(entity).insert(FullscreenMaterialBindGroup {
                a: create_bind_group(main_texture_view),
                b: create_bind_group(main_texture_other_view),
                _marker: PhantomData::<T>,
            });
        }
    }
}

fn fullscreen_material_system<T: FullscreenMaterial>(
    view: ViewQuery<(
        &ViewTarget,
        &DynamicUniformIndex<T>,
        &FullscreenMaterialBindGroup<T>,
    )>,
    fullscreen_pipeline: Option<Res<FullscreenMaterialPipeline<T>>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let Some(fullscreen_pipeline) = fullscreen_pipeline else {
        return;
    };

    let (view_target, settings_index, bind_groups) = view.into_inner();

    let pipeline_id = if view_target.is_hdr() {
        fullscreen_pipeline.pipeline_id_hdr
    } else {
        fullscreen_pipeline.pipeline_id
    };

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
        return;
    };

    let post_process = view_target.post_process_write();
    let source = post_process.source;
    let destination = post_process.destination;

    let (_, bind_group) = if bind_groups.a.0 == source.id() {
        &bind_groups.a
    } else {
        &bind_groups.b
    };

    let pass_descriptor = RenderPassDescriptor {
        label: Some("fullscreen_material_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);
    }
}
