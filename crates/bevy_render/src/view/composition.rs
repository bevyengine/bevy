//! Resolution of per-camera [`CompositingSpace`] requests, and a check on
//! how the cameras in a stack tonemap.
//!
//! Cameras that render to the same target share main textures when their
//! settings match, and composite over each other in that texture. A camera
//! that clears starts a new stack on it, since the clear covers the whole
//! texture. Later passes need to know how the texture is encoded, so each
//! stack has to agree on one compositing space. This module picks that space
//! each frame and stores it in each view's [`ResolvedCompositingSpace`].
//!
//! [`check_stacked_tonemapping`] warns when a camera's tonemapping pass
//! tonemaps pixels that an earlier camera in the stack already tonemapped.

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

use super::{main_texture_key, ExtractedView, MainTextureKey, Msaa, Tonemapping};
use crate::camera::ExtractedCamera;

/// The compositing space a camera view actually uses this frame, written by
/// [`resolve_composition_spaces`].
///
/// `None` means linear. An explicit [`CompositingSpace::Linear`] request also
/// resolves to `None`, so both forms of linear produce the same pipeline
/// keys.
///
/// The value depends on every camera in the stack. Adding or removing a
/// camera on a render target can change it for the cameras already there,
/// which respecializes their 2d pipelines for a frame.
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
/// `sort_cameras`. Readers in `CreateViews` should order after this set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveCompositingSpaces;

/// Whether a main texture format can store the negative values that Oklab
/// channels take. Float and snorm formats can. Unorm formats clamp them.
fn stores_signed_values(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Float
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba16Snorm
    )
}

/// Writes each camera view's [`ResolvedCompositingSpace`]. Runs in
/// [`ResolveCompositingSpaces`].
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
    let any_request = views
        .iter()
        .any(|(.., resolved)| resolved.0.is_some_and(|space| !space.is_linear()));
    if !any_request {
        for (.., mut resolved) in views.iter_mut() {
            *resolved = ResolvedCompositingSpace(None);
        }
        return;
    }

    let inputs: Vec<(MainTextureKey, SpaceInput)> = views
        .iter()
        .map(
            |(entity, camera, view, texture_usage, msaa, is_camera_2d, resolved)| {
                (
                    main_texture_key(camera, view, texture_usage, *msaa),
                    SpaceInput {
                        entity,
                        sorted_index: camera.sorted_camera_index_for_target,
                        // Extraction seeded each component with the camera's
                        // own request.
                        request: resolved.0,
                        loads_previous: matches!(camera.clear_color, ClearColorConfig::None),
                        is_camera_2d,
                        signed_storage: stores_signed_values(view.target_format),
                    },
                )
            },
        )
        .collect();

    let (spaces, diagnostics) = resolve_spaces(inputs);
    // `resolve_spaces` returns an entry for every view it was given. A
    // missing entry reads as linear.
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
            CompositingSpaceResolutionError::FrameStartLoadsOtherStack {
                first,
                first_space,
                last_space,
            } => warn_once!(
                "Camera {first} is the first camera on its render target and uses \
                ClearColorConfig::None, so it loads what the last camera stack left in the \
                main texture the previous frame. That stack composites in {last_space:?} and \
                this camera's stack in {first_space:?}. Give both stacks the same \
                CompositingSpace."
            ),
            CompositingSpaceResolutionError::NonCamera2dRequest { non_camera_2d } => warn_once!(
                "A CompositingSpace request resolves to linear because the views \
                {non_camera_2d:?} are not Camera2d views and their render paths do not encode \
                into compositing spaces. Remove the CompositingSpace component or use a Camera2d."
            ),
            CompositingSpaceResolutionError::OklabWithoutSignedStorage { entities } => {
                warn_once!(
                    "CompositingSpace::Oklab on views {entities:?} resolves to linear because \
                    the main texture format cannot store the signed Oklab channels. Add the Hdr \
                    component to the camera to get a main texture format that can."
                );
            }
        }
    }
}

