pub mod visibility;
pub mod window;

pub use visibility::*;
use wgpu::{
    Color, Extent3d, Operations, RenderPassColorAttachment, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};
pub use window::*;

use crate::{
    camera::{ExtractedCamera, ExtractedCameraNames},
    render_resource::{DynamicUniformVec, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, TextureCache},
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::GlobalTransform;
use crevice::std140::AsStd140;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Msaa>().add_plugin(VisibilityPlugin);

        app.sub_app(RenderApp)
            .init_resource::<ViewUniforms>()
            .add_system_to_stage(RenderStage::Extract, extract_msaa)
            .add_system_to_stage(RenderStage::Prepare, prepare_view_uniforms)
            .add_system_to_stage(
                RenderStage::Prepare,
                prepare_view_targets.after(WindowSystem::Prepare),
            );
    }
}

#[derive(Clone)]
pub struct Msaa {
    /// The number of samples to run for Multi-Sample Anti-Aliasing. Higher numbers result in
    /// smoother edges. Note that WGPU currently only supports 1 or 4 samples.
    /// Ultimately we plan on supporting whatever is natively supported on a given device.
    /// Check out this issue for more info: <https://github.com/gfx-rs/wgpu/issues/1832>
    /// It defaults to 1 in wasm - https://github.com/gfx-rs/wgpu/issues/2149     
    pub samples: u32,
}

impl Default for Msaa {
    fn default() -> Self {
        Self {
            #[cfg(feature = "webgl")]
            samples: 1,
            #[cfg(not(feature = "webgl"))]
            samples: 4,
        }
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
    pub view: TextureView,
    pub sampled_target: Option<TextureView>,
}

impl ViewTarget {
    pub fn get_color_attachment(&self, ops: Operations<Color>) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: if let Some(sampled_target) = &self.sampled_target {
                sampled_target
            } else {
                &self.view
            },
            resolve_target: if self.sampled_target.is_some() {
                Some(&self.view)
            } else {
                None
            },
            ops,
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
        let inverse_view = camera.transform.compute_matrix().inverse();
        let view_uniforms = ViewUniformOffset {
            offset: view_uniforms.uniforms.push(ViewUniform {
                view_proj: projection * inverse_view,
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

fn prepare_view_targets(
    mut commands: Commands,
    camera_names: Res<ExtractedCameraNames>,
    windows: Res<ExtractedWindows>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    cameras: Query<&ExtractedCamera>,
) {
    for entity in camera_names.entities.values().copied() {
        let camera = if let Ok(camera) = cameras.get(entity) {
            camera
        } else {
            continue;
        };
        let window = if let Some(window) = windows.get(&camera.window_id) {
            window
        } else {
            continue;
        };
        let swap_chain_texture = if let Some(texture) = &window.swap_chain_texture {
            texture
        } else {
            continue;
        };
        let sampled_target = if msaa.samples > 1 {
            let sampled_texture = texture_cache.get(
                &render_device,
                TextureDescriptor {
                    label: Some("sampled_color_attachment_texture"),
                    size: Extent3d {
                        width: window.physical_width,
                        height: window.physical_height,
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
        };

        commands.entity(entity).insert(ViewTarget {
            view: swap_chain_texture.clone(),
            sampled_target,
        });
    }
}
