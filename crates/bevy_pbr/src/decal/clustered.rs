//! Clustered decals, bounding regions that project textures onto surfaces.
//!
//! A *clustered decal* is a bounding box that projects a texture onto any
//! surface within its bounds along the positive Z axis. In Bevy, clustered
//! decals use the *clustered forward* rendering technique.
//!
//! Clustered decals are the highest-quality types of decals that Bevy supports,
//! but they require bindless textures. This means that they presently can't be
//! used on WebGL 2, WebGPU, macOS, or iOS. Bevy's clustered decals can be used
//! with forward or deferred rendering and don't require a prepass.
//!
//! On their own, clustered decals only project the base color of a texture. You
//! can, however, use the built-in *tag* field to customize the appearance of a
//! clustered decal arbitrarily. See the documentation in `clustered.wgsl` for
//! more information and the `clustered_decals` example for an example of use.

use core::{num::NonZero, ops::Deref};

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    prelude::ReflectComponent,
    query::With,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_math::Mat4;
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_resource::{
        binding_types, BindGroupLayoutEntryBuilder, Buffer, BufferUsages, RawBufferVec, Sampler,
        SamplerBindingType, Shader, ShaderType, TextureSampleType, TextureView,
    },
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    texture::{FallbackImage, GpuImage},
    view::{self, ViewVisibility, Visibility, VisibilityClass},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::{components::GlobalTransform, prelude::Transform};
use bytemuck::{Pod, Zeroable};

use crate::{
    binding_arrays_are_usable, prepare_lights, GlobalClusterableObjectMeta, LightVisibilityClass,
};

/// The handle to the `clustered.wgsl` shader.
pub(crate) const CLUSTERED_DECAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("87929002-3509-42f1-8279-2d2765dd145c");

/// The maximum number of decals that can be present in a view.
///
/// This number is currently relatively low in order to work around the lack of
/// first-class binding arrays in `wgpu`. When that feature is implemented, this
/// limit can be increased.
pub(crate) const MAX_VIEW_DECALS: usize = 8;

/// A plugin that adds support for clustered decals.
///
/// In environments where bindless textures aren't available, clustered decals
/// can still be added to a scene, but they won't project any decals.
pub struct ClusteredDecalPlugin;

/// An object that projects a decal onto surfaces within its bounds.
///
/// Conceptually, a clustered decal is a 1×1×1 cube centered on its origin. It
/// projects the given [`Self::image`] onto surfaces in the +Z direction (thus
/// you may find [`Transform::looking_at`] useful).
///
/// Clustered decals are the highest-quality types of decals that Bevy supports,
/// but they require bindless textures. This means that they presently can't be
/// used on WebGL 2, WebGPU, macOS, or iOS. Bevy's clustered decals can be used
/// with forward or deferred rendering and don't require a prepass.
#[derive(Component, Debug, Clone, Reflect, ExtractComponent)]
#[reflect(Component, Debug, Clone)]
#[require(Transform, Visibility, VisibilityClass)]
#[component(on_add = view::add_visibility_class::<LightVisibilityClass>)]
pub struct ClusteredDecal {
    /// The image that the clustered decal projects.
    ///
    /// This must be a 2D image. If it has an alpha channel, it'll be alpha
    /// blended with the underlying surface and/or other decals. All decal
    /// images in the scene must use the same sampler.
    pub image: Handle<Image>,

    /// An application-specific tag you can use for any purpose you want.
    ///
    /// See the `clustered_decals` example for an example of use.
    pub tag: u32,
}

/// Stores information about all the clustered decals in the scene.
#[derive(Resource, Default)]
pub struct RenderClusteredDecals {
    /// Maps an index in the shader binding array to the associated decal image.
    ///
    /// [`Self::texture_to_binding_index`] holds the inverse mapping.
    binding_index_to_textures: Vec<AssetId<Image>>,
    /// Maps a decal image to the shader binding array.
    ///
    /// [`Self::binding_index_to_textures`] holds the inverse mapping.
    texture_to_binding_index: HashMap<AssetId<Image>, u32>,
    /// The information concerning each decal that we provide to the shader.
    decals: Vec<RenderClusteredDecal>,
    /// Maps the [`bevy_render::sync_world::RenderEntity`] of each decal to the
    /// index of that decal in the [`Self::decals`] list.
    entity_to_decal_index: EntityHashMap<usize>,
}