/// Per-view input to [`resolve_spaces`].
struct SpaceInput {
    entity: Entity,
    /// The camera's position in its render target's sorted camera order.
    sorted_index: usize,
    request: Option<CompositingSpace>,
    /// Whether the view's main pass loads the previous camera's output,
    /// true for [`ClearColorConfig::None`]. A clear covers the whole texture,
    /// viewport or not, so a view that clears starts a new stack.
    loads_previous: bool,
    is_camera_2d: bool,
    /// Whether the main texture format stores signed values, see
    /// [`stores_signed_values`]. The format is part of the texture key, so
    /// this is the same for every view in a group.
    signed_storage: bool,
}

/// A view's place in its main texture's camera order, for
/// [`group_by_texture`] and [`stacks`].
trait StackMember {
    fn sorted_index(&self) -> usize;
    /// True for [`ClearColorConfig::None`]. A view that clears starts a new
    /// stack.
    fn loads_previous(&self) -> bool;
}

impl StackMember for SpaceInput {
    fn sorted_index(&self) -> usize {
        self.sorted_index
    }
    fn loads_previous(&self) -> bool {
        self.loads_previous
    }
}

/// Groups views by main texture and sorts each group by camera order.
///
/// Groups require unique `sorted_index` values and don't handle ties.
fn group_by_texture<T: StackMember>(
    views: impl IntoIterator<Item = (MainTextureKey, T)>,
) -> Vec<Vec<T>> {
    let mut groups: HashMap<MainTextureKey, Vec<T>> = HashMap::default();
    for (texture, view) in views {
        groups.entry(texture).or_default().push(view);
    }
    let mut groups: Vec<Vec<T>> = groups.into_values().collect();
    for group in &mut groups {
        group.sort_unstable_by_key(StackMember::sorted_index);
        debug_assert!(
            group
                .windows(2)
                .all(|pair| pair[0].sorted_index() != pair[1].sorted_index()),
            "sorted camera indices must be unique within a texture group"
        );
    }
    groups
}

/// Splits one texture's sorted views into stacks. Each clearing view after
/// the first starts a new stack.
fn stacks<T: StackMember>(group: &[T]) -> impl Iterator<Item = &[T]> {
    group.chunk_by(|_, next| next.loads_previous())
}

/// A misconfiguration found while resolving compositing spaces.
/// `resolve_composition_spaces` reports each one as a warning.
/// `resolve_spaces` returns them so tests can check when each fires.
#[derive(Debug, PartialEq, Eq)]
enum CompositingSpaceResolutionError {
    /// A compositing stack requests both `Srgb` and `Oklab`.
    ConflictingStackRequests {
        requests: Vec<(Entity, CompositingSpace)>,
    },
    /// The texture's first view loads what the previous frame's last stack
    /// left, and the two stacks resolve to different spaces.
    FrameStartLoadsOtherStack {
        first: Entity,
        first_space: Option<CompositingSpace>,
        last_space: Option<CompositingSpace>,
    },
    /// A view that isn't a `Camera2d`, or a stack that holds one, requests
    /// `Srgb` or `Oklab`.
    NonCamera2dRequest { non_camera_2d: Vec<Entity> },
    /// A resolved `Oklab` lands on a main texture without signed storage.
    OklabWithoutSignedStorage { entities: Vec<Entity> },
}

/// Resolves one compositing space per view. Views that share a main texture
/// split into stacks at every clear, and each stack resolves on its own.
fn resolve_spaces(
    views: impl IntoIterator<Item = (MainTextureKey, SpaceInput)>,
) -> (
    EntityHashMap<Option<CompositingSpace>>,
    Vec<CompositingSpaceResolutionError>,
) {
    let views = views.into_iter().map(|(texture, mut view)| {
        view.request = view.request.filter(|space| !space.is_linear());
        (texture, view)
    });

    let mut resolved = EntityHashMap::default();
    let mut diagnostics = Vec::new();
    for group in group_by_texture(views) {
        let stacks: Vec<&[SpaceInput]> = stacks(&group).collect();
        for stack in &stacks {
            resolve_members(stack, &mut resolved, &mut diagnostics);
        }

        // The main texture persists across frames. A first member that loads
        // blends over what the last stack left the previous frame, so the
        // two stacks must agree.
        let first = &group[0];
        if first.loads_previous && stacks.len() > 1 {
            let space_of = |entity| resolved.get(&entity).copied().flatten();
            let first_space = space_of(first.entity);
            let last_space = space_of(group[group.len() - 1].entity);
            if first_space != last_space {
                diagnostics.push(CompositingSpaceResolutionError::FrameStartLoadsOtherStack {
                    first: first.entity,
                    first_space,
                    last_space,
                });
            }
        }
    }
    (resolved, diagnostics)
}

