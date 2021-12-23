pub mod visibility;
pub mod window;

pub use visibility::*;
use wgpu::{
    Color, Extent3d, Operations, RenderPassColorAttachment, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};
pub use window::*;

use crate::{
    camera::ExtractedCamera,
    prelude::Image,
    render_asset::RenderAssets,
    render_resource::{std140::AsStd140, DynamicUniformVec, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, TextureCache},
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, UVec2, Vec3};
use bevy_transform::components::GlobalTransform;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Msaa>().add_plugin(VisibilityPlugin);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ViewUniforms>()
                .add_system_to_stage(RenderStage::Extract, extract_msaa)
                .add_system_to_stage(RenderStage::Prepare, prepare_view_uniforms)
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_targets.after(WindowSystem::Prepare),
                );
        }
    }
}

#[derive(Clone)]
/// Configuration resource for [Multi-Sample Anti-Aliasing](https://en.wikipedia.org/wiki/Multisample_anti-aliasing).
///
/// # Example
/// ```
/// # use bevy_app::prelude::App;
/// # use bevy_render::prelude::Msaa;
/// App::new()
///     .insert_resource(Msaa { samples: 4 })
///     .run();
/// ```
pub struct Msaa {
    /// The number of samples to run for Multi-Sample Anti-Aliasing. Higher numbers result in
    /// smoother edges.
    /// Defaults to 4.
    ///
    /// Note that WGPU currently only supports 1 or 4 samples.
    /// Ultimately we plan on supporting whatever is natively supported on a given device.
    /// Check out this issue for more info: <https://github.com/gfx-rs/wgpu/issues/1832>
    pub samples: u32,
}

impl Default for Msaa {
    fn default() -> Self {
        Self { samples: 4 }
    }
}

pub fn extract_msaa(mut commands: Commands, msaa: Res<Msaa>) {
    // NOTE: windows.is_changed() handles cases where a window was resized
    commands.insert_resource(msaa.clone());
}

#[derive(Component)]
pub struct ExtractedView {
    pub projection: Mat4,
    pub transform: GlobalTransform,
    pub width: u32,
    pub height: u32,
    pub near: f32,
    pub far: f32,
}

#[derive(Clone, AsStd140)]
pub struct ViewUniform {
    view_proj: Mat4,
    view: Mat4,
    inverse_view: Mat4,
    projection: Mat4,
    world_position: Vec3,
    near: f32,
    far: f32,
    width: f32,
    height: f32,
}

#[derive(Default)]
pub struct ViewUniforms {
    pub uniforms: DynamicUniformVec<ViewUniform>,
}

#[derive(Component)]
pub struct ViewUniformOffset {
    pub offset: u32,
}

#[derive(Component)]
pub struct ViewTarget {
    pub hdr_texture: TextureView,
    pub sampled_hdr_texture: Option<TextureView>,

    pub ldr_texture: TextureView,
    pub out_texture: TextureView,
}

impl ViewTarget {
    pub const TEXTURE_FORMAT_HDR: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(
        render_device: &RenderDevice,
        texture_cache: &mut TextureCache,
        msaa: &Msaa,
        size: UVec2,
        out_texture: TextureView,
    ) -> ViewTarget {
        let size = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };

        let hdr_texture = texture_cache.get(
            render_device,
            TextureDescriptor {
                label: Some("hdr_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );

        let sampled_hdr_texture = (msaa.samples > 1).then(|| {
            texture_cache
                .get(
                    render_device,
                    TextureDescriptor {
                        label: Some("hdr_texture_sampled"),
                        size,
                        mip_level_count: 1,
                        sample_count: msaa.samples,
                        dimension: TextureDimension::D2,
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
                        usage: TextureUsages::RENDER_ATTACHMENT,
                    },
                )
                .default_view
        });

        let ldr_texture = texture_cache.get(
            render_device,
            TextureDescriptor {
                label: Some("ldr_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::bevy_default(),
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );

        ViewTarget {
            hdr_texture: hdr_texture.default_view,
            sampled_hdr_texture,
            ldr_texture: ldr_texture.default_view,
            out_texture,
        }
    }

    pub fn get_color_attachment(&self, ops: Operations<Color>) -> RenderPassColorAttachment {
        match &self.sampled_hdr_texture {
            Some(sampled_target) => RenderPassColorAttachment {
                view: sampled_target,
                resolve_target: Some(&self.hdr_texture),
                ops,
            },
            None => RenderPassColorAttachment {
                view: &self.hdr_texture,
                resolve_target: None,
                ops,
            },
        }
    }
}

#[derive(Component)]
pub struct ViewDepthTexture {
    pub texture: Texture,
    pub view: TextureView,
}

fn prepare_view_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<ViewUniforms>,
    views: Query<(Entity, &ExtractedView)>,
) {
    view_uniforms.uniforms.clear();
    for (entity, camera) in views.iter() {
        let projection = camera.projection;
        let view = camera.transform.compute_matrix();
        let inverse_view = view.inverse();
        let view_uniforms = ViewUniformOffset {
            offset: view_uniforms.uniforms.push(ViewUniform {
                view_proj: projection * inverse_view,
                view,
                inverse_view,
                projection,
                world_position: camera.transform.translation,
                near: camera.near,
                far: camera.far,
                width: camera.width as f32,
                height: camera.height as f32,
            }),
        };

        commands.entity(entity).insert(view_uniforms);
    }

    view_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

#[allow(clippy::too_many_arguments)]
fn prepare_view_targets(
    mut commands: Commands,
    windows: Res<ExtractedWindows>,
    images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    cameras: Query<(Entity, &ExtractedCamera)>,
) {
    for (entity, camera) in cameras.iter() {
        if let Some(size) = camera.physical_size {
            if let Some(texture_view) = camera.target.get_texture_view(&windows, &images) {
                let view_target = ViewTarget::new(
                    &*render_device,
                    &mut *texture_cache,
                    &*msaa,
                    size,
                    texture_view.clone(),
                );

                /*let sampled_target = if msaa.samples > 1 {
                    let sampled_texture = texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("sampled_color_attachment_texture"),
                            size: Extent3d {
                                width: size.x,
                                height: size.y,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: msaa.samples,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::bevy_default(),
                            usage: TextureUsages::RENDER_ATTACHMENT,
                        },
                    );
                    Some(sampled_texture.default_view.clone())
                } else {
                    None
                };*/
                commands.entity(entity).insert(view_target);
            }
        }
    }
}
