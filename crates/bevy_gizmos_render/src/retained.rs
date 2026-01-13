//! This module is for 'retained' alternatives to the 'immediate mode' [`Gizmos`](bevy_gizmos::gizmos::Gizmos) system parameter.

use {
    crate::LineGizmoUniform,
    bevy_asset::{AssetEvent, AssetId, Assets},
    bevy_camera::{
        primitives::Aabb,
        visibility::{NoFrustumCulling, RenderLayers, ViewVisibility},
    },
    bevy_ecs::{
        change_detection::DetectChangesMut,
        entity::Entity,
        message::MessageReader,
        query::{Changed, Or, Without},
        system::{Commands, Local, Query, Res},
    },
    bevy_gizmos::{
        config::{GizmoLineJoint, GizmoLineStyle},
        retained::Gizmo,
        GizmoAsset,
    },
    bevy_math::{bounding::Aabb3d, Affine3, Isometry3d, Vec3A},
    bevy_platform::{collections::HashSet, hash::FixedHasher},
    bevy_render::{
        sync_world::{MainEntity, TemporaryRenderEntity},
        Extract,
    },
    bevy_transform::components::GlobalTransform,
    bevy_utils::once,
    tracing::warn,
};

pub(crate) fn extract_linegizmos(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<
        Query<(
            Entity,
            &Gizmo,
            &GlobalTransform,
            &ViewVisibility,
            Option<&RenderLayers>,
        )>,
    >,
) {
    let mut values = Vec::with_capacity(*previous_len);

    #[cfg_attr(
        not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
        expect(
            unused_variables,
            reason = "`render_layers` is unused when bevy_pbr and bevy_sprite_render are both disabled."
        )
    )]
    for (entity, gizmo, transform, view_visibility, render_layers) in &query {
        if !view_visibility.get() {
            continue;
        }
        let joints_resolution = if let GizmoLineJoint::Round(resolution) = gizmo.line_config.joints
        {
            resolution
        } else {
            0
        };
        let (gap_scale, line_scale) = if let GizmoLineStyle::Dashed {
            gap_scale,
            line_scale,
        } = gizmo.line_config.style
        {
            if gap_scale <= 0.0 {
                once!(warn!("when using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the gap scale should be greater than zero"));
            }
            if line_scale <= 0.0 {
                once!(warn!("when using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the line scale should be greater than zero"));
            }
            (gap_scale, line_scale)
        } else {
            (1.0, 1.0)
        };

        values.push((
            LineGizmoUniform {
                world_from_local: Affine3::from(&transform.affine()).to_transpose(),
                line_width: gizmo.line_config.width,
                depth_bias: gizmo.depth_bias,
                joints_resolution,
                gap_scale,
                line_scale,
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _webgl2_padding: Default::default(),
            },
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite_render"))]
            bevy_gizmos::config::GizmoMeshConfig {
                line_perspective: gizmo.line_config.perspective,
                line_style: gizmo.line_config.style,
                line_joints: gizmo.line_config.joints,
                render_layers: render_layers.cloned().unwrap_or_default(),
                handle: gizmo.handle.clone(),
            },
            MainEntity::from(entity),
            TemporaryRenderEntity,
        ));
    }
    *previous_len = values.len();
    commands.spawn_batch(values);
}

pub(crate) fn calculate_bounds(
    mut commands: Commands,
    gizmo_assets: Res<Assets<GizmoAsset>>,
    needs_aabb: Query<
        (Entity, &Gizmo),
        (
            Or<(Changed<Gizmo>, Without<Aabb>)>,
            Without<NoFrustumCulling>,
        ),
    >,
) {
    for (entity, gizmo) in &needs_aabb {
        if let Some(gizmo_asset) = gizmo_assets.get(&gizmo.handle) {
            let aabb_3d = Aabb3d::from_point_cloud(
                Isometry3d::IDENTITY,
                gizmo_asset
                    .list_positions
                    .iter()
                    .chain(gizmo_asset.strip_positions.iter())
                    .filter(|p| p.is_finite())
                    .map(|&p| Vec3A::from(p)),
            );
            let aabb: Aabb = aabb_3d.into();
            commands.entity(entity).insert(aabb);
        }
    }
}

pub(crate) fn mark_gizmos_as_changed_if_their_assets_changed(
    mut gizmos: Query<&mut Gizmo>,
    mut gizmo_asset_events: MessageReader<AssetEvent<GizmoAsset>>,
) {
    let mut changed_gizmos: HashSet<AssetId<GizmoAsset>, FixedHasher> = HashSet::default();
    for mesh_asset_event in gizmo_asset_events.read() {
        if let AssetEvent::Modified { id } = mesh_asset_event {
            changed_gizmos.insert(*id);
        }
    }

    if changed_gizmos.is_empty() {
        return;
    }

    for mut gizmo in &mut gizmos {
        if changed_gizmos.contains(&gizmo.handle.id()) {
            gizmo.set_changed();
        }
    }
}