impl RenderClusteredDecals {
    /// Clears out this [`RenderClusteredDecals`] in preparation for a new
    /// frame.
    fn clear(&mut self) {
        self.binding_index_to_textures.clear();
        self.texture_to_binding_index.clear();
        self.decals.clear();
        self.entity_to_decal_index.clear();
    }
}

/// The per-view bind group entries pertaining to decals.
pub(crate) struct RenderViewClusteredDecalBindGroupEntries<'a> {
    /// The list of decals, corresponding to `mesh_view_bindings::decals` in the
    /// shader.
    pub(crate) decals: &'a Buffer,
    /// The list of textures, corresponding to
    /// `mesh_view_bindings::decal_textures` in the shader.
    pub(crate) texture_views: Vec<&'a <TextureView as Deref>::Target>,
    /// The sampler that the shader uses to sample decals, corresponding to
    /// `mesh_view_bindings::decal_sampler` in the shader.
    pub(crate) sampler: &'a Sampler,
}

/// A render-world resource that holds the buffer of [`ClusteredDecal`]s ready
/// to upload to the GPU.
#[derive(Resource, Deref, DerefMut)]
pub struct DecalsBuffer(RawBufferVec<RenderClusteredDecal>);

impl Default for DecalsBuffer {
    fn default() -> Self {
        DecalsBuffer(RawBufferVec::new(BufferUsages::STORAGE))
    }
}

impl Plugin for ClusteredDecalPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            CLUSTERED_DECAL_SHADER_HANDLE,
            "clustered.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(ExtractComponentPlugin::<ClusteredDecal>::default())
            .register_type::<ClusteredDecal>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DecalsBuffer>()
            .init_resource::<RenderClusteredDecals>()
            .add_systems(ExtractSchedule, extract_decals)
            .add_systems(
                Render,
                prepare_decals
                    .in_set(RenderSet::ManageViews)
                    .after(prepare_lights),
            )
            .add_systems(Render, upload_decals.in_set(RenderSet::PrepareResources));
    }
}

/// The GPU data structure that stores information about each decal.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct RenderClusteredDecal {
    /// The inverse of the model matrix.
    ///
    /// The shader uses this in order to back-transform world positions into
    /// model space.
    local_from_world: Mat4,
    /// The index of the decal texture in the binding array.
    image_index: u32,
    /// A custom tag available for application-defined purposes.
    tag: u32,
    /// Padding.
    pad_a: u32,
    /// Padding.
    pad_b: u32,
}

/// Extracts decals from the main world into the render world.
pub fn extract_decals(
    decals: Extract<
        Query<(
            RenderEntity,
            &ClusteredDecal,
            &GlobalTransform,
            &ViewVisibility,
        )>,
    >,
    mut render_decals: ResMut<RenderClusteredDecals>,
) {
    // Clear out the `RenderDecals` in preparation for a new frame.
    render_decals.clear();

    // Loop over each decal.
    for (decal_entity, clustered_decal, global_transform, view_visibility) in &decals {
        // If the decal is invisible, skip it.
        if !view_visibility.get() {
            continue;
        }

        // Insert or add the image.
        let image_index = render_decals.get_or_insert_image(&clustered_decal.image.id());

        // Record the decal.
        let decal_index = render_decals.decals.len();
        render_decals
            .entity_to_decal_index
            .insert(decal_entity, decal_index);

        render_decals.decals.push(RenderClusteredDecal {
            local_from_world: global_transform.affine().inverse().into(),
            image_index,
            tag: clustered_decal.tag,
            pad_a: 0,
            pad_b: 0,
        });
    }
}

/// Adds all decals in the scene to the [`GlobalClusterableObjectMeta`] table.
fn prepare_decals(
    decals: Query<Entity, With<ClusteredDecal>>,
    mut global_clusterable_object_meta: ResMut<GlobalClusterableObjectMeta>,
    render_decals: Res<RenderClusteredDecals>,
) {
    for decal_entity in &decals {
        if let Some(index) = render_decals.entity_to_decal_index.get(&decal_entity) {
            global_clusterable_object_meta
                .entity_to_index
                .insert(decal_entity, *index);
        }
    }
}