/// Resolves the compositing space for a list of views.
///
/// It reports a diagnostic and falls back to linear in the following cases:
/// * The views request both [`Srgb`](CompositingSpace::Srgb) and
///   [`Oklab`](CompositingSpace::Oklab).
/// * A view that is not a [`Camera2d`] is in the list and any view requests a
///   non-linear compositing space. 3d render paths write linear values.
/// * The views request [`Oklab`](CompositingSpace::Oklab) on a texture format
///   without signed storage, as Oklab requires negative numbers.
///
/// Otherwise the requested compositing space is chosen. With no request the
/// views resolve to linear.
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

    if space == Some(CompositingSpace::Oklab) && !members[0].signed_storage {
        diagnostics.push(CompositingSpaceResolutionError::OklabWithoutSignedStorage {
            entities: members.iter().map(|member| member.entity).collect(),
        });
        space = None;
    }

    for member in members {
        resolved.insert(member.entity, space);
    }
}

/// Warns when a camera's tonemapping pass tonemaps pixels that another camera
/// on the same main texture already tonemapped.
pub fn check_stacked_tonemapping(
    views: Query<(
        &ExtractedCamera,
        &ExtractedView,
        &CameraMainTextureUsages,
        &Msaa,
        Option<&Tonemapping>,
    )>,
) {
    let flags = || {
        views
            .iter()
            .map(|(camera, view, texture_usage, msaa, tonemapping)| {
                (
                    main_texture_key(camera, view, texture_usage, *msaa),
                    applies_tone_curve(camera, tonemapping),
                    matches!(camera.clear_color, ClearColorConfig::None),
                )
            })
    };
    if !may_tonemap_twice(flags) {
        return;
    }

    let inputs = views
        .iter()
        .map(|(camera, view, texture_usage, msaa, tonemapping)| {
            (
                main_texture_key(camera, view, texture_usage, *msaa),
                tonemap_input(camera, view, tonemapping),
            )
        });
    for error in check_tonemapping(inputs) {
        match error {
            StackedTonemappingError::TonemappedTwice { earlier, later } => warn_once!(
                "Camera {later} renders later than cameras {earlier:?} to the same render \
                target, and its tonemapping pass tonemaps their pixels a second time. Give \
                cameras {earlier:?} Tonemapping::Linear so that only camera {later} applies a \
                tone curve."
            ),
            StackedTonemappingError::FrameStartLoadsTonemappedOutput { first } => warn_once!(
                "Camera {first} uses ClearColorConfig::None and no camera writes its main \
                texture before it, so it loads the previous frame's tonemapped image. A \
                tonemapping pass on this render target then tonemaps that image again every \
                frame. Give camera {first} a clear color, or give every camera on this render \
                target Tonemapping::Linear."
            ),
        }
    }
}

/// Per-view input to [`check_tonemapping`].
struct TonemapInput {
    /// The camera's main world entity, named in warnings.
    main_entity: Entity,
    sorted_index: usize,
    loads_previous: bool,
    /// Whether the view's tonemapping pass runs and applies a tone curve.
    applies_tone_curve: bool,
    has_viewport: bool,
}

impl StackMember for TonemapInput {
    fn sorted_index(&self) -> usize {
        self.sorted_index
    }
    fn loads_previous(&self) -> bool {
        self.loads_previous
    }
}

fn tonemap_input(
    camera: &ExtractedCamera,
    view: &ExtractedView,
    tonemapping: Option<&Tonemapping>,
) -> TonemapInput {
    TonemapInput {
        main_entity: view.retained_view_entity.main_entity.id(),
        sorted_index: camera.sorted_camera_index_for_target,
        loads_previous: matches!(camera.clear_color, ClearColorConfig::None),
        applies_tone_curve: applies_tone_curve(camera, tonemapping),
        has_viewport: camera.viewport.is_some(),
    }
}

/// Whether the view's tonemapping pass runs and applies a tone curve.
fn applies_tone_curve(camera: &ExtractedCamera, tonemapping: Option<&Tonemapping>) -> bool {
    camera.runs_tonemapping_pass && tonemapping.is_some_and(Tonemapping::applies_tone_curve)
}

