use core::f32::consts::PI;

use super::{RaytracingMesh3d, RaytracingSceneBindings};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_camera::{visibility::InheritedVisibility, Camera};
use bevy_color::{ColorToComponents, LinearRgba};
use bevy_ecs::{
    component::Component,
    lifecycle::RemovedComponents,
    message::MessageReader,
    query::{Added, Changed, Or, With},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_light::{
    DirectionalLight, EnvironmentMapLight, GeneratedEnvironmentMapLight, PointLight, RectLight,
    SpotLight, SunDisk,
};
use bevy_math::{ops::cos, Quat, Vec3};
use bevy_pbr::{MeshMaterial3d, PreviousGlobalTransform, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_render::{sync_world::RenderEntity, Extract};
use bevy_transform::components::GlobalTransform;
use bevy_utils::once;
use tracing::warn;

/// Creates or removes components in the render world related to raytracing instances.
pub fn extract_raytracing_scene_structural(
    new_instances: Extract<
        Query<
            (
                RenderEntity,
                &RaytracingMesh3d,
                &MeshMaterial3d<StandardMaterial>,
                &GlobalTransform,
                Option<&PreviousGlobalTransform>,
            ),
            Added<RaytracingMesh3d>,
        >,
    >,
    mut removed_raytracing_meshes: Extract<RemovedComponents<RaytracingMesh3d>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    // Process removed components before additions, that way it properly handles same-frame removal->insertion
    for main_entity in removed_raytracing_meshes.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands.entity(render_entity).remove::<RaytracingMesh3d>();
        }
    }

    for (render_entity, mesh, material, transform, previous_frame_transform) in &new_instances {
        commands.entity(render_entity).insert((
            mesh.clone(),
            material.clone(),
            *transform,
            previous_frame_transform
                .cloned()
                .unwrap_or(PreviousGlobalTransform(transform.affine())),
        ));
    }
}

/// Copies the transforms of moved raytracing instances from the main world
/// straight into their GPU buffers.
pub fn extract_raytracing_scene_transforms(
    main_instances: Extract<
        Query<
            (
                RenderEntity,
                &GlobalTransform,
                Option<&PreviousGlobalTransform>,
            ),
            (
                Or<(Changed<GlobalTransform>, Changed<PreviousGlobalTransform>)>,
                With<RaytracingMesh3d>,
            ),
        >,
    >,
    bindings: Res<RaytracingSceneBindings>,
) {
    main_instances
        .par_iter()
        .for_each(|(render_entity, transform, previous_frame_transform)| {
            let previous_frame_transform = previous_frame_transform
                .cloned()
                .unwrap_or(PreviousGlobalTransform(transform.affine()));

            bindings.move_instance(render_entity, transform, &previous_frame_transform);
        });
}