/// Returns the layout for the clustered-decal-related bind group entries for a
/// single view.
pub(crate) fn get_bind_group_layout_entries(
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
) -> Option<[BindGroupLayoutEntryBuilder; 3]> {
    // If binding arrays aren't supported on the current platform, we have no
    // bind group layout entries.
    if !clustered_decals_are_usable(render_device, render_adapter) {
        return None;
    }

    Some([
        // `decals`
        binding_types::storage_buffer_read_only::<RenderClusteredDecal>(false),
        // `decal_textures`
        binding_types::texture_2d(TextureSampleType::Float { filterable: true })
            .count(NonZero::<u32>::new(MAX_VIEW_DECALS as u32).unwrap()),
        // `decal_sampler`
        binding_types::sampler(SamplerBindingType::Filtering),
    ])
}

impl<'a> RenderViewClusteredDecalBindGroupEntries<'a> {
    /// Creates and returns the bind group entries for clustered decals for a
    /// single view.
    pub(crate) fn get(
        render_decals: &RenderClusteredDecals,
        decals_buffer: &'a DecalsBuffer,
        images: &'a RenderAssets<GpuImage>,
        fallback_image: &'a FallbackImage,
        render_device: &RenderDevice,
        render_adapter: &RenderAdapter,
    ) -> Option<RenderViewClusteredDecalBindGroupEntries<'a>> {
        // Skip the entries if decals are unsupported on the current platform.
        if !clustered_decals_are_usable(render_device, render_adapter) {
            return None;
        }

        // We use the first sampler among all the images. This assumes that all
        // images use the same sampler, which is a documented restriction. If
        // there's no sampler, we just use the one from the fallback image.
        let sampler = match render_decals
            .binding_index_to_textures
            .iter()
            .filter_map(|image_id| images.get(*image_id))
            .next()
        {
            Some(gpu_image) => &gpu_image.sampler,
            None => &fallback_image.d2.sampler,
        };

        // Gather up the decal textures.
        let mut texture_views = vec![];
        for image_id in &render_decals.binding_index_to_textures {
            match images.get(*image_id) {
                None => texture_views.push(&*fallback_image.d2.texture_view),
                Some(gpu_image) => texture_views.push(&*gpu_image.texture_view),
            }
        }

        // Pad out the binding array to its maximum length, which is
        // required on some platforms.
        while texture_views.len() < MAX_VIEW_DECALS {
            texture_views.push(&*fallback_image.d2.texture_view);
        }

        Some(RenderViewClusteredDecalBindGroupEntries {
            decals: decals_buffer.buffer()?,
            texture_views,
            sampler,
        })
    }
}

impl RenderClusteredDecals {
    /// Returns the index of the given image in the decal texture binding array,
    /// adding it to the list if necessary.
    fn get_or_insert_image(&mut self, image_id: &AssetId<Image>) -> u32 {
        *self
            .texture_to_binding_index
            .entry(*image_id)
            .or_insert_with(|| {
                let index = self.binding_index_to_textures.len() as u32;
                self.binding_index_to_textures.push(*image_id);
                index
            })
    }
}

/// Uploads the list of decals from [`RenderClusteredDecals::decals`] to the
/// GPU.
fn upload_decals(
    render_decals: Res<RenderClusteredDecals>,
    mut decals_buffer: ResMut<DecalsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    decals_buffer.clear();

    for &decal in &render_decals.decals {
        decals_buffer.push(decal);
    }

    // Make sure the buffer is non-empty.
    // Otherwise there won't be a buffer to bind.
    if decals_buffer.is_empty() {
        decals_buffer.push(RenderClusteredDecal::default());
    }

    decals_buffer.write_buffer(&render_device, &render_queue);
}

/// Returns true if clustered decals are usable on the current platform or false
/// otherwise.
///
/// Clustered decals are currently disabled on macOS and iOS due to insufficient
/// texture bindings and limited bindless support in `wgpu`.
pub fn clustered_decals_are_usable(
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
) -> bool {
    // Disable binding arrays on Metal. There aren't enough texture bindings available.
    // See issue #17553.
    // Re-enable this when `wgpu` has first-class bindless.
    binding_arrays_are_usable(render_device, render_adapter)
        && cfg!(not(any(target_os = "macos", target_os = "ios")))
}
