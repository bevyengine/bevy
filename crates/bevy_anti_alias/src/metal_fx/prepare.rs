use super::{MetalFx, MetalFxFeature};
use bevy_camera::{Camera3d, CameraMainTextureUsages, MainPassResolutionOverride};
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_math::Vec4Swizzles;
use bevy_render::{
    camera::{MipBias, TemporalJitter},
    render_resource::TextureUsages,
    renderer::RenderDevice,
    view::ExtractedView,
};
use metalfx_wgpu::{MetalFxQualityMode, MetalFxTextureUsages};
use std::sync::Mutex;

#[derive(Component)]
pub struct MetalFxRenderContext<F: MetalFxFeature> {
    pub context: Mutex<F::Context>,
    pub quality_mode: MetalFxQualityMode,
}

pub fn prepare_metal_fx<F: MetalFxFeature>(
    mut query: Query<
        (
            Entity,
            &ExtractedView,
            &MetalFx<F>,
            &mut Camera3d,
            &mut CameraMainTextureUsages,
            &mut TemporalJitter,
            &mut MipBias,
            Option<&mut MetalFxRenderContext<F>>,
        ),
        (
            With<Camera3d>,
            With<TemporalJitter>,
            With<DepthPrepass>,
            With<MotionVectorPrepass>,
        ),
    >,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    mut commands: Commands,
) {
    for (
        entity,
        view,
        metal_fx,
        mut camera_3d,
        mut camera_main_texture_usages,
        mut temporal_jitter,
        mut mip_bias,
        mut metal_fx_context,
    ) in &mut query
    {
        let upscaled_resolution = view.viewport.zw();

        match metal_fx_context.as_deref_mut() {
            Some(ctx)
                if upscaled_resolution == F::upscaled_resolution(&ctx.context.lock().unwrap())
                    && metal_fx.quality_mode == ctx.quality_mode =>
            {
                let ctx = ctx.context.lock().unwrap();
                let render_resolution = F::render_resolution(&ctx);
                temporal_jitter.offset =
                    F::suggested_jitter(&ctx, frame_count.0, render_resolution);
                mip_bias.0 = F::suggested_mip_bias(&ctx, render_resolution);
            }
            _ => {
                let context =
                    F::new_context(upscaled_resolution, metal_fx.quality_mode, &render_device)
                        .expect("Failed to create MetalFxRenderContext");

                let usages = F::texture_usages(&context);
                apply_texture_usages(&mut camera_main_texture_usages, &mut camera_3d, usages);

                let render_resolution = F::render_resolution(&context);
                temporal_jitter.offset =
                    F::suggested_jitter(&context, frame_count.0, render_resolution);
                mip_bias.0 = F::suggested_mip_bias(&context, render_resolution);

                commands.entity(entity).insert((
                    MetalFxRenderContext::<F> {
                        context: Mutex::new(context),
                        quality_mode: metal_fx.quality_mode,
                    },
                    MainPassResolutionOverride(render_resolution),
                ));
            }
        }
    }
}

fn apply_texture_usages(
    camera_main_texture_usages: &mut CameraMainTextureUsages,
    camera_3d: &mut Camera3d,
    usages: MetalFxTextureUsages,
) {
    camera_main_texture_usages.0 |= usages.color
        | usages.output
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING;

    let mut depth_texture_usages = TextureUsages::from(camera_3d.depth_texture_usages);
    depth_texture_usages |= usages.depth | TextureUsages::TEXTURE_BINDING;
    camera_3d.depth_texture_usages = depth_texture_usages.into();
}