/// Updates the mesh and material of existing raytracing instances in the render world.
pub fn extract_raytracing_scene_meshes_and_materials(
    main_instances: Extract<
        Query<
            (
                RenderEntity,
                &RaytracingMesh3d,
                &MeshMaterial3d<StandardMaterial>,
            ),
            Or<(
                Changed<RaytracingMesh3d>,
                Changed<MeshMaterial3d<StandardMaterial>>,
            )>,
        >,
    >,
    mut render_instances: Query<(&mut RaytracingMesh3d, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (render_entity, new_mesh, new_material) in &main_instances {
        if let Ok((mut mesh, mut material)) = render_instances.get_mut(render_entity) {
            *mesh = new_mesh.clone();
            *material = new_material.clone();
        }
    }
}

/// The set of [`StandardMaterial`] in the scene, mirrored into the render world.
#[derive(Resource, Default)]
pub struct StandardMaterialAssets {
    materials: HashMap<AssetId<StandardMaterial>, StandardMaterial>,
    /// Materials added or modified this frame.
    pub changed: Vec<AssetId<StandardMaterial>>,
    /// Materials removed this frame.
    pub removed: Vec<AssetId<StandardMaterial>>,
}

impl StandardMaterialAssets {
    pub fn get(&self, id: &AssetId<StandardMaterial>) -> Option<&StandardMaterial> {
        self.materials.get(id)
    }
}

/// Keeps [`StandardMaterialAssets`] up to date in the render world.
pub fn extract_raytracing_material_assets(
    main_materials: Extract<Res<Assets<StandardMaterial>>>,
    mut render_materials: ResMut<StandardMaterialAssets>,
    mut events: Extract<MessageReader<AssetEvent<StandardMaterial>>>,
) {
    let render_materials = &mut *render_materials;

    render_materials.changed.clear();
    render_materials.removed.clear();

    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some(material) = main_materials.get(*id) {
                    render_materials.materials.insert(*id, material.clone());
                    render_materials.changed.push(*id);
                }
            }
            AssetEvent::Removed { id } => {
                render_materials.materials.remove(id);
                render_materials.removed.push(*id);
            }
            AssetEvent::Unused { .. } | AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct ExtractedRaytracingDirectionalLight {
    pub direction_to_light: Vec3,
    pub illuminance: Vec3,
    pub sun_disk_angular_size: f32,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct ExtractedRaytracingPointLight {
    pub position: Vec3,
    pub radius: f32,
    pub intensity: Vec3,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct ExtractedRaytracingSpotLight {
    pub position: Vec3,
    pub radius: f32,
    pub intensity: Vec3,
    pub axis: Vec3,
    pub cos_inner: f32,
    pub cos_outer: f32,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct ExtractedRaytracingRectLight {
    pub center: Vec3,
    pub edge_x: Vec3,
    pub edge_y: Vec3,
    pub area: f32,
    pub radiance: Vec3,
}

#[derive(Resource, Default, Clone, PartialEq)]
pub struct ExtractedEnvironmentMapLight {
    pub cubemap: Option<Handle<Image>>,
    pub intensity: f32,
    pub rotation: Quat,
}

/// Lights whose extracted data needs updating.
type ChangedLightFilter<L> = Or<(
    Changed<L>,
    Changed<GlobalTransform>,
    Changed<InheritedVisibility>,
)>;

type ChangedLightQuery<'w, 's, L> = Query<
    'w,
    's,
    (
        RenderEntity,
        &'static L,
        &'static GlobalTransform,
        &'static InheritedVisibility,
    ),
    ChangedLightFilter<L>,
>;

/// Keeps the render world's directional lights up to date.
pub fn extract_raytracing_directional_lights(
    changed_directional_lights: Extract<
        Query<
            (
                RenderEntity,
                &DirectionalLight,
                &GlobalTransform,
                &InheritedVisibility,
                Option<&SunDisk>,
            ),
            Or<(ChangedLightFilter<DirectionalLight>, Changed<SunDisk>)>,
        >,
    >,
    directional_lights: Extract<
        Query<(
            RenderEntity,
            &DirectionalLight,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&SunDisk>,
        )>,
    >,
    mut removed_directional_lights: Extract<RemovedComponents<DirectionalLight>>,
    mut removed_sun_disks: Extract<RemovedComponents<SunDisk>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    // Process removed components before changes, that way it properly handles same-frame removal->insertion
    for main_entity in removed_directional_lights.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands
                .entity(render_entity)
                .remove::<ExtractedRaytracingDirectionalLight>();
        }
    }

    // Removing a light's SunDisk changes its angular size back to the default, but no change
    // filter fires for a component that no longer exists, so re-extract those lights too
    let sun_disk_removed = removed_sun_disks
        .read()
        .filter_map(|main_entity| directional_lights.get(main_entity).ok());

    for (render_entity, directional_light, transform, visibility, sun_disk) in
        changed_directional_lights.iter().chain(sun_disk_removed)
    {
        let mut entity_commands = commands.entity(render_entity);
        if !visibility.get() {
            entity_commands.remove::<ExtractedRaytracingDirectionalLight>();
            continue;
        }

        entity_commands.insert(ExtractedRaytracingDirectionalLight {
            direction_to_light: transform.back().into(),
            illuminance: LinearRgba::from(directional_light.color).to_vec3()
                * directional_light.illuminance,
            sun_disk_angular_size: sun_disk.unwrap_or_default().angular_size,
        });
    }
}

/// Keeps the render world's point lights up to date.
pub fn extract_raytracing_point_lights(
    changed_point_lights: Extract<ChangedLightQuery<PointLight>>,
    mut removed_point_lights: Extract<RemovedComponents<PointLight>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    // Process removed components before changes, that way it properly handles same-frame removal->insertion
    for main_entity in removed_point_lights.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands
                .entity(render_entity)
                .remove::<ExtractedRaytracingPointLight>();
        }
    }

    for (render_entity, point_light, transform, visibility) in &changed_point_lights {
        let mut entity_commands = commands.entity(render_entity);
        if !visibility.get() {
            entity_commands.remove::<ExtractedRaytracingPointLight>();
            continue;
        }

        entity_commands.insert(ExtractedRaytracingPointLight {
            position: transform.translation(),
            radius: point_light.radius,
            // Map from luminous power in lumens to luminous intensity in candela
            intensity: LinearRgba::from(point_light.color).to_vec3() * point_light.intensity
                / (4.0 * PI),
        });
    }
}

