//! Resolves camera stacks, the cameras that draw into one main texture.
//!
//! Cameras on the same render target share a main texture when their
//! settings match. Each camera draws over what the cameras below it left. A
//! camera that clears starts a new stack, because the clear covers the whole
//! texture even with a viewport.
//!
//! Some choices belong to the whole stack. Its cameras must agree on a
//! [`CompositingSpace`], since they encode into one texture. And when more
//! than one of them tonemaps, the tonemapping pass has to run once over the
//! finished texture, or the lower cameras' output is tonemapped again. Each
//! frame [`resolve_camera_stacks`] makes both choices and writes them to
//! every view as [`ResolvedCompositingSpace`] and [`ViewStackContract`].

use bevy_camera::{
    Camera2d, CameraMainTextureUsages, CameraOutputMode, ClearColorConfig, CompositingSpace,
};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    query::Has,
    schedule::SystemSet,
    system::{Commands, Query},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use wgpu::TextureFormat;

use super::{main_texture_key, ExtractedView, MainTextureKey, Msaa, Tonemapping};
use crate::camera::ExtractedCamera;

/// The compositing space a camera view uses this frame. It can differ from
/// the camera's own [`CompositingSpace`], because every camera in a stack
/// uses the same space. Written by [`resolve_camera_stacks`].
///
/// `None` means linear. A [`CompositingSpace::Linear`] request also resolves
/// to `None`, so both give the same pipeline keys.
///
/// Adding or removing a camera on a render target can change the value for
/// the other cameras there, which respecializes their pipelines.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedCompositingSpace(pub Option<CompositingSpace>);

impl ResolvedCompositingSpace {
    /// Returns a view's resolved space. A missing component means linear.
    pub fn space(this: Option<&Self>) -> Option<CompositingSpace> {
        this.and_then(|resolved| resolved.0)
    }
}

/// The system set that resolves camera stacks. It runs in
/// [`RenderSystems::CreateViews`](crate::RenderSystems::CreateViews) after
/// `sort_cameras`. Readers of [`ResolvedCompositingSpace`] in `CreateViews`
/// must run after this set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveCompositingSpaces;

/// A view's role in a pass that can run once for its whole camera stack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackRole {
    /// The view runs its own pass.
    Solo,
    /// The view skips the pass, because its stack's finalizer covers it.
    HandledByFinalizer,
    /// The view runs the pass once for the whole stack.
    Finalizer {
        /// The view whose tonemapping settings the pass uses. It is the
        /// stack's first tonemapping camera, because its scene fills the
        /// texture. It's always another member of this stack, and its
        /// tonemapping is enabled.
        settings: Entity,
    },
}

/// What a view's upscaling blit does. A blend state set on the camera
/// overrides `Replace` and `Blend`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackBlit {
    /// The blit replaces the out texture, because no camera below this one
    /// on the render target has written it.
    Replace,
    /// The blit alpha blends over what the cameras below wrote.
    Blend,
    /// The view is below its stack's finalizer, so its output isn't
    /// tonemapped yet. The finalizer's blit writes it instead.
    Skip,
}

/// What a view does in its camera stack's tonemapping pass and upscaling
/// blit. Written each frame by [`resolve_camera_stacks`].
///
/// The component is never removed, so a view that loses its `ViewTarget`
/// keeps a stale contract. Queries that read it must also require
/// `ViewTarget`.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ViewStackContract {
    /// The view's role in the tonemapping pass.
    pub tonemap: StackRole,
    /// The view's upscaling blit.
    pub blit: StackBlit,
}

/// Whether a camera's tonemapping pass runs. It matches the check in
/// `prepare_view_tonemapping_pipelines`, so a stack never counts on a pass
/// that doesn't run.
fn tonemap_pass_runs(camera: &ExtractedCamera, tonemapping: &Tonemapping) -> bool {
    tonemapping.is_enabled() && !camera.tonemap_in_shader
}

/// Whether a main texture format can store negative values, which Oklab
/// channels need. Unorm formats clamp them.
fn stores_signed_values(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Float
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba16Snorm
    )
}

/// Writes each camera view's [`ResolvedCompositingSpace`] and
/// [`ViewStackContract`]. Runs in [`ResolveCompositingSpaces`].
pub fn resolve_camera_stacks(
    mut commands: Commands,
    mut views: Query<(
        Entity,
        &ExtractedCamera,
        &ExtractedView,
        &CameraMainTextureUsages,
        &Msaa,
        Has<Camera2d>,
        Option<&Tonemapping>,
        &mut ResolvedCompositingSpace,
    )>,
) {
    let mut textures: HashMap<MainTextureKey, Vec<Member>> = HashMap::default();
    for (entity, camera, view, texture_usage, msaa, is_camera_2d, tonemapping, resolved) in
        views.iter()
    {
        let tonemapping = tonemapping.copied().unwrap_or(Tonemapping::None);
        textures
            .entry(main_texture_key(camera, view, texture_usage, *msaa))
            .or_default()
            .push(Member {
                entity,
                sorted_index: camera.sorted_camera_index_for_target,
                loads_previous: matches!(camera.clear_color, ClearColorConfig::None),
                fullscreen: camera.viewport.is_none(),
                output_writes: !matches!(camera.output_mode, CameraOutputMode::Skip),
                pass_runs: tonemap_pass_runs(camera, &tonemapping),
                method: tonemapping,
                // Extraction seeds this with the camera's own request.
                request: resolved.0,
                is_camera_2d,
                signed_storage: stores_signed_values(view.target_format),
            });
    }

    let mut spaces = EntityHashMap::default();
    let mut contracts = EntityHashMap::default();
    let mut errors = Vec::new();
    for members in textures.into_values() {
        resolve_texture(
            &split_stacks(members),
            &mut spaces,
            &mut contracts,
            &mut errors,
        );
    }

    // A view in no stack resolves to linear.
    for (entity, .., mut resolved) in views.iter_mut() {
        *resolved = ResolvedCompositingSpace(spaces.get(&entity).copied().flatten());
    }
    for (entity, contract) in contracts {
        commands.entity(entity).insert_if_neq(contract);
    }
    for error in errors {
        warn(error);
    }
}

