use bevy_asset::Handle;
use bevy_color::ColorToComponents;
use bevy_core_pipeline::{
    core_3d::Camera3d,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_ecs::{
    entity::Entity,
    query::{Has, With},
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::{vec4, Mat3A, Mat4, Vec3A, Vec4, Vec4Swizzles};
use bevy_render::{
    mesh::RenderMesh,
    render_asset::RenderAssets,
    render_resource::{PipelineCache, SpecializedRenderPipelines, TextureUsages},
    renderer::{RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    view::{ExtractedView, Msaa},
    Extract,
};
use bevy_render_3d::{MeshPipelineViewLayoutKey, VolumetricLight};
use bevy_transform::components::GlobalTransform;

use crate::{
    render::{
        ViewFogVolume, ViewVolumetricFog, ViewVolumetricFogPipelines, VolumetricFogPipeline,
        VolumetricFogPipelineKey, VolumetricFogPipelineKeyFlags, VolumetricFogUniform,
        VolumetricFogUniformBuffer,
    },
    FogVolume, VolumetricFog,
};

use super::PLANE_MESH;

/// A matrix that converts from local 1×1×1 space to UVW 3D density texture
/// space.
static UVW_FROM_LOCAL: Mat4 = Mat4::from_cols(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, 1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.5, 0.5, 0.5, 1.0),
);

/// Extracts [`VolumetricFog`], [`FogVolume`], and [`VolumetricLight`]s
/// from the main world to the render world.
pub fn extract_volumetric_fog(
    mut commands: Commands,
    view_targets: Extract<Query<(RenderEntity, &VolumetricFog)>>,
    fog_volumes: Extract<Query<(RenderEntity, &FogVolume, &GlobalTransform)>>,
    volumetric_lights: Extract<Query<(RenderEntity, &VolumetricLight)>>,
) {
    if volumetric_lights.is_empty() {
        // TODO: needs better way to handle clean up in render world
        for (entity, ..) in view_targets.iter() {
            commands
                .entity(entity)
                .remove::<(VolumetricFog, ViewVolumetricFogPipelines, ViewVolumetricFog)>();
        }
        for (entity, ..) in fog_volumes.iter() {
            commands.entity(entity).remove::<FogVolume>();
        }
        return;
    }

    for (entity, volumetric_fog) in view_targets.iter() {
        commands
            .get_entity(entity)
            .expect("Volumetric fog entity wasn't synced.")
            .insert(*volumetric_fog);
    }

    for (entity, fog_volume, fog_transform) in fog_volumes.iter() {
        commands
            .get_entity(entity)
            .expect("Fog volume entity wasn't synced.")
            .insert((*fog_volume).clone())
            .insert(*fog_transform);
    }

    for (entity, volumetric_light) in volumetric_lights.iter() {
        commands
            .get_entity(entity)
            .expect("Volumetric light entity wasn't synced.")
            .insert(*volumetric_light);
    }
}

/// Specializes volumetric fog pipelines for all views with that effect enabled.
pub fn prepare_volumetric_fog_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<VolumetricFogPipeline>>,
    volumetric_lighting_pipeline: Res<VolumetricFogPipeline>,
    view_targets: Query<
        (
            Entity,
            &ExtractedView,
            &Msaa,
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        With<VolumetricFog>,
    >,
    meshes: Res<RenderAssets<RenderMesh>>,
) {
    let Some(plane_mesh) = meshes.get(&PLANE_MESH) else {
        // There's an off chance that the mesh won't be prepared yet if `RenderAssetBytesPerFrame` limiting is in use.
        return;
    };

    for (
        entity,
        view,
        msaa,
        normal_prepass,
        depth_prepass,
        motion_vector_prepass,
        deferred_prepass,
    ) in view_targets.iter()
    {
        // Create a mesh pipeline view layout key corresponding to the view.
        let mut mesh_pipeline_view_key = MeshPipelineViewLayoutKey::from(*msaa);
        mesh_pipeline_view_key.set(MeshPipelineViewLayoutKey::NORMAL_PREPASS, normal_prepass);
        mesh_pipeline_view_key.set(MeshPipelineViewLayoutKey::DEPTH_PREPASS, depth_prepass);
        mesh_pipeline_view_key.set(
            MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS,
            motion_vector_prepass,
        );
        mesh_pipeline_view_key.set(
            MeshPipelineViewLayoutKey::DEFERRED_PREPASS,
            deferred_prepass,
        );

        let mut textureless_flags = VolumetricFogPipelineKeyFlags::empty();
        textureless_flags.set(VolumetricFogPipelineKeyFlags::HDR, view.hdr);

        // Specialize the pipeline.
        let textureless_pipeline_key = VolumetricFogPipelineKey {
            mesh_pipeline_view_key,
            vertex_buffer_layout: plane_mesh.layout.clone(),
            flags: textureless_flags,
        };
        let textureless_pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &volumetric_lighting_pipeline,
            textureless_pipeline_key.clone(),
        );
        let textured_pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &volumetric_lighting_pipeline,
            VolumetricFogPipelineKey {
                flags: textureless_pipeline_key.flags
                    | VolumetricFogPipelineKeyFlags::DENSITY_TEXTURE,
                ..textureless_pipeline_key
            },
        );

        commands.entity(entity).insert(ViewVolumetricFogPipelines {
            textureless: textureless_pipeline_id,
            textured: textured_pipeline_id,
        });
    }
}