/// Keeps the render world's spot lights up to date.
pub fn extract_raytracing_spot_lights(
    changed_spot_lights: Extract<ChangedLightQuery<SpotLight>>,
    mut removed_spot_lights: Extract<RemovedComponents<SpotLight>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    // Process removed components before changes, that way it properly handles same-frame removal->insertion
    for main_entity in removed_spot_lights.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands
                .entity(render_entity)
                .remove::<ExtractedRaytracingSpotLight>();
        }
    }

    for (render_entity, spot_light, transform, visibility) in &changed_spot_lights {
        let mut entity_commands = commands.entity(render_entity);
        if !visibility.get() {
            entity_commands.remove::<ExtractedRaytracingSpotLight>();
            continue;
        }

        let outer_angle = spot_light.outer_angle;
        let inner_angle = spot_light.inner_angle.min(outer_angle);

        entity_commands.insert(ExtractedRaytracingSpotLight {
            position: transform.translation(),
            radius: spot_light.radius,
            intensity: LinearRgba::from(spot_light.color).to_vec3() * spot_light.intensity
                / (4.0 * PI),
            axis: transform.forward().into(),
            cos_inner: cos(inner_angle),
            cos_outer: cos(outer_angle),
        });
    }
}

/// Keeps the render world's rect lights up to date.
pub fn extract_raytracing_rect_lights(
    changed_rect_lights: Extract<ChangedLightQuery<RectLight>>,
    mut removed_rect_lights: Extract<RemovedComponents<RectLight>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    // Process removed components before changes, that way it properly handles same-frame removal->insertion
    for main_entity in removed_rect_lights.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands
                .entity(render_entity)
                .remove::<ExtractedRaytracingRectLight>();
        }
    }

    for (render_entity, rect_light, transform, visibility) in &changed_rect_lights {
        let affine = transform.affine();
        let edge_x = Vec3::from(affine.matrix3.x_axis) * rect_light.width;
        let edge_y = Vec3::from(affine.matrix3.y_axis) * rect_light.height;
        let area = edge_x.cross(edge_y).length();

        let mut entity_commands = commands.entity(render_entity);
        if !visibility.get() || !area.is_finite() || area <= 0.0 {
            entity_commands.remove::<ExtractedRaytracingRectLight>();
            continue;
        }

        entity_commands.insert(ExtractedRaytracingRectLight {
            center: transform.translation(),
            edge_x,
            edge_y,
            area,
            // Map from luminous power in lumens to luminance
            radiance: LinearRgba::from(rect_light.color).to_vec3() * rect_light.intensity
                / (area * PI),
        });
    }
}

/// Finds the environment map light to use for the raytraced scene, if any.
pub fn extract_raytracing_environment_map_light(
    cameras: Extract<
        Query<(
            &Camera,
            Option<&GeneratedEnvironmentMapLight>,
            Option<&EnvironmentMapLight>,
        )>,
    >,
    mut environment_map_light: ResMut<ExtractedEnvironmentMapLight>,
) {
    let mut extracted_env_map_light = ExtractedEnvironmentMapLight::default();

    for (camera, generated, pregenerated) in &cameras {
        if !camera.is_active {
            continue;
        }

        let env_map_light = match (generated, pregenerated) {
            (Some(generated), _) => ExtractedEnvironmentMapLight {
                cubemap: Some(generated.environment_map.clone()),
                intensity: generated.intensity,
                rotation: generated.rotation,
            },
            (None, Some(pregenerated)) => ExtractedEnvironmentMapLight {
                cubemap: Some(pregenerated.specular_map.clone()),
                intensity: pregenerated.intensity,
                rotation: pregenerated.rotation,
            },
            (None, None) => continue,
        };

        if extracted_env_map_light.cubemap.is_none() {
            extracted_env_map_light = env_map_light;
        } else if extracted_env_map_light != env_map_light {
            once!(warn!(
                "bevy_solari only supports a single environment light for the whole scene, but \
                 multiple cameras have differing environment lights. Using the first one found."
            ));
        }
    }

    *environment_map_light = extracted_env_map_light;
}