/// One camera view's input to the resolution.
struct Member {
    entity: Entity,
    sorted_index: usize,
    /// True for [`ClearColorConfig::None`].
    loads_previous: bool,
    /// True when the camera has no viewport.
    fullscreen: bool,
    /// False for [`CameraOutputMode::Skip`].
    output_writes: bool,
    /// [`tonemap_pass_runs`] for the view.
    pass_runs: bool,
    /// [`Tonemapping::None`] when the view has no `Tonemapping` component.
    method: Tonemapping,
    request: Option<CompositingSpace>,
    is_camera_2d: bool,
    /// [`stores_signed_values`] for the main texture. It's the same for every
    /// member of a stack, since the format is part of the texture key.
    signed_storage: bool,
}

/// A misconfiguration that [`resolve_camera_stacks`] reports as a warning.
/// [`resolve_texture`] returns these so tests can check when each one fires.
#[derive(Debug, PartialEq, Eq)]
enum StackResolutionError {
    /// A stack requests both `Srgb` and `Oklab`.
    ConflictingSpaceRequests {
        requests: Vec<(Entity, CompositingSpace)>,
    },
    /// A stack with a camera that isn't a `Camera2d` requests `Srgb` or
    /// `Oklab`.
    NonCamera2dRequest { non_camera_2d: Vec<Entity> },
    /// A stack resolves to `Oklab` on a main texture that can't store
    /// negative values.
    OklabWithoutSignedStorage { entities: Vec<Entity> },
    /// The texture's first camera draws over the last stack's output from
    /// the previous frame, and the two stacks use different spaces.
    FrameStartLoadsOtherSpace {
        first: Entity,
        first_space: Option<CompositingSpace>,
        last_space: Option<CompositingSpace>,
    },
    /// The texture's first camera draws over the last stack's output from
    /// the previous frame, and that stack tonemaps.
    FrameStartLoadsTonemappedOutput { first: Entity },
    /// A fullscreen member that doesn't clear blits over regions that
    /// viewport members below it already tonemapped.
    FullscreenBlitOverPerCameraPasses { fullscreen_camera: Entity },
    /// A member's tonemapping method differs from the one its stack's pass
    /// applies.
    TonemappingMismatch {
        member: Entity,
        own: Tonemapping,
        applied: Tonemapping,
    },
}

/// Sorts one main texture's members and splits them into stacks at every
/// member that clears.
///
/// Sorted indices must be unique within a texture. They are, because
/// `sort_cameras` counts them per render target and the texture key
/// includes the target.
fn split_stacks(mut members: Vec<Member>) -> Vec<Vec<Member>> {
    members.sort_unstable_by_key(|member| member.sorted_index);
    debug_assert!(
        members
            .windows(2)
            .all(|pair| pair[0].sorted_index != pair[1].sorted_index),
        "sorted camera indices must be unique within a main texture"
    );

    let mut stacks: Vec<Vec<Member>> = Vec::new();
    for member in members {
        match stacks.last_mut() {
            Some(stack) if member.loads_previous => stack.push(member),
            _ => stacks.push(vec![member]),
        }
    }
    stacks
}

/// Resolves the stacks on one main texture, in sorted order.
fn resolve_texture(
    stacks: &[Vec<Member>],
    spaces: &mut EntityHashMap<Option<CompositingSpace>>,
    contracts: &mut EntityHashMap<ViewStackContract>,
    errors: &mut Vec<StackResolutionError>,
) {
    let stack_spaces: Vec<Option<CompositingSpace>> = stacks
        .iter()
        .map(|members| {
            let space = resolve_space(members, errors);
            for member in members {
                spaces.insert(member.entity, space);
            }
            resolve_contracts(members, contracts, errors);
            space
        })
        .collect();

    // The main texture persists across frames. A first member that doesn't
    // clear draws over what the last stack left there, so the two stacks
    // must agree on a space, and the first member is tonemapped again each
    // frame when the last stack tonemaps.
    let (Some(first), Some(last)) = (
        stacks.first().and_then(|stack| stack.first()),
        stacks.last(),
    ) else {
        return;
    };
    if !first.loads_previous {
        return;
    }
    if stack_spaces.first() != stack_spaces.last() {
        errors.push(StackResolutionError::FrameStartLoadsOtherSpace {
            first: first.entity,
            first_space: stack_spaces[0],
            last_space: stack_spaces[stack_spaces.len() - 1],
        });
    }
    if last.iter().any(|member| member.pass_runs) {
        errors.push(StackResolutionError::FrameStartLoadsTonemappedOutput {
            first: first.entity,
        });
    }
}

