//! Resolution of per-camera [`CompositingSpace`] requests.
//!
//! Cameras with matching texture settings that render to the same target
//! share one set of main textures. Cameras that composite over each other in
//! the shared texture form a stack, and later passes need to know how the
//! texture is encoded, so the whole stack has to agree on one compositing
//! space. Each frame this module resolves a compositing space for every
//! camera and stores the result in [`ResolvedCompositingSpace`].

use bevy_camera::{Camera2d, CameraMainTextureUsages, ClearColorConfig, CompositingSpace};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    query::Has,
    schedule::SystemSet,
    system::Query,
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use wgpu::TextureFormat;

use super::{main_texture_key, ExtractedView, MainTextureKey, Msaa};
use crate::camera::ExtractedCamera;

/// A camera view's per-frame resolved compositing space.
///
/// `None` means the view composites in linear light. An explicit
/// [`CompositingSpace::Linear`] request also resolves to `None`, so a request
/// for linear and no request produce the same pipeline keys.
///
/// Read this instead of [`ExtractedCamera::compositing_space`], which holds
/// the camera's raw request and feeds the extract-time choice of main-texture
/// format.
///
/// Spawning or despawning an overlay camera can flip the base view's space
/// and trigger a one-frame respecialization of its 2d pipelines.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedCompositingSpace(pub Option<CompositingSpace>);

impl ResolvedCompositingSpace {
    /// Reads a view's resolved space. An absent component means linear.
    pub fn space(this: Option<&Self>) -> Option<CompositingSpace> {
        this.and_then(|resolved| resolved.0)
    }
}

/// The system set that writes [`ResolvedCompositingSpace`]. It runs in
/// [`RenderSystems::CreateViews`](crate::RenderSystems::CreateViews) after
/// `sort_cameras`. Order `CreateViews` readers of the component after this
/// set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveCompositingSpaces;

/// Whether a camera composites over the previous camera's output and covers
/// the whole render target. This decides membership in a compositing stack.
pub fn composites_fullscreen(camera: &ExtractedCamera) -> bool {
    matches!(camera.clear_color, ClearColorConfig::None) && camera.viewport.is_none()
}

/// Per-view input to [`resolve_spaces`].
struct SpaceInput {
    entity: Entity,
    /// The camera's position in its render target's sorted camera order.
    sorted_index: usize,
    request: Option<CompositingSpace>,
    composites_fullscreen: bool,
    is_camera_2d: bool,
    /// Whether the main-texture format stores signed floats. The format is
    /// part of the texture key, so the value is uniform within a group.
    signed_float_storage: bool,
}

/// A misconfiguration found while resolving compositing spaces.
/// `resolve_composition_spaces` reports each one as a warning.
/// `resolve_spaces` returns them so tests can check the trigger conditions.
#[derive(Debug, PartialEq, Eq)]
enum CompositingSpaceResolutionError {
    /// A compositing stack requests both `Srgb` and `Oklab`.
    ConflictingStackRequests {
        requests: Vec<(Entity, CompositingSpace)>,
    },
    /// Views that share a main texture without forming a stack disagree on a
    /// compositing space.
    MixedSharedTextureRequests {
        requests: Vec<(Entity, Option<CompositingSpace>)>,
    },
    /// A view that isn't a `Camera2d`, or a stack that holds one, requests
    /// `Srgb` or `Oklab`.
    NonCamera2dRequest { non_camera_2d: Vec<Entity> },
    /// A resolved `Oklab` lands on a main texture without signed-float storage.
    OklabWithoutSignedFloatStorage { entities: Vec<Entity> },
}