/// A misconfiguration found by [`check_tonemapping`]. The entities are main
/// world entities.
#[derive(Debug, PartialEq, Eq)]
enum StackedTonemappingError {
    /// The pass of `later` tonemaps pixels that the `earlier` views already
    /// tonemapped.
    TonemappedTwice { earlier: Vec<Entity>, later: Entity },
    /// The texture's first view loads the previous frame's output, which the
    /// last stack tonemapped, and the first stack tonemaps it again.
    FrameStartLoadsTonemappedOutput { first: Entity },
}

/// Whether [`check_tonemapping`] could report anything. Each item is a view's
/// main texture key, whether it applies a tone curve, and whether it loads
/// the previous output. Pixels are tonemapped twice within a frame only when
/// two cameras apply a tone curve. With one such camera, they are tonemapped
/// again across frames only when no camera on its main texture clears.
fn may_tonemap_twice<I: Iterator<Item = (MainTextureKey, bool, bool)>>(
    views: impl Fn() -> I,
) -> bool {
    let mut curves = 0;
    let mut texture = None;
    for (key, applies_tone_curve, _) in views() {
        if applies_tone_curve {
            curves += 1;
            texture = Some(key);
        }
    }
    let Some(texture) = texture else {
        return false;
    };
    curves > 1 || views().all(|(key, _, loads_previous)| key != texture || loads_previous)
}

/// Finds pixels that reach the render target tonemapped more than once.
fn check_tonemapping(
    views: impl IntoIterator<Item = (MainTextureKey, TonemapInput)>,
) -> Vec<StackedTonemappingError> {
    let mut errors = Vec::new();
    for group in group_by_texture(views) {
        let stacks: Vec<&[TonemapInput]> = stacks(&group).collect();
        errors.extend(stacks.iter().filter_map(|stack| check_stack(stack)));

        let first = &group[0];
        let curve = |stack: &[TonemapInput]| stack.iter().any(|view| view.applies_tone_curve);
        if first.loads_previous && curve(stacks[0]) && curve(stacks[stacks.len() - 1]) {
            errors.push(StackedTonemappingError::FrameStartLoadsTonemappedOutput {
                first: first.main_entity,
            });
        }
    }
    errors
}