/// Resolves one stack's compositing space. It falls back to linear, and
/// records an error, when the stack can't use the space it requests.
///
/// A stack with a member that isn't a [`Camera2d`] is always linear, because
/// 3d render paths write linear values.
fn resolve_space(
    members: &[Member],
    errors: &mut Vec<StackResolutionError>,
) -> Option<CompositingSpace> {
    let first = members.first()?;

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
            errors.push(StackResolutionError::ConflictingSpaceRequests {
                requests: members
                    .iter()
                    .filter_map(|member| member.request.map(|space| (member.entity, space)))
                    .filter(|(_, space)| !space.is_linear())
                    .collect(),
            });
            None
        }
    };

    if has_non_camera_2d {
        // Warn only when a request exists to be overridden.
        if has_srgb || has_oklab {
            errors.push(StackResolutionError::NonCamera2dRequest {
                non_camera_2d: members
                    .iter()
                    .filter(|member| !member.is_camera_2d)
                    .map(|member| member.entity)
                    .collect(),
            });
        }
        space = None;
    }

    if space == Some(CompositingSpace::Oklab) && !first.signed_storage {
        errors.push(StackResolutionError::OklabWithoutSignedStorage {
            entities: members.iter().map(|member| member.entity).collect(),
        });
        space = None;
    }

    space
}

/// The members that run a stack's tonemapping pass and supply its settings,
/// as indices into the stack.
#[derive(Clone, Copy)]
struct StackPass {
    finalizer: usize,
    settings: usize,
}

/// Returns the stack's finalizer and settings member, or `None` when each
/// camera runs its own pass.
///
/// A stack gets a finalizer only when at least two members tonemap and every
/// tonemapping member after the first is fullscreen. The finalizer is the
/// last tonemapping member, so the pass runs after every member draws.
fn pass_finalizer(members: &[Member]) -> Option<StackPass> {
    let mut tail = members
        .iter()
        .enumerate()
        .filter(|(_, member)| member.pass_runs);
    let (settings, _) = tail.next()?;
    let mut finalizer = None;
    for (index, member) in tail {
        // Every member after the first draws over the texture without
        // clearing, so a fullscreen one covers all of it.
        if !member.fullscreen {
            return None;
        }
        finalizer = Some(index);
    }
    let finalizer = finalizer?;

    // A `CameraOutputMode::Skip` camera never blits. As the finalizer,
    // nothing would write the stack's output to the target.
    members[finalizer].output_writes.then_some(StackPass {
        finalizer,
        settings,
    })
}

/// Resolves the contracts of one stack of sorted members.
fn resolve_contracts(
    members: &[Member],
    contracts: &mut EntityHashMap<ViewStackContract>,
    errors: &mut Vec<StackResolutionError>,
) {
    let pass = pass_finalizer(members);

    // Without a finalizer each camera tonemaps on its own. A fullscreen
    // member that doesn't clear, above a viewport member, then blits the
    // whole target and writes the viewport's tonemapped pixels a second
    // time. A viewport member above members that tonemap on their own gets
    // no warning, because any check for it would also fire on plain split
    // screen.
    if pass.is_none() {
        let flagged = members.iter().enumerate().find(|(index, candidate)| {
            let below = &members[..*index];
            candidate.loads_previous
                && candidate.fullscreen
                && below
                    .iter()
                    .any(|member| member.loads_previous && !member.fullscreen)
                && below.iter().any(|member| member.pass_runs)
        });
        if let Some((_, flagged)) = flagged {
            errors.push(StackResolutionError::FullscreenBlitOverPerCameraPasses {
                fullscreen_camera: flagged.entity,
            });
        }
    }

    for (index, member) in members.iter().enumerate() {
        // Nothing below this camera has written the out texture when every
        // lower sorted index on the target is a skipped blit in this stack.
        let skipped_below = pass.map_or(0, |pass| pass.finalizer.min(index));
        let blit = if member.sorted_index == skipped_below {
            StackBlit::Replace
        } else {
            StackBlit::Blend
        };

        let (tonemap, blit) = match pass {
            Some(StackPass { finalizer, .. }) if index < finalizer => {
                let tonemap = if member.pass_runs {
                    StackRole::HandledByFinalizer
                } else {
                    StackRole::Solo
                };
                (tonemap, StackBlit::Skip)
            }
            Some(StackPass {
                finalizer,
                settings,
            }) if index == finalizer => (
                StackRole::Finalizer {
                    settings: members[settings].entity,
                },
                blit,
            ),
            _ => (StackRole::Solo, blit),
        };

        // The pass applies the settings member's method to every member it
        // covers.
        if let Some(StackPass { settings, .. }) = pass {
            let applied = members[settings].method;
            if tonemap != StackRole::Solo && member.method != applied {
                errors.push(StackResolutionError::TonemappingMismatch {
                    member: member.entity,
                    own: member.method,
                    applied,
                });
            }
        }

        contracts.insert(member.entity, ViewStackContract { tonemap, blit });
    }
}