/// Resolves one compositing space per view. Views that composite over each
/// other must agree on the shared texture's encoding, so a stack resolves as
/// one unit. Members of other shared-texture groups resolve on their own.
fn resolve_spaces(
    views: impl IntoIterator<Item = (MainTextureKey, SpaceInput)>,
) -> (
    EntityHashMap<Option<CompositingSpace>>,
    Vec<CompositingSpaceResolutionError>,
) {
    let mut groups: HashMap<MainTextureKey, Vec<SpaceInput>> = HashMap::default();
    for (texture, mut view) in views {
        view.request = view.request.filter(|space| !space.is_linear());
        groups.entry(texture).or_default().push(view);
    }

    let mut resolved = EntityHashMap::default();
    let mut diagnostics = Vec::new();
    for group in groups.values_mut() {
        group.sort_unstable_by_key(|view| view.sorted_index);
        let is_stack = group.len() >= 2 && group[1..].iter().all(|view| view.composites_fullscreen);
        if is_stack {
            resolve_members(group, &mut resolved, &mut diagnostics);
        } else {
            warn_on_mixed_requests(group, &mut diagnostics);
            // Non-stack members resolve like solo views, each with its own
            // request and its own overrides.
            for member in 0..group.len() {
                resolve_members(&group[member..=member], &mut resolved, &mut diagnostics);
            }
        }
    }
    (resolved, diagnostics)
}

/// Warns when views that share a texture without forming a stack disagree on
/// a compositing space. Blending between their pixels is wrong where their
/// regions meet.
fn warn_on_mixed_requests(
    members: &[SpaceInput],
    diagnostics: &mut Vec<CompositingSpaceResolutionError>,
) {
    if members.len() < 2 {
        return;
    }
    let first = members[0].request;
    let mixed = members[1..].iter().any(|member| member.request != first);
    let any_space = members.iter().any(|member| member.request.is_some());
    if mixed && any_space {
        diagnostics.push(
            CompositingSpaceResolutionError::MixedSharedTextureRequests {
                requests: members
                    .iter()
                    .map(|member| (member.entity, member.request))
                    .collect(),
            },
        );
    }
}

/// Resolves one space for a whole stack, or for a single view of a non-stack
/// group. The single distinct request among the members wins. With no
/// request, or with conflicting requests, the unit resolves to linear.
///
/// Two overrides apply in order. A member that isn't a `Camera2d` forces
/// linear because only 2d render paths encode their output. A resolved
/// `Oklab` degrades to linear when the main texture would clamp its signed
/// channels.
fn resolve_members(
    members: &[SpaceInput],
    resolved: &mut EntityHashMap<Option<CompositingSpace>>,
    diagnostics: &mut Vec<CompositingSpaceResolutionError>,
) {
    let mut has_srgb = false;
    let mut has_oklab = false;
    let mut has_non_camera_2d = false;
    for member in members {
        has_srgb |= member.request == Some(CompositingSpace::Srgb);
        has_oklab |= member.request == Some(CompositingSpace::Oklab);
        has_non_camera_2d |= !member.is_camera_2d;
    }

    let mut space = match (has_srgb, has_oklab) {
        (false, false) => None,
        (true, false) => Some(CompositingSpace::Srgb),
        (false, true) => Some(CompositingSpace::Oklab),
        (true, true) => {
            diagnostics.push(CompositingSpaceResolutionError::ConflictingStackRequests {
                requests: members
                    .iter()
                    .filter_map(|member| member.request.map(|space| (member.entity, space)))
                    .collect(),
            });
            None
        }
    };

    if has_non_camera_2d {
        // Warn only when a request exists to be overridden.
        if has_srgb || has_oklab {
            diagnostics.push(CompositingSpaceResolutionError::NonCamera2dRequest {
                non_camera_2d: members
                    .iter()
                    .filter(|member| !member.is_camera_2d)
                    .map(|member| member.entity)
                    .collect(),
            });
        }
        space = None;
    }

    if space == Some(CompositingSpace::Oklab) && !members[0].signed_float_storage {
        diagnostics.push(
            CompositingSpaceResolutionError::OklabWithoutSignedFloatStorage {
                entities: members.iter().map(|member| member.entity).collect(),
            },
        );
        space = None;
    }

    for member in members {
        resolved.insert(member.entity, space);
    }
}