/// Reports the latest view in the stack without a viewport whose pass
/// tonemaps earlier views' pixels again, so `Tonemapping::Linear` on those
/// earlier views leaves exactly one tone curve. A later view with a viewport
/// isn't reported, because that fix would leave the earlier pixels outside
/// its viewport without a tone curve.
fn check_stack(stack: &[TonemapInput]) -> Option<StackedTonemappingError> {
    let later = stack
        .iter()
        .rposition(|view| view.applies_tone_curve && !view.has_viewport)?;
    let earlier: Vec<Entity> = stack[..later]
        .iter()
        .filter(|view| view.applies_tone_curve)
        .map(|view| view.main_entity)
        .collect();
    (!earlier.is_empty()).then(|| StackedTonemappingError::TonemappedTwice {
        earlier,
        later: stack[later].main_entity,
    })
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

    /// A `Camera2d` view that loads the previous output, on a main texture
    /// that stores signed values.
    /// Tests override the fields they care about. `texture` picks which of
    /// two texture keys the view groups under.
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
                loads_previous: true,
                is_camera_2d: true,
                signed_storage: true,
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

    fn has_frame_start(diagnostics: &[CompositingSpaceResolutionError]) -> bool {
        diagnostics.iter().any(|d| {
            matches!(
                d,
                CompositingSpaceResolutionError::FrameStartLoadsOtherStack { .. }
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
                CompositingSpaceResolutionError::OklabWithoutSignedStorage { .. }
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
    }

    /// Turns a view into one that clears instead of loading the previous
    /// output.
    fn clearing(mut view: (MainTextureKey, SpaceInput)) -> (MainTextureKey, SpaceInput) {
        view.1.loads_previous = false;
        view
    }

    // A clear in the middle starts a second stack, and each stack resolves
    // on its own.
    #[test]
    fn clear_starts_a_second_stack() {
        let (resolved, diagnostics) = resolve_spaces([
            clearing(view(1, 0, 0, SRGB)),
            view(2, 0, 1, None),
            clearing(view(3, 0, 2, OKLAB)),
            view(4, 0, 3, None),
        ]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), SRGB);
        assert_eq!(resolved_for(&resolved, 3), OKLAB);
        assert_eq!(resolved_for(&resolved, 4), OKLAB);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn three_stacks_resolve_independently() {
        let (resolved, diagnostics) = resolve_spaces([
            clearing(view(1, 0, 0, SRGB)),
            clearing(view(2, 0, 1, None)),
            clearing(view(3, 0, 2, OKLAB)),
            view(4, 0, 3, None),
        ]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert_eq!(resolved_for(&resolved, 3), OKLAB);
        assert_eq!(resolved_for(&resolved, 4), OKLAB);
        assert!(diagnostics.is_empty());
    }

    // A conflict in one stack does not change the other stack on the same
    // texture.
    #[test]
    fn stack_conflict_stays_within_its_stack() {
        let (resolved, diagnostics) = resolve_spaces([
            clearing(view(1, 0, 0, SRGB)),
            view(2, 0, 1, OKLAB),
            clearing(view(3, 0, 2, OKLAB)),
            view(4, 0, 3, None),
        ]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert_eq!(resolved_for(&resolved, 3), OKLAB);
        assert_eq!(resolved_for(&resolved, 4), OKLAB);
        assert!(has_conflict(&diagnostics));
    }

    // Two stacks never blend in the main texture, so different requests in
    // different stacks produce no diagnostic.
    #[test]
    fn clearing_camera_keeps_its_own_request() {
        let (resolved, diagnostics) = resolve_spaces([
            clearing(view(1, 0, 0, SRGB)),
            clearing(view(2, 0, 1, OKLAB)),
        ]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), OKLAB);
        assert!(diagnostics.is_empty());
    }

    // A non-`Camera2d` view forces only its own stack to linear.
    #[test]
    fn non_camera_2d_in_a_later_stack_forces_only_that_stack() {
        let mut camera_3d = clearing(view(2, 0, 1, SRGB));
        camera_3d.1.is_camera_2d = false;
        let (resolved, diagnostics) = resolve_spaces([
            clearing(view(1, 0, 0, SRGB)),
            camera_3d,
            view(3, 0, 2, None),
        ]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert_eq!(resolved_for(&resolved, 3), None);
        assert!(has_non_camera_2d(&diagnostics));
    }

    // The texture persists across frames. A first member that loads blends
    // over what the last stack left, so the two stacks must agree.
    #[test]
    fn frame_start_load_warns_when_the_last_stack_differs() {
        let (resolved, diagnostics) = resolve_spaces([
            view(1, 0, 0, SRGB),
            clearing(view(2, 0, 1, OKLAB)),
            view(3, 0, 2, None),
        ]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), OKLAB);
        assert_eq!(resolved_for(&resolved, 3), OKLAB);
        assert!(has_frame_start(&diagnostics));
    }

    #[test]
    fn frame_start_load_with_a_matching_last_stack_is_silent() {
        let (_, diagnostics) = resolve_spaces([
            view(1, 0, 0, SRGB),
            clearing(view(2, 0, 1, SRGB)),
            view(3, 0, 2, None),
        ]);
        assert!(diagnostics.is_empty());
    }

    // A first member that clears loads nothing from the previous frame.
    #[test]
    fn clearing_first_member_never_warns_about_the_last_stack() {
        let (_, diagnostics) = resolve_spaces([
            clearing(view(1, 0, 0, SRGB)),
            clearing(view(2, 0, 1, OKLAB)),
        ]);
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
        base.1.loads_previous = false;
        let (resolved, diagnostics) = resolve_spaces([base, view(2, 0, 1, SRGB)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(has_non_camera_2d(&diagnostics));
    }

    #[test]
    fn non_camera_2d_stack_without_requests_does_not_warn() {
        let mut base = view(1, 0, 0, None);
        base.1.is_camera_2d = false;
        base.1.loads_previous = false;
        let (resolved, diagnostics) = resolve_spaces([base, view(2, 0, 1, None)]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(diagnostics.is_empty());
    }

    // The non-`Camera2d` view starts its own stack, so the `Camera2d` keeps
    // its request and there is no non-2d warning.
    #[test]
    fn camera_2d_below_a_clearing_camera_3d_keeps_its_request() {
        let mut camera_3d = clearing(view(2, 0, 1, None));
        camera_3d.1.is_camera_2d = false;
        let (resolved, diagnostics) = resolve_spaces([clearing(view(1, 0, 0, SRGB)), camera_3d]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), None);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn signed_storage_accepts_float_and_snorm_formats() {
        for format in [
            TextureFormat::Rgba16Float,
            TextureFormat::Rgba32Float,
            TextureFormat::Rgba8Snorm,
            TextureFormat::Rgba16Snorm,
        ] {
            assert!(stores_signed_values(format), "{format:?}");
        }
        for format in [
            TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Bgra8UnormSrgb,
            TextureFormat::Rgb10a2Unorm,
        ] {
            assert!(!stores_signed_values(format), "{format:?}");
        }
    }

    #[test]
    fn oklab_without_signed_storage_degrades_to_linear() {
        let mut camera = view(1, 0, 0, OKLAB);
        camera.1.signed_storage = false;
        let (resolved, diagnostics) = resolve_spaces([camera]);
        assert_eq!(resolved_for(&resolved, 1), None);
        assert!(has_oklab_storage(&diagnostics));
    }

    #[test]
    fn stack_resolved_oklab_degrades_on_unorm_storage() {
        let mut base = view(1, 0, 0, None);
        base.1.signed_storage = false;
        let mut overlay = view(2, 0, 1, OKLAB);
        overlay.1.signed_storage = false;
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
        camera_3d.1.signed_storage = false;
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
        base.1.loads_previous = false;
        // The overlay comes first in the input. Sorting by sorted_index puts
        // the clearing camera back at the front, so the two views form one
        // stack.
        let (resolved, diagnostics) = resolve_spaces([view(2, 0, 1, SRGB), base]);
        assert_eq!(resolved_for(&resolved, 1), SRGB);
        assert_eq!(resolved_for(&resolved, 2), SRGB);
        assert!(diagnostics.is_empty());
    }

    // Tonemapping checks. `curve` is a view whose pass applies a tone curve,
    // and `no_curve` is one that doesn't. Both load the previous output and
    // have no viewport.
    fn member(raw: u32, index: usize, applies_tone_curve: bool) -> (MainTextureKey, TonemapInput) {
        let (texture, _) = view(raw, 0, index, None);
        (
            texture,
            TonemapInput {
                main_entity: entity(raw),
                sorted_index: index,
                loads_previous: true,
                applies_tone_curve,
                has_viewport: false,
            },
        )
    }

    fn curve(raw: u32, index: usize) -> (MainTextureKey, TonemapInput) {
        member(raw, index, true)
    }

    fn no_curve(raw: u32, index: usize) -> (MainTextureKey, TonemapInput) {
        member(raw, index, false)
    }

    fn clears(mut view: (MainTextureKey, TonemapInput)) -> (MainTextureKey, TonemapInput) {
        view.1.loads_previous = false;
        view
    }

    fn with_viewport(mut view: (MainTextureKey, TonemapInput)) -> (MainTextureKey, TonemapInput) {
        view.1.has_viewport = true;
        view
    }

    fn twice(earlier: &[u32], later: u32) -> StackedTonemappingError {
        StackedTonemappingError::TonemappedTwice {
            earlier: earlier.iter().map(|raw| entity(*raw)).collect(),
            later: entity(later),
        }
    }

    fn frame_start(first: u32) -> StackedTonemappingError {
        StackedTonemappingError::FrameStartLoadsTonemappedOutput {
            first: entity(first),
        }
    }

    /// Fast-path input: `texture` picks one of two texture keys.
    fn flags(
        texture: usize,
        applies_tone_curve: bool,
        loads_previous: bool,
    ) -> (MainTextureKey, bool, bool) {
        (
            view(0, texture, 0, None).0,
            applies_tone_curve,
            loads_previous,
        )
    }

    #[test]
    fn fast_path_runs_the_check_only_when_a_repeat_is_possible() {
        let may =
            |views: &[(MainTextureKey, bool, bool)]| may_tonemap_twice(|| views.iter().cloned());
        assert!(may(&[flags(0, true, true)]));
        assert!(!may(&[flags(0, true, true), flags(0, false, false)]));
        assert!(may(&[flags(0, true, true), flags(1, false, false)]));
        assert!(may(&[flags(0, true, false), flags(1, true, false)]));
        assert!(!may(&[flags(0, false, true), flags(0, false, true)]));
    }

    #[test]
    fn linear_applies_no_tone_curve() {
        assert!(!Tonemapping::Linear.applies_tone_curve());
        assert!(Tonemapping::TonyMcMapface.applies_tone_curve());
    }

    #[test]
    fn single_tonemapping_camera_is_silent() {
        assert!(check_tonemapping([clears(curve(1, 0))]).is_empty());
    }

    #[test]
    fn later_tonemapping_camera_tonemaps_the_earlier_one_twice() {
        let errors = check_tonemapping([clears(curve(1, 0)), curve(2, 1)]);
        assert_eq!(errors, vec![twice(&[1], 2)]);
    }

    // One error per stack, naming the latest camera with a tone curve, so
    // the fix leaves exactly one.
    #[test]
    fn three_tonemapping_cameras_report_the_last_one_once() {
        let errors = check_tonemapping([clears(curve(1, 0)), curve(2, 1), curve(3, 2)]);
        assert_eq!(errors, vec![twice(&[1, 2], 3)]);
    }

    #[test]
    fn cameras_without_a_tone_curve_are_never_flagged() {
        assert!(check_tonemapping([clears(curve(1, 0)), no_curve(2, 1)]).is_empty());
        assert!(check_tonemapping([clears(no_curve(1, 0)), curve(2, 1)]).is_empty());
    }

    #[test]
    fn tonemapping_cameras_around_a_camera_without_a_curve_are_flagged() {
        let errors = check_tonemapping([clears(curve(1, 0)), no_curve(2, 1), curve(3, 2)]);
        assert_eq!(errors, vec![twice(&[1], 3)]);
    }

    #[test]
    fn two_stacks_report_one_error_each() {
        let errors = check_tonemapping([
            clears(curve(1, 0)),
            curve(2, 1),
            clears(curve(3, 2)),
            curve(4, 3),
        ]);
        assert_eq!(errors, vec![twice(&[1], 2), twice(&[3], 4)]);
    }

    #[test]
    fn separate_textures_are_silent() {
        let mut other = clears(curve(2, 0));
        other.0 .3 = Msaa::Sample4;
        assert!(check_tonemapping([clears(curve(1, 0)), other]).is_empty());
    }

    #[test]
    fn viewport_camera_over_a_fullscreen_camera_is_silent() {
        let errors = check_tonemapping([clears(curve(1, 0)), with_viewport(curve(2, 1))]);
        assert!(errors.is_empty());
    }

    #[test]
    fn fullscreen_camera_over_viewport_cameras_is_flagged() {
        let errors = check_tonemapping([
            with_viewport(clears(curve(1, 0))),
            with_viewport(curve(2, 1)),
            curve(3, 2),
        ]);
        assert_eq!(errors, vec![twice(&[1, 2], 3)]);
    }

    #[test]
    fn frame_start_load_of_own_tonemapped_output_is_flagged() {
        assert_eq!(check_tonemapping([curve(1, 0)]), vec![frame_start(1)]);
    }

    #[test]
    fn frame_start_load_is_silent_when_the_last_stack_has_no_tone_curve() {
        assert!(check_tonemapping([curve(1, 0), clears(no_curve(2, 1))]).is_empty());
    }

    #[test]
    fn frame_start_load_is_silent_when_the_first_stack_has_no_tone_curve() {
        assert!(check_tonemapping([no_curve(1, 0), clears(curve(2, 1))]).is_empty());
    }

    // Two stacks don't tonemap each other's pixels within a frame, but the
    // first loads the second's output across frames.
    #[test]
    fn frame_start_load_across_stacks_is_flagged_once() {
        let errors = check_tonemapping([curve(1, 0), clears(curve(2, 1))]);
        assert_eq!(errors, vec![frame_start(1)]);
    }

    #[test]
    fn frame_start_load_and_repeat_within_the_frame_are_both_flagged() {
        let errors = check_tonemapping([curve(1, 0), curve(2, 1)]);
        assert_eq!(errors, vec![twice(&[1], 2), frame_start(1)]);
    }
}