/// A system that converts [`VolumetricFog`] into [`VolumetricFogUniform`]s.
pub fn prepare_volumetric_fog_uniforms(
    mut commands: Commands,
    mut volumetric_lighting_uniform_buffer: ResMut<VolumetricFogUniformBuffer>,
    view_targets: Query<(Entity, &ExtractedView, &VolumetricFog)>,
    fog_volumes: Query<(Entity, &FogVolume, &GlobalTransform)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut local_from_world_matrices: Local<Vec<Mat4>>,
) {
    // Do this up front to avoid O(n^2) matrix inversion.
    local_from_world_matrices.clear();
    for (_, _, fog_transform) in fog_volumes.iter() {
        local_from_world_matrices.push(fog_transform.compute_matrix().inverse());
    }

    let uniform_count = view_targets.iter().len() * local_from_world_matrices.len();

    let Some(mut writer) =
        volumetric_lighting_uniform_buffer.get_writer(uniform_count, &render_device, &render_queue)
    else {
        return;
    };

    for (view_entity, extracted_view, volumetric_fog) in view_targets.iter() {
        let world_from_view = extracted_view.world_from_view.compute_matrix();

        let mut view_fog_volumes = vec![];

        for ((_, fog_volume, _), local_from_world) in
            fog_volumes.iter().zip(local_from_world_matrices.iter())
        {
            // Calculate the transforms to and from 1×1×1 local space.
            let local_from_view = *local_from_world * world_from_view;
            let view_from_local = local_from_view.inverse();

            // Determine whether the camera is inside or outside the volume, and
            // calculate the clip space transform.
            let interior = camera_is_inside_fog_volume(&local_from_view);
            let hull_clip_from_local = calculate_fog_volume_clip_from_local_transforms(
                interior,
                &extracted_view.clip_from_view,
                &view_from_local,
            );

            // Calculate the radius of the sphere that bounds the fog volume.
            let bounding_radius = (Mat3A::from_mat4(view_from_local) * Vec3A::splat(0.5)).length();

            // Write out our uniform.
            let uniform_buffer_offset = writer.write(&VolumetricFogUniform {
                clip_from_local: hull_clip_from_local,
                uvw_from_world: UVW_FROM_LOCAL * *local_from_world,
                far_planes: get_far_planes(&view_from_local),
                fog_color: fog_volume.fog_color.to_linear().to_vec3(),
                light_tint: fog_volume.light_tint.to_linear().to_vec3(),
                ambient_color: volumetric_fog.ambient_color.to_linear().to_vec3(),
                ambient_intensity: volumetric_fog.ambient_intensity,
                step_count: volumetric_fog.step_count,
                bounding_radius,
                absorption: fog_volume.absorption,
                scattering: fog_volume.scattering,
                density: fog_volume.density_factor,
                density_texture_offset: fog_volume.density_texture_offset,
                scattering_asymmetry: fog_volume.scattering_asymmetry,
                light_intensity: fog_volume.light_intensity,
                jitter_strength: volumetric_fog.jitter,
            });

            view_fog_volumes.push(ViewFogVolume {
                uniform_buffer_offset,
                exterior: !interior,
                density_texture: fog_volume.density_texture.as_ref().map(Handle::id),
            });
        }

        commands
            .entity(view_entity)
            .insert(ViewVolumetricFog(view_fog_volumes));
    }
}

/// A system that marks all view depth textures as readable in shaders.
///
/// The volumetric lighting pass needs to do this, and it doesn't happen by
/// default.
pub fn prepare_view_depth_textures_for_volumetric_fog(
    mut view_targets: Query<&mut Camera3d>,
    fog_volumes: Query<&VolumetricFog>,
) {
    if fog_volumes.is_empty() {
        return;
    }

    for mut camera in view_targets.iter_mut() {
        camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
    }
}

fn get_far_planes(view_from_local: &Mat4) -> [Vec4; 3] {
    let (mut far_planes, mut next_index) = ([Vec4::ZERO; 3], 0);
    let view_from_normal_local = Mat3A::from_mat4(*view_from_local);

    for &local_normal in &[
        Vec3A::X,
        Vec3A::NEG_X,
        Vec3A::Y,
        Vec3A::NEG_Y,
        Vec3A::Z,
        Vec3A::NEG_Z,
    ] {
        let view_normal = (view_from_normal_local * local_normal).normalize_or_zero();
        if view_normal.z <= 0.0 {
            continue;
        }

        let view_position = *view_from_local * (-local_normal * 0.5).extend(1.0);
        let plane_coords = view_normal.extend(-view_normal.dot(view_position.xyz().into()));

        far_planes[next_index] = plane_coords;
        next_index += 1;
        if next_index == far_planes.len() {
            continue;
        }
    }

    far_planes
}

/// Given the transform from the view to the 1×1×1 cube in local fog volume
/// space, returns true if the camera is inside the volume.
fn camera_is_inside_fog_volume(local_from_view: &Mat4) -> bool {
    Vec3A::from(local_from_view.col(3).xyz())
        .abs()
        .cmple(Vec3A::splat(0.5))
        .all()
}

/// Given the local transforms, returns the matrix that transforms model space
/// to clip space.
fn calculate_fog_volume_clip_from_local_transforms(
    interior: bool,
    clip_from_view: &Mat4,
    view_from_local: &Mat4,
) -> Mat4 {
    if !interior {
        return *clip_from_view * *view_from_local;
    }

    // If the camera is inside the fog volume, then we'll be rendering a full
    // screen quad. The shader will start its raymarch at the fragment depth
    // value, however, so we need to make sure that the depth of the full screen
    // quad is at the near clip plane `z_near`.
    let z_near = clip_from_view.w_axis[2];
    Mat4::from_cols(
        vec4(z_near, 0.0, 0.0, 0.0),
        vec4(0.0, z_near, 0.0, 0.0),
        vec4(0.0, 0.0, 0.0, 0.0),
        vec4(0.0, 0.0, z_near, z_near),
    )
}