/// Writes each camera view's [`ResolvedCompositingSpace`]. Runs in
/// [`ResolveCompositingSpaces`].
///
/// `Has<Camera2d>` reads the marker that `bevy_core_pipeline` extracts.
/// Without that plugin every view counts as non-2d.
pub fn resolve_composition_spaces(
    mut views: Query<(
        Entity,
        &ExtractedCamera,
        &ExtractedView,
        &CameraMainTextureUsages,
        &Msaa,
        Has<Camera2d>,
        &mut ResolvedCompositingSpace,
    )>,
) {
    // When every camera requests linear or nothing, all views resolve to
    // linear and no diagnostic can fire, so skip the grouping.
    let any_request = views.iter().any(|(_, camera, ..)| {
        camera
            .compositing_space
            .is_some_and(|space| !space.is_linear())
    });
    if !any_request {
        for (.., mut resolved) in views.iter_mut() {
            *resolved = ResolvedCompositingSpace(None);
        }
        return;
    }

    let inputs: Vec<(MainTextureKey, SpaceInput)> = views
        .iter()
        .map(
            |(entity, camera, view, texture_usage, msaa, is_camera_2d, _)| {
                (
                    main_texture_key(camera, view, texture_usage, *msaa),
                    SpaceInput {
                        entity,
                        sorted_index: camera.sorted_camera_index_for_target,
                        request: camera.compositing_space,
                        composites_fullscreen: composites_fullscreen(camera),
                        is_camera_2d,
                        signed_float_storage: matches!(
                            view.target_format,
                            TextureFormat::Rgba16Float | TextureFormat::Rgba32Float
                        ),
                    },
                )
            },
        )
        .collect();

    let (spaces, diagnostics) = resolve_spaces(inputs);
    // A view missing from the map falls back to linear, the resolver's own
    // default, so no unwrap is needed.
    for (entity, .., mut resolved) in views.iter_mut() {
        *resolved = ResolvedCompositingSpace(spaces.get(&entity).copied().flatten());
    }

    for diagnostic in diagnostics {
        match diagnostic {
            CompositingSpaceResolutionError::ConflictingStackRequests { requests } => warn_once!(
                "Cameras stacked on one shared main texture request conflicting compositing \
                spaces: {requests:?}. The stack composites in linear instead. Give every \
                camera in the stack the same CompositingSpace."
            ),
            CompositingSpaceResolutionError::MixedSharedTextureRequests { requests } => warn_once!(
                "Cameras sharing a render target mix compositing-space requests: {requests:?}. \
                Blending between their pixels will be wrong where their regions meet. Use one \
                CompositingSpace for every camera on a shared target."
            ),
            CompositingSpaceResolutionError::NonCamera2dRequest { non_camera_2d } => warn_once!(
                "A CompositingSpace request resolves to linear because the views \
                {non_camera_2d:?} are not Camera2d views and their render paths do not encode \
                into compositing spaces. Remove the CompositingSpace component or use a Camera2d."
            ),
            CompositingSpaceResolutionError::OklabWithoutSignedFloatStorage { entities } => {
                warn_once!(
                    "CompositingSpace::Oklab on views {entities:?} resolves to linear because \
                    the main texture format cannot store the signed Oklab channels. Add the Hdr \
                    component to the camera to get a signed-float main texture."
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::TextureUsages;

    const SRGB: Option<CompositingSpace> = Some(CompositingSpace::Srgb);
    const OKLAB: Option<CompositingSpace> = Some(CompositingSpace::Oklab);
    const LINEAR: Option<CompositingSpace> = Some(CompositingSpace::Linear);

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    /// A `Camera2d` view on signed-float storage. Cases override what they
    /// need. `texture` selects one of two distinct grouping keys.
    fn view(
        raw: u32,
        texture: usize,
        index: usize,
        request: Option<CompositingSpace>,
    ) -> (MainTextureKey, SpaceInput) {
        let msaa = [Msaa::Off, Msaa::Sample4][texture];
        (
            (
                None,
                TextureUsages::RENDER_ATTACHMENT,
                TextureFormat::Rgba16Float,
                msaa,
            ),
            SpaceInput {
                entity: entity(raw),
                sorted_index: index,
                request,
                composites_fullscreen: true,
                is_camera_2d: true,
                signed_float_storage: true,
            },
        )
    }

    fn resolved_for(
        output: &EntityHashMap<Option<CompositingSpace>>,
        raw: u32,
    ) -> Option<CompositingSpace> {
        *output.get(&entity(raw)).expect("view must be resolved")
    }

    fn has_conflict(diagnostics: &[CompositingSpaceResolutionError]) -> bool {
        diagnostics.iter().any(|d| {
            matches!(
                d,
                CompositingSpaceResolutionError::ConflictingStackRequests { .. }
            )
        })
    }

    fn has_mixed(diagnostics: &[CompositingSpaceResolutionError]) -> bool {
        diagnostics.iter().any(|d| {
            matches!(
                d,
                CompositingSpaceResolutionError::MixedSharedTextureRequests { .. }
            )
        })
    }

    fn has_non_camera_2d(diagnostics: &[CompositingSpaceResolutionError]) -> bool {
        diagnostics.iter().any(|d| {
            matches!(
                d,
                CompositingSpaceResolutionError::NonCamera2dRequest { .. }
            )
        })
    }

    fn has_oklab_storage(diagnostics: &[CompositingSpaceResolutionError]) -> bool {
        diagnostics.iter().any(|d| {
            matches!(
                d,
                CompositingSpaceResolutionError::OklabWithoutSignedFloatStorage { .. }
            )
        })
    }

    #[test]
    fn solo_default_camera_keeps_no_request() {
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, None)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn solo_linear_request_normalizes_to_none() {
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, LINEAR)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn stack_of_linear_requests_normalizes_to_none() {
        let (resolved, diagnostics) =
            resolve_spaces([view(1, 0, 0, LINEAR), view(2, 0, 1, LINEAR)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn stack_with_one_distinct_space_resolves_every_member_to_it() {
        let (resolved, diagnostics) = resolve_spaces([
            view(1, 0, 0, None),
            view(2, 0, 1, SRGB),
            view(3, 0, 2, SRGB),
        ]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), SRGB);
        assert_eq!(resolved_for(&resolved, 3), SRGB);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn stack_with_conflicting_spaces_resolves_to_none_and_warns() {
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, SRGB), view(2, 0, 1, OKLAB)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(has_conflict(&diagnostics));
        assert!(!has_mixed(&diagnostics));
    }

    #[test]
    fn viewport_splitscreen_keeps_per_view_requests() {
        let mut base = view(1, 0, 0, SRGB);
        base.1.composites_fullscreen = false;
        let mut pip = view(2, 0, 1, OKLAB);
        pip.1.composites_fullscreen = false;
        let (resolved, diagnostics) = resolve_spaces([base, pip]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), OKLAB);
        assert!(has_mixed(&diagnostics));
        assert!(!has_conflict(&diagnostics));
    }

    #[test]
    fn mixed_request_and_no_request_non_stack_warns() {
        let mut upper = view(2, 0, 1, None);
        upper.1.composites_fullscreen = false;
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, SRGB), upper]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(has_mixed(&diagnostics));
    }

    #[test]
    fn same_request_non_stack_does_not_warn() {
        let mut upper = view(2, 0, 1, SRGB);
        upper.1.composites_fullscreen = false;
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, SRGB), upper]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), SRGB);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn linear_vs_no_request_non_stack_does_not_warn() {
        let mut upper = view(2, 0, 1, None);
        upper.1.composites_fullscreen = false;
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, LINEAR), upper]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn solo_non_camera_2d_srgb_request_resolves_to_none() {
        let mut camera_3d = view(1, 0, 0, SRGB);
        camera_3d.1.is_camera_2d = false;
        let (resolved, diagnostics) = resolve_spaces([camera_3d]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(has_non_camera_2d(&diagnostics));
    }

    #[test]
    fn solo_non_camera_2d_linear_request_resolves_without_warning() {
        let mut camera_3d = view(1, 0, 0, LINEAR);
        camera_3d.1.is_camera_2d = false;
        let (resolved, diagnostics) = resolve_spaces([camera_3d]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn stack_with_non_camera_2d_member_resolves_to_none() {
        let mut base = view(1, 0, 0, None);
        base.1.is_camera_2d = false;
        base.1.composites_fullscreen = false;
        let (resolved, diagnostics) = resolve_spaces([base, view(2, 0, 1, SRGB)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(has_non_camera_2d(&diagnostics));
    }

    #[test]
    fn non_camera_2d_stack_without_requests_does_not_warn() {
        let mut base = view(1, 0, 0, None);
        base.1.is_camera_2d = false;
        base.1.composites_fullscreen = false;
        let (resolved, diagnostics) = resolve_spaces([base, view(2, 0, 1, None)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(diagnostics.is_empty());
    }

    // The non-`Camera2d` member has no request, so it draws no non-2d warning.
    #[test]
    fn camera_2d_member_of_mixed_non_stack_group_keeps_request() {
        let mut camera_2d = view(1, 0, 0, SRGB);
        camera_2d.1.composites_fullscreen = false;
        let mut camera_3d = view(2, 0, 1, None);
        camera_3d.1.composites_fullscreen = false;
        camera_3d.1.is_camera_2d = false;
        let (resolved, diagnostics) = resolve_spaces([camera_2d, camera_3d]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(has_mixed(&diagnostics));
        assert!(!has_non_camera_2d(&diagnostics));
    }

    #[test]
    fn oklab_without_signed_float_storage_degrades_to_linear() {
        let mut camera = view(1, 0, 0, OKLAB);
        camera.1.signed_float_storage = false;
        let (resolved, diagnostics) = resolve_spaces([camera]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(has_oklab_storage(&diagnostics));
    }

    #[test]
    fn stack_resolved_oklab_degrades_on_unorm_storage() {
        let mut base = view(1, 0, 0, None);
        base.1.signed_float_storage = false;
        let mut overlay = view(2, 0, 1, OKLAB);
        overlay.1.signed_float_storage = false;
        let (resolved, diagnostics) = resolve_spaces([base, overlay]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(has_oklab_storage(&diagnostics));
    }

    // The non-`Camera2d` rule runs before the storage rule, so a request
    // forced to linear never warns twice.
    #[test]
    fn non_camera_2d_oklab_fires_non_2d_warning_not_storage_warning() {
        let mut camera_3d = view(1, 0, 0, OKLAB);
        camera_3d.1.is_camera_2d = false;
        camera_3d.1.signed_float_storage = false;
        let (resolved, diagnostics) = resolve_spaces([camera_3d]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(has_non_camera_2d(&diagnostics));
        assert!(!has_oklab_storage(&diagnostics));
    }

    #[test]
    fn separate_textures_resolve_independently() {
        let (resolved, diagnostics) = resolve_spaces([view(1, 0, 0, SRGB), view(2, 1, 0, OKLAB)]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), OKLAB);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn sorted_index_orders_the_group_not_insertion_order() {
        let mut base = view(1, 0, 0, None);
        base.1.composites_fullscreen = false;
        // Insert the overlay first. The group is still a stack because the
        // clearing member sorts to the front.
        let (resolved, diagnostics) = resolve_spaces([view(2, 0, 1, SRGB), base]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), SRGB);
        assert!(diagnostics.is_empty());
    }
}