fn warn(error: StackResolutionError) {
    match error {
        StackResolutionError::ConflictingSpaceRequests { requests } => warn_once!(
            "Cameras in one camera stack request different compositing spaces: \
            {requests:?}. The stack composites in linear instead. Give every camera in \
            the stack the same CompositingSpace."
        ),
        StackResolutionError::NonCamera2dRequest { non_camera_2d } => warn_once!(
            "Only Camera2d supports the Srgb and Oklab compositing spaces. Cameras \
            {non_camera_2d:?} are not Camera2d, so their camera stack composites in \
            linear instead. Remove the CompositingSpace component or use Camera2d."
        ),
        StackResolutionError::OklabWithoutSignedStorage { entities } => warn_once!(
            "Cameras {entities:?} share a camera stack that requests \
            CompositingSpace::Oklab, but their main texture format can't store the \
            negative values Oklab needs. They composite in linear instead. Add Hdr to \
            these cameras."
        ),
        StackResolutionError::FrameStartLoadsOtherSpace {
            first,
            first_space,
            last_space,
        } => warn_once!(
            "Camera {first} draws first into its main texture and uses \
            ClearColorConfig::None, so it draws over what the last camera stack left there \
            last frame. That stack composites in {last_space:?}, but this camera's stack \
            composites in {first_space:?}. Give both stacks the same CompositingSpace."
        ),
        StackResolutionError::FrameStartLoadsTonemappedOutput { first } => warn_once!(
            "Camera {first} draws first into its main texture and uses \
            ClearColorConfig::None, so it draws over last frame's image. The last camera \
            stack on that texture tonemaps the image, so it is tonemapped again every frame \
            and drifts. For trail or feedback effects, use Tonemapping::None on the cameras \
            in that stack."
        ),
        StackResolutionError::FullscreenBlitOverPerCameraPasses { fullscreen_camera } => {
            warn_once!(
                "Camera {fullscreen_camera} uses ClearColorConfig::None without a \
                viewport, and draws over cameras with viewports that each tonemap. Its \
                output covers the whole render target and writes their tonemapped pixels \
                a second time. Give this camera its own render target."
            );
        }
        StackResolutionError::TonemappingMismatch {
            member,
            own,
            applied,
        } => warn_once!(
            "Camera {member} uses {own:?}, but its camera stack is tonemapped once with \
            {applied:?}. The whole stack uses the Tonemapping, ColorGrading, and \
            DebandDither of its first tonemapping camera. Give every camera in the stack \
            the same Tonemapping."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRGB: Option<CompositingSpace> = Some(CompositingSpace::Srgb);
    const OKLAB: Option<CompositingSpace> = Some(CompositingSpace::Oklab);
    const LINEAR: Option<CompositingSpace> = Some(CompositingSpace::Linear);

    const ALPHA: StackBlit = StackBlit::Blend;
    const REPLACE: StackBlit = StackBlit::Replace;
    const SKIP: StackBlit = StackBlit::Skip;

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    fn finalizer(settings: u32) -> StackRole {
        StackRole::Finalizer {
            settings: entity(settings),
        }
    }

    /// An `Hdr` `Camera2d` member that clears its target, tonemaps, and
    /// requests no compositing space, on a main texture that stores signed
    /// values.
    fn clearing(raw: u32, index: usize) -> Member {
        Member {
            entity: entity(raw),
            sorted_index: index,
            loads_previous: false,
            fullscreen: true,
            output_writes: true,
            pass_runs: true,
            method: Tonemapping::TonyMcMapface,
            request: None,
            is_camera_2d: true,
            signed_storage: true,
        }
    }

    /// A fullscreen `ClearColorConfig::None` member.
    fn compositing(raw: u32, index: usize) -> Member {
        let mut member = clearing(raw, index);
        member.loads_previous = true;
        member
    }

    /// A viewport `ClearColorConfig::None` member.
    fn viewport(raw: u32, index: usize) -> Member {
        let mut member = compositing(raw, index);
        member.fullscreen = false;
        member
    }

    /// An SDR member that clears and requests a compositing space.
    fn clear_2d(raw: u32, index: usize, request: Option<CompositingSpace>) -> Member {
        requesting(sdr(clearing(raw, index)), request)
    }

    /// An SDR member that doesn't clear and requests a compositing space.
    fn load_2d(raw: u32, index: usize, request: Option<CompositingSpace>) -> Member {
        requesting(sdr(compositing(raw, index)), request)
    }

    /// Marks a member as `Tonemapping::None`, which keeps its pass off.
    fn disabled(mut member: Member) -> Member {
        member.method = Tonemapping::None;
        member.pass_runs = false;
        member
    }

    /// Marks a member that tonemaps in shader, like an SDR camera on a shared
    /// target. Its pass is off whatever the method.
    fn sdr(mut member: Member) -> Member {
        member.pass_runs = false;
        member
    }

    /// Marks a member as `CameraOutputMode::Skip`.
    fn skipping(mut member: Member) -> Member {
        member.output_writes = false;
        member
    }

    fn with_method(mut member: Member, method: Tonemapping) -> Member {
        member.method = method;
        member
    }

    fn requesting(mut member: Member, request: Option<CompositingSpace>) -> Member {
        member.request = request;
        member
    }

    fn camera_3d(mut member: Member) -> Member {
        member.is_camera_2d = false;
        member
    }

    fn unsigned_storage(mut member: Member) -> Member {
        member.signed_storage = false;
        member
    }

    struct Resolved {
        spaces: EntityHashMap<Option<CompositingSpace>>,
        contracts: EntityHashMap<ViewStackContract>,
        errors: Vec<StackResolutionError>,
    }

    impl Resolved {
        fn space(&self, raw: u32) -> Option<CompositingSpace> {
            *self
                .spaces
                .get(&entity(raw))
                .expect("view must be resolved")
        }

        fn contract(&self, raw: u32) -> ViewStackContract {
            *self
                .contracts
                .get(&entity(raw))
                .expect("view must have a contract")
        }

        fn has_conflict(&self) -> bool {
            self.errors
                .iter()
                .any(|e| matches!(e, StackResolutionError::ConflictingSpaceRequests { .. }))
        }

        fn has_non_camera_2d(&self) -> bool {
            self.errors
                .iter()
                .any(|e| matches!(e, StackResolutionError::NonCamera2dRequest { .. }))
        }

        fn has_oklab_storage(&self) -> bool {
            self.errors
                .iter()
                .any(|e| matches!(e, StackResolutionError::OklabWithoutSignedStorage { .. }))
        }

        fn has_frame_start_space(&self) -> bool {
            self.errors
                .iter()
                .any(|e| matches!(e, StackResolutionError::FrameStartLoadsOtherSpace { .. }))
        }
    }

    /// Resolves the stacks of one main texture, bottom to top.
    fn resolve(stacks: Vec<Vec<Member>>) -> Resolved {
        let mut resolved = Resolved {
            spaces: EntityHashMap::default(),
            contracts: EntityHashMap::default(),
            errors: Vec::new(),
        };
        resolve_texture(
            &stacks,
            &mut resolved.spaces,
            &mut resolved.contracts,
            &mut resolved.errors,
        );
        resolved
    }

    fn entities(stacks: &[Vec<Member>]) -> Vec<Vec<Entity>> {
        stacks
            .iter()
            .map(|stack| stack.iter().map(|member| member.entity).collect())
            .collect()
    }

    // Splitting into stacks.

    #[test]
    fn loading_members_join_the_stack_below() {
        let stacks = split_stacks(vec![clearing(1, 0), compositing(2, 1), compositing(3, 2)]);
        assert_eq!(
            entities(&stacks),
            vec![vec![entity(1), entity(2), entity(3)]]
        );
    }

    #[test]
    fn a_clear_starts_a_new_stack() {
        let stacks = split_stacks(vec![
            clearing(1, 0),
            compositing(2, 1),
            clearing(3, 2),
            compositing(4, 3),
        ]);
        assert_eq!(
            entities(&stacks),
            vec![vec![entity(1), entity(2)], vec![entity(3), entity(4)]]
        );
    }

    #[test]
    fn a_first_member_that_does_not_clear_starts_the_first_stack() {
        let stacks = split_stacks(vec![compositing(1, 0), compositing(2, 1)]);
        assert_eq!(entities(&stacks), vec![vec![entity(1), entity(2)]]);
    }

    #[test]
    fn sorted_index_orders_members_not_insertion_order() {
        let stacks = split_stacks(vec![compositing(2, 1), clearing(1, 0)]);
        assert_eq!(entities(&stacks), vec![vec![entity(1), entity(2)]]);
    }

    // Compositing spaces.

    #[test]
    fn solo_default_camera_keeps_no_request() {
        let resolved = resolve(vec![vec![clear_2d(1, 0, None)]]);
        assert_eq!(resolved.space(1), None);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn linear_requests_resolve_to_none() {
        let resolved = resolve(vec![vec![clear_2d(1, 0, LINEAR), load_2d(2, 1, LINEAR)]]);
        assert_eq!(resolved.space(1), None);
        assert_eq!(resolved.space(2), None);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn stack_with_one_distinct_space_resolves_every_member_to_it() {
        let resolved = resolve(vec![vec![
            clear_2d(1, 0, None),
            load_2d(2, 1, SRGB),
            load_2d(3, 2, SRGB),
        ]]);
        assert_eq!(resolved.space(1), SRGB);
        assert_eq!(resolved.space(2), SRGB);
        assert_eq!(resolved.space(3), SRGB);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn stack_with_conflicting_spaces_resolves_to_none_and_warns() {
        let resolved = resolve(vec![vec![clear_2d(1, 0, SRGB), load_2d(2, 1, OKLAB)]]);
        assert_eq!(resolved.space(1), None);
        assert_eq!(resolved.space(2), None);
        assert!(resolved.has_conflict());
    }

    #[test]
    fn each_stack_resolves_its_own_space() {
        let resolved = resolve(vec![
            vec![clear_2d(1, 0, SRGB), load_2d(2, 1, None)],
            vec![clear_2d(3, 2, OKLAB), load_2d(4, 3, None)],
        ]);
        assert_eq!(resolved.space(1), SRGB);
        assert_eq!(resolved.space(2), SRGB);
        assert_eq!(resolved.space(3), OKLAB);
        assert_eq!(resolved.space(4), OKLAB);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn space_conflict_stays_within_its_stack() {
        let resolved = resolve(vec![
            vec![clear_2d(1, 0, SRGB), load_2d(2, 1, OKLAB)],
            vec![clear_2d(3, 2, OKLAB), load_2d(4, 3, None)],
        ]);
        assert_eq!(resolved.space(1), None);
        assert_eq!(resolved.space(2), None);
        assert_eq!(resolved.space(3), OKLAB);
        assert_eq!(resolved.space(4), OKLAB);
        assert!(resolved.has_conflict());
    }

    #[test]
    fn non_camera_2d_forces_only_its_own_stack_to_linear() {
        let resolved = resolve(vec![
            vec![clear_2d(1, 0, SRGB)],
            vec![camera_3d(clear_2d(2, 1, SRGB)), load_2d(3, 2, None)],
        ]);
        assert_eq!(resolved.space(1), SRGB);
        assert_eq!(resolved.space(2), None);
        assert_eq!(resolved.space(3), None);
        assert!(resolved.has_non_camera_2d());
    }

    #[test]
    fn non_camera_2d_linear_request_resolves_without_warning() {
        let resolved = resolve(vec![vec![camera_3d(clear_2d(1, 0, LINEAR))]]);
        assert_eq!(resolved.space(1), None);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn stack_with_non_camera_2d_member_resolves_to_none() {
        let resolved = resolve(vec![vec![
            camera_3d(clear_2d(1, 0, None)),
            load_2d(2, 1, SRGB),
        ]]);
        assert_eq!(resolved.space(1), None);
        assert_eq!(resolved.space(2), None);
        assert!(resolved.has_non_camera_2d());
    }

    #[test]
    fn non_camera_2d_stack_without_requests_does_not_warn() {
        let resolved = resolve(vec![vec![
            camera_3d(clear_2d(1, 0, None)),
            load_2d(2, 1, None),
        ]]);
        assert_eq!(resolved.space(1), None);
        assert!(resolved.errors.is_empty());
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
    fn oklab_without_signed_storage_resolves_to_linear() {
        let resolved = resolve(vec![vec![
            unsigned_storage(clear_2d(1, 0, None)),
            unsigned_storage(load_2d(2, 1, OKLAB)),
        ]]);
        assert_eq!(resolved.space(1), None);
        assert_eq!(resolved.space(2), None);
        assert!(resolved.has_oklab_storage());
    }

    // The non-`Camera2d` rule runs before the storage rule, so a request
    // forced to linear never warns twice.
    #[test]
    fn non_camera_2d_oklab_warns_once() {
        let resolved = resolve(vec![vec![camera_3d(unsigned_storage(clear_2d(
            1, 0, OKLAB,
        )))]]);
        assert_eq!(resolved.space(1), None);
        assert!(resolved.has_non_camera_2d());
        assert!(!resolved.has_oklab_storage());
    }

    #[test]
    fn frame_start_load_warns_when_the_last_stack_space_differs() {
        let resolved = resolve(vec![
            vec![load_2d(1, 0, SRGB)],
            vec![clear_2d(2, 1, OKLAB), load_2d(3, 2, None)],
        ]);
        assert_eq!(resolved.space(1), SRGB);
        assert_eq!(resolved.space(3), OKLAB);
        assert!(resolved.has_frame_start_space());
    }

    #[test]
    fn frame_start_load_with_a_matching_last_stack_space_is_silent() {
        let resolved = resolve(vec![
            vec![load_2d(1, 0, SRGB)],
            vec![clear_2d(2, 1, SRGB), load_2d(3, 2, None)],
        ]);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn clearing_first_member_never_warns_about_the_last_stack() {
        let resolved = resolve(vec![
            vec![clear_2d(1, 0, SRGB)],
            vec![clear_2d(2, 1, OKLAB)],
        ]);
        assert!(resolved.errors.is_empty());
    }

    // Tonemapping roles and blits.

    #[test]
    fn solo_camera_runs_its_own_pass_and_replaces() {
        let resolved = resolve(vec![vec![clearing(1, 0)]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(1).blit, REPLACE);
        assert!(resolved.errors.is_empty());
    }

    // Sorted index 1 means a camera on another main texture wrote the out
    // texture first.
    #[test]
    fn solo_camera_above_another_texture_blends() {
        let resolved = resolve(vec![vec![clearing(1, 1)]]);
        assert_eq!(resolved.contract(1).blit, ALPHA);
    }

    #[test]
    fn hdr_stack_tonemaps_once_and_skips_lower_blits() {
        let resolved = resolve(vec![vec![clearing(1, 0), compositing(2, 1)]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(2).tonemap, finalizer(1));
        assert_eq!(resolved.contract(1).blit, SKIP);
        assert_eq!(resolved.contract(2).blit, REPLACE);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn sdr_stack_is_solo_and_keeps_its_blits() {
        let resolved = resolve(vec![vec![sdr(clearing(1, 0)), sdr(compositing(2, 1))]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(2).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(1).blit, REPLACE);
        assert_eq!(resolved.contract(2).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn single_tonemapping_member_keeps_per_camera_behavior() {
        let resolved = resolve(vec![vec![clearing(1, 0), disabled(compositing(2, 1))]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(2).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(1).blit, REPLACE);
        assert_eq!(resolved.contract(2).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn three_member_stack_finalizes_on_the_top_member() {
        let resolved = resolve(vec![vec![
            clearing(1, 0),
            compositing(2, 1),
            compositing(3, 2),
        ]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(2).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(3).tonemap, finalizer(1));
    }

    #[test]
    fn finalizer_uses_the_first_tonemapping_members_settings() {
        let resolved = resolve(vec![vec![
            disabled(clearing(1, 0)),
            compositing(2, 1),
            compositing(3, 2),
        ]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(2).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(3).tonemap, finalizer(2));
        assert_eq!(resolved.contract(1).blit, SKIP);
        assert!(resolved.errors.is_empty());
    }

    // The finalizer rule checks every tonemapping member after the first,
    // not just the last.
    #[test]
    fn partially_covering_middle_member_keeps_per_camera_passes() {
        let resolved = resolve(vec![vec![
            clearing(1, 0),
            viewport(2, 1),
            compositing(3, 2),
        ]]);
        for raw in 1..=3 {
            assert_eq!(resolved.contract(raw).tonemap, StackRole::Solo);
        }
    }

    #[test]
    fn viewport_split_screen_keeps_per_camera_passes() {
        let mut left = viewport(1, 0);
        left.loads_previous = false;
        let resolved = resolve(vec![vec![left, viewport(2, 1)]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(2).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(1).blit, REPLACE);
        assert_eq!(resolved.contract(2).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn stacks_with_one_tonemapping_member_keep_per_camera_passes() {
        let resolved = resolve(vec![
            vec![clearing(1, 0)],
            vec![disabled(clearing(2, 1)), compositing(3, 2)],
        ]);
        for raw in 1..=3 {
            assert_eq!(resolved.contract(raw).tonemap, StackRole::Solo);
        }
        assert_eq!(resolved.contract(1).blit, REPLACE);
        assert_eq!(resolved.contract(2).blit, ALPHA);
        assert_eq!(resolved.contract(3).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    // The disabled member is Solo, but the finalizer's blit writes its
    // output, so its own blit is skipped.
    #[test]
    fn disabled_compositing_member_below_finalizer_skips_blit() {
        let resolved = resolve(vec![vec![
            clearing(1, 0),
            disabled(compositing(2, 1)),
            compositing(3, 2),
        ]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(2).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(3).tonemap, finalizer(1));
        assert_eq!(resolved.contract(1).blit, SKIP);
        assert_eq!(resolved.contract(2).blit, SKIP);
        assert_eq!(resolved.contract(3).blit, REPLACE);
        assert!(resolved.errors.is_empty());
    }

    // The member above the finalizer blends over the finalizer's blit.
    #[test]
    fn disabled_member_above_finalizer_keeps_alpha_blit() {
        let resolved = resolve(vec![vec![
            clearing(1, 0),
            compositing(2, 1),
            disabled(compositing(3, 2)),
        ]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(2).tonemap, finalizer(1));
        assert_eq!(resolved.contract(3).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(1).blit, SKIP);
        assert_eq!(resolved.contract(2).blit, REPLACE);
        assert_eq!(resolved.contract(3).blit, ALPHA);
    }

    #[test]
    fn skip_finalizer_keeps_per_camera_passes_and_blits() {
        let resolved = resolve(vec![vec![clearing(1, 0), skipping(compositing(2, 1))]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(2).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(1).blit, REPLACE);
        assert_eq!(resolved.contract(2).blit, ALPHA);
    }

    #[test]
    fn fullscreen_camera_above_viewport_cameras_is_flagged() {
        let resolved = resolve(vec![vec![
            viewport(1, 0),
            viewport(2, 1),
            disabled(compositing(3, 2)),
        ]]);
        for raw in 1..=3 {
            assert_eq!(resolved.contract(raw).tonemap, StackRole::Solo);
        }
        assert!(resolved.errors.contains(
            &StackResolutionError::FullscreenBlitOverPerCameraPasses {
                fullscreen_camera: entity(3)
            }
        ));
    }

    // See `resolve_contracts` for why this doesn't warn.
    #[test]
    fn viewport_above_tonemapping_members_is_silent() {
        let resolved = resolve(vec![vec![clearing(1, 0), viewport(2, 1)]]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(2).tonemap, StackRole::Solo);
        assert!(resolved.errors.is_empty());
    }

    // The finalizer's own method is ignored, so the finalizer is flagged.
    #[test]
    fn tonemapping_mismatch_is_flagged_on_the_member_whose_method_is_ignored() {
        let resolved = resolve(vec![vec![
            with_method(clearing(1, 0), Tonemapping::AcesFitted),
            compositing(2, 1),
        ]]);
        assert_eq!(
            resolved.errors,
            vec![StackResolutionError::TonemappingMismatch {
                member: entity(2),
                own: Tonemapping::TonyMcMapface,
                applied: Tonemapping::AcesFitted,
            }]
        );
    }

    // Each stack has its own settings member.
    #[test]
    fn tonemapping_mismatch_stays_within_its_stack() {
        let resolved = resolve(vec![
            vec![
                with_method(clearing(1, 0), Tonemapping::AcesFitted),
                compositing(2, 1),
            ],
            vec![
                with_method(clearing(3, 2), Tonemapping::AcesFitted),
                with_method(compositing(4, 3), Tonemapping::AcesFitted),
            ],
        ]);
        assert_eq!(
            resolved.errors,
            vec![StackResolutionError::TonemappingMismatch {
                member: entity(2),
                own: Tonemapping::TonyMcMapface,
                applied: Tonemapping::AcesFitted,
            }]
        );
    }

    #[test]
    fn frame_start_load_with_tonemapping_is_flagged() {
        let resolved = resolve(vec![vec![compositing(1, 0)]]);
        assert_eq!(
            resolved.errors,
            vec![StackResolutionError::FrameStartLoadsTonemappedOutput { first: entity(1) }]
        );
    }

    #[test]
    fn frame_start_load_without_passes_is_silent() {
        let resolved = resolve(vec![vec![disabled(compositing(1, 0))]]);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn frame_start_load_is_flagged_when_the_last_stack_tonemaps() {
        let resolved = resolve(vec![
            vec![disabled(compositing(1, 0))],
            vec![clearing(2, 1), compositing(3, 2)],
        ]);
        assert!(resolved
            .errors
            .contains(&StackResolutionError::FrameStartLoadsTonemappedOutput { first: entity(1) }));
    }

    // The last stack clears over the tonemapped output.
    #[test]
    fn frame_start_load_is_silent_when_only_an_earlier_stack_tonemaps() {
        let resolved = resolve(vec![
            vec![disabled(compositing(1, 0))],
            vec![clearing(2, 1), compositing(3, 2)],
            vec![disabled(clearing(4, 3))],
        ]);
        assert!(resolved.errors.is_empty());
    }

    // The second finalizer blends over the first stack's output.
    #[test]
    fn each_stack_tonemaps_once() {
        let resolved = resolve(vec![
            vec![clearing(1, 0), compositing(2, 1)],
            vec![clearing(3, 2), compositing(4, 3)],
        ]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(2).tonemap, finalizer(1));
        assert_eq!(resolved.contract(3).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(4).tonemap, finalizer(3));
        assert_eq!(resolved.contract(1).blit, SKIP);
        assert_eq!(resolved.contract(2).blit, REPLACE);
        assert_eq!(resolved.contract(3).blit, SKIP);
        assert_eq!(resolved.contract(4).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    // A fullscreen finalizer can cover a viewport member that clears.
    #[test]
    fn viewport_clear_starts_a_second_stack() {
        let mut inset = viewport(3, 2);
        inset.loads_previous = false;
        let resolved = resolve(vec![
            vec![clearing(1, 0), compositing(2, 1)],
            vec![inset, compositing(4, 3)],
        ]);
        assert_eq!(resolved.contract(3).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(4).tonemap, finalizer(3));
        assert_eq!(resolved.contract(2).blit, REPLACE);
        assert_eq!(resolved.contract(3).blit, SKIP);
        assert_eq!(resolved.contract(4).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    // Sorted index 1 is a camera on another main texture, so the finalizer
    // blends over it.
    #[test]
    fn finalizer_above_a_camera_on_another_texture_keeps_alpha_blit() {
        let resolved = resolve(vec![vec![clearing(1, 0), compositing(2, 2)]]);
        assert_eq!(resolved.contract(1).blit, SKIP);
        assert_eq!(resolved.contract(2).blit, ALPHA);
        assert!(resolved.errors.is_empty());
    }

    #[test]
    fn stack_starting_above_index_zero_keeps_alpha_blit() {
        let resolved = resolve(vec![vec![clearing(1, 1), compositing(2, 2)]]);
        assert_eq!(resolved.contract(2).tonemap, finalizer(1));
        assert_eq!(resolved.contract(2).blit, ALPHA);
    }

    #[test]
    fn skip_finalizer_in_a_later_stack_leaves_the_first_stack_alone() {
        let resolved = resolve(vec![
            vec![clearing(1, 0), compositing(2, 1)],
            vec![clearing(3, 2), skipping(compositing(4, 3))],
        ]);
        assert_eq!(resolved.contract(1).tonemap, StackRole::HandledByFinalizer);
        assert_eq!(resolved.contract(2).tonemap, finalizer(1));
        assert_eq!(resolved.contract(3).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(4).tonemap, StackRole::Solo);
        assert_eq!(resolved.contract(3).blit, ALPHA);
        assert_eq!(resolved.contract(4).blit, ALPHA);
    }
}
