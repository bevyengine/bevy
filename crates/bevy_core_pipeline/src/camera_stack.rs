//! Decides which camera in a camera stack runs the tonemapping pass, and
//! writes a [`ViewStackContract`] on every camera view that has a
//! [`ViewTarget`], for prepare systems that depend on the stack.
//!
//! Cameras that render to the same target share main textures when their
//! settings match. `prepare_view_targets` allocates them. A camera that
//! clears starts a new stack on the texture, since the clear covers the whole
//! texture, so one texture can have more than one stack in a frame. When an
//! earlier camera runs a fullscreen pass, every later camera in its stack
//! composites on top of pixels that pass already processed. Running the pass
//! per camera would tonemap a lower camera's output a second time. Instead
//! the stack's last tonemapping camera runs the pass once, over the
//! composited buffer.

use bevy_app::{App, Plugin};
use bevy_camera::{CameraOutputMode, ClearColorConfig};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::{Entity, EntityHashMap},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    view::{prepare_view_targets, ViewTarget},
    Render, RenderApp, RenderSystems,
};
use core::hash::Hash;

use crate::tonemapping::Tonemapping;

/// Registers [`resolve_camera_stack_contracts`], which turns each frame's
/// camera stacks into a [`ViewStackContract`] component on every camera
/// view that has a [`ViewTarget`].
pub struct CameraStackPlugin;

impl Plugin for CameraStackPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            resolve_camera_stack_contracts
                .in_set(RenderSystems::PrepareViews)
                .after(prepare_view_targets),
        );
    }
}

/// A view's role for a fullscreen pass that can run once for the whole
/// camera stack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackRole {
    /// The view runs its own pass.
    Solo,
    /// The view doesn't run the pass. The camera named here runs it once
    /// for the whole stack.
    HandledBy(Entity),
    /// The view runs the pass once for the whole stack.
    Finalizer,
}

/// Whether a view's upscaling blit runs, and with which auto-detected blend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlitDisposition {
    /// The view's blit runs.
    Run {
        /// Whether the auto-detected alpha blend becomes a replacing blend.
        /// This is true on a finalizer when every camera below it on the
        /// render target is a member of its stack whose blit is skipped. Its
        /// blit writes the whole stack's output, and no earlier blit has
        /// written the out texture, so it must replace rather than blend.
        /// Any other finalizer keeps the alpha blend and composites over what
        /// the cameras below it wrote.
        force_replace: bool,
    },
    /// The view sits below its stack's finalizer, so its blit would write
    /// pixels the stack hasn't tonemapped yet. The blit is skipped. The
    /// finalizer's blit writes the whole stack's output instead.
    SkipForFinalizer,
}

/// A view's resolved composition state.
///
/// The contract is overwritten in place every frame and never removed, so a
/// view whose `ViewTarget` was dropped keeps a stale contract. Systems that
/// read the contract must also require `ViewTarget` in their query.
///
/// The component isn't registered for reflection. It's internal to the
/// render world, and its field types aren't `Reflect`.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ViewStackContract {
    /// The view's role for the tonemapping pass.
    pub tonemap: StackRole,
    /// The view's upscaling blit disposition.
    pub blit: BlitDisposition,
}

/// One view's input to [`resolve_contracts`].
struct ContractInput<K> {
    entity: Entity,
    /// Identity of the main textures the view renders into. Views
    /// resolve together only when they share it.
    texture: K,
    /// The camera's position in its render target's sorted camera order.
    sorted_index: usize,
    /// Whether the view covers the whole texture, true without a viewport.
    fullscreen: bool,
    /// Whether the camera renders to an HDR main texture, from
    /// [`ExtractedCamera::hdr`].
    hdr: bool,
    /// Whether the camera writes to its render target. False for
    /// [`CameraOutputMode::Skip`].
    output_writes: bool,
    /// Whether the view's main pass loads the previous buffer contents,
    /// true for [`ClearColorConfig::None`]. A clear covers the whole texture,
    /// viewport or not, so a view that clears starts a new stack.
    loads_previous: bool,
    /// The view's tonemapping method. [`Tonemapping::None`] when the view
    /// has no `Tonemapping` component.
    method: Tonemapping,
}

impl<K> ContractInput<K> {
    /// Whether the view composites over the previous camera's output across
    /// the whole texture. Only such a view can have a finalizer run its pass.
    fn composites_fullscreen(&self) -> bool {
        self.loads_previous && self.fullscreen
    }
}

/// A view's tonemapping pass runs when the camera renders to an HDR main
/// texture and its tonemapping is enabled. This mirrors the tonemapping
/// pass's own gate, so a stack never relies on a pass that doesn't run.
fn tonemap_pass_runs<K>(input: &ContractInput<K>) -> bool {
    input.hdr && input.method.is_enabled()
}

/// A stack misconfiguration found during contract resolution.
/// [`resolve_camera_stack_contracts`] reports each error as a warning with
/// `warn_once!`. [`resolve_contracts`] returns the errors so tests can
/// check the trigger conditions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StackResolutionError {
    /// A fullscreen compositing member blits over regions that members
    /// below it tonemap per camera.
    FullscreenBlitOverPerCameraPasses { fullscreen_camera: Entity },
    /// The texture's first member loads the previous buffer contents while
    /// the texture's last stack runs a tonemapping pass.
    FrameStartLoadsProcessedOutput { first: Entity },
    /// A `HandledBy` member's tonemapping method differs from its finalizer's.
    TonemappingMismatch {
        member: Entity,
        own: Tonemapping,
        finalizing: Tonemapping,
    },
}

/// Groups views by shared main texture, splits each group into stacks at
/// every clearing member, then resolves each view's stack role and blit
/// disposition. Returns the contracts and the errors that fired.
///
/// The resolver requires unique `sorted_index` values within a texture
/// group and doesn't handle ties. `sort_cameras` counts the index per
/// render target and `prepare_view_targets` keys main textures by target,
/// so one group holds one target's cameras. Cameras whose target normalized
/// to `None` are never counted and would tie at zero.
fn resolve_contracts<K: Copy + Eq + Hash>(
    views: Vec<ContractInput<K>>,
) -> (EntityHashMap<ViewStackContract>, Vec<StackResolutionError>) {
    let mut groups: HashMap<K, Vec<ContractInput<K>>> = HashMap::default();
    for view in views {
        groups.entry(view.texture).or_default().push(view);
    }

    let mut contracts = EntityHashMap::default();
    let mut errors = Vec::new();
    for group in groups.values_mut() {
        group.sort_unstable_by_key(|view| view.sorted_index);
        debug_assert!(
            group
                .windows(2)
                .all(|pair| pair[0].sorted_index != pair[1].sorted_index),
            "sorted camera indices must be unique within a texture group"
        );

        // Each clearing member after the first starts a new stack.
        let mut start = 0;
        for index in 1..group.len() {
            if !group[index].loads_previous {
                resolve_stack(&group[start..index], &mut contracts, &mut errors);
                start = index;
            }
        }
        resolve_stack(&group[start..], &mut contracts, &mut errors);

        // The main texture persists across frames. The group's first member
        // loads what the previous frame's last stack wrote, tonemapped when
        // that stack runs the pass. A stack that doesn't tonemap keeps the
        // buffer scene-referred and accumulates stably, so the warning fires
        // only when the pass runs.
        if group[0].loads_previous && group[start..].iter().any(tonemap_pass_runs) {
            errors.push(StackResolutionError::FrameStartLoadsProcessedOutput {
                first: group[0].entity,
            });
        }
    }
    (contracts, errors)
}

/// Returns the index of the member that runs one fullscreen pass for the
/// whole sorted stack, or `None` when the pass runs per camera.
///
/// The stack gets a finalizer only when at least two members tonemap and
/// every tonemapping member after the first composites fullscreen. The
/// finalizer is the last tonemapping member. Any other arrangement keeps
/// each camera running its own pass over what it rendered.
fn pass_finalizer<K>(members: &[ContractInput<K>]) -> Option<usize> {
    let mut tail = members
        .iter()
        .enumerate()
        .filter(|(_, member)| tonemap_pass_runs(member));
    tail.next()?;
    let mut finalizer = None;
    for (index, member) in tail {
        if !member.composites_fullscreen() {
            return None;
        }
        finalizer = Some(index);
    }
    finalizer
}

/// Resolves one stack of sorted members into contracts and errors.
fn resolve_stack<K>(
    members: &[ContractInput<K>],
    contracts: &mut EntityHashMap<ViewStackContract>,
    errors: &mut Vec<StackResolutionError>,
) {
    let tonemap_finalizer = pass_finalizer(members);

    // The finalizer whose blit writes the whole stack's output.
    // A `CameraOutputMode::Skip` finalizer never blits. Lower members skip
    // their blits only because the finalizer's blit writes their output,
    // so without it the stack keeps every blit.
    let blitting_finalizer =
        tonemap_finalizer.filter(|&finalizer| members[finalizer].output_writes);

    // A member covers partially when its output doesn't composite over the
    // whole target. The first member is expected to clear, so a clearing
    // viewport as the first member counts as covering.
    let covers_partially = |index: usize, member: &ContractInput<K>| {
        if index == 0 {
            !member.composites_fullscreen() && member.loads_previous
        } else {
            !member.composites_fullscreen()
        }
    };
    // A fullscreen compositing member above a partially covering member
    // blits the whole target, so regions tonemapped per camera below it
    // get written twice. Per-camera passes exist only when the stack has
    // no finalizer, so the check is gated on that. The reverse shape, a
    // viewport member above members that run their own passes, gets no
    // warning. Any trigger for it would also fire on ordinary split screen.
    if tonemap_finalizer.is_none() {
        let flagged = members.iter().enumerate().find(|(index, candidate)| {
            candidate.composites_fullscreen()
                && members[..*index]
                    .iter()
                    .enumerate()
                    .any(|(below_index, below)| covers_partially(below_index, below))
                && members[..*index].iter().any(tonemap_pass_runs)
        });
        if let Some((_, flagged)) = flagged {
            errors.push(StackResolutionError::FullscreenBlitOverPerCameraPasses {
                fullscreen_camera: flagged.entity,
            });
        }
    }

    for (index, member) in members.iter().enumerate() {
        let tonemap = match tonemap_finalizer {
            Some(finalizer) if index == finalizer => StackRole::Finalizer,
            Some(finalizer) if index < finalizer && tonemap_pass_runs(member) => {
                StackRole::HandledBy(members[finalizer].entity)
            }
            _ => StackRole::Solo,
        };

        let blit = match blitting_finalizer {
            Some(finalizer) if index < finalizer => BlitDisposition::SkipForFinalizer,
            // The finalizer replaces only when every camera below it on the
            // render target is a skipped member of this stack. Its position
            // in the stack then equals its sorted index. A camera on another
            // main texture of the target, or in an earlier stack, has a lower
            // index and has already written the out texture.
            Some(finalizer) if index == finalizer => BlitDisposition::Run {
                force_replace: member.sorted_index == index,
            },
            // Members above the blitting finalizer composite over what
            // the finalizer blitted. Without one, every blit composites
            // over whatever blitted before it.
            _ => BlitDisposition::Run {
                force_replace: false,
            },
        };

        if let (StackRole::HandledBy(_), Some(finalizer)) = (tonemap, tonemap_finalizer) {
            let finalizing = members[finalizer].method;
            if member.method != finalizing {
                errors.push(StackResolutionError::TonemappingMismatch {
                    member: member.entity,
                    own: member.method,
                    finalizing,
                });
            }
        }

        contracts.insert(member.entity, ViewStackContract { tonemap, blit });
    }
}

/// Resolves every camera view's stack into a [`ViewStackContract`].
///
/// Runs in [`RenderSystems::PrepareViews`] after `prepare_view_targets`,
/// which supplies the `ViewTarget`.
pub fn resolve_camera_stack_contracts(
    mut commands: Commands,
    views: Query<(Entity, &ExtractedCamera, &ViewTarget, Option<&Tonemapping>)>,
    mut contracts: Query<&mut ViewStackContract>,
) {
    let inputs: Vec<_> = views
        .iter()
        .map(|(entity, camera, view_target, tonemapping)| ContractInput {
            entity,
            texture: view_target.main_texture().id(),
            sorted_index: camera.sorted_camera_index_for_target,
            fullscreen: camera.viewport.is_none(),
            hdr: camera.hdr,
            output_writes: !matches!(camera.output_mode, CameraOutputMode::Skip),
            loads_previous: matches!(camera.clear_color, ClearColorConfig::None),
            method: *tonemapping.unwrap_or(&Tonemapping::None),
        })
        .collect();

    let (resolved, errors) = resolve_contracts(inputs);
    for error in errors {
        emit_stack_resolution_error(error);
    }
    for (entity, contract) in resolved {
        if let Ok(mut existing) = contracts.get_mut(entity) {
            existing.set_if_neq(contract);
        } else {
            commands.entity(entity).insert(contract);
        }
    }
}

/// Warns about one resolution error.
fn emit_stack_resolution_error(error: StackResolutionError) {
    match error {
        StackResolutionError::FullscreenBlitOverPerCameraPasses { fullscreen_camera } => {
            warn_once!(
                "Fullscreen ClearColorConfig::None camera {fullscreen_camera} composites \
                above viewport cameras whose tonemapping passes run per camera. Its blit \
                covers the whole target and writes their tonemapped pixels a second time. \
                Give that camera its own render target."
            );
        }
        StackResolutionError::FrameStartLoadsProcessedOutput { first } => {
            warn_once!(
                "The first camera rendering to a target, view {first}, uses \
                ClearColorConfig::None while the last camera stack on its main texture runs a \
                tonemapping pass. The main texture persists across frames, so each frame \
                reprocesses last frame's tonemapped output. Feedback and trail effects built \
                this way drift over time. Stable accumulation needs Tonemapping::None, which \
                never tonemaps the buffer."
            );
        }
        StackResolutionError::TonemappingMismatch {
            member,
            own,
            finalizing,
        } => {
            warn_once!(
                "Stacked cameras rendering to the same target use different tonemapping \
                methods. View {member} uses {own:?}, but the stack composites into one \
                shared buffer and tonemaps it once with {finalizing:?}. The last tonemapping \
                camera's Tonemapping, ColorGrading, and DebandDither apply to the whole \
                stack. Give every camera in the stack the same Tonemapping."
            );
        }
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    const RUN: BlitDisposition = BlitDisposition::Run {
        force_replace: false,
    };
    const RUN_REPLACE: BlitDisposition = BlitDisposition::Run {
        force_replace: true,
    };

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    /// An `Hdr` member that clears its target and has tonemapping enabled.
    fn clearing(raw: u32, index: usize) -> ContractInput<u32> {
        ContractInput {
            entity: entity(raw),
            texture: 0,
            sorted_index: index,
            fullscreen: true,
            hdr: true,
            output_writes: true,
            loads_previous: false,
            method: Tonemapping::TonyMcMapface,
        }
    }

    /// A fullscreen `ClearColorConfig::None` member.
    fn compositing(raw: u32, index: usize) -> ContractInput<u32> {
        let mut input = clearing(raw, index);
        input.loads_previous = true;
        input
    }

    /// A viewport member that loads previous content.
    fn viewport(raw: u32, index: usize) -> ContractInput<u32> {
        let mut input = clearing(raw, index);
        input.fullscreen = false;
        input.loads_previous = true;
        input
    }

    /// Marks a member as `Tonemapping::None`, which keeps its pass off.
    fn disabled(mut input: ContractInput<u32>) -> ContractInput<u32> {
        input.method = Tonemapping::None;
        input
    }

    /// Marks a member as SDR, which keeps its pass off whatever the method.
    fn sdr(mut input: ContractInput<u32>) -> ContractInput<u32> {
        input.hdr = false;
        input
    }

    fn resolve(
        views: Vec<ContractInput<u32>>,
    ) -> (EntityHashMap<ViewStackContract>, Vec<StackResolutionError>) {
        resolve_contracts(views)
    }

    fn contract(contracts: &EntityHashMap<ViewStackContract>, raw: u32) -> ViewStackContract {
        *contracts
            .get(&entity(raw))
            .expect("view must have a contract")
    }

    #[test]
    fn solo_camera_runs_its_own_pass_and_blit() {
        let (contracts, errors) = resolve(vec![clearing(1, 0)]);
        let solo = contract(&contracts, 1);
        assert_eq!(solo.tonemap, StackRole::Solo);
        assert_eq!(solo.blit, RUN);
        assert!(errors.is_empty());
    }

    #[test]
    fn hdr_stack_tonemaps_once_and_skips_lower_blits() {
        let (contracts, errors) = resolve(vec![clearing(1, 0), compositing(2, 1)]);
        let base = contract(&contracts, 1);
        let top = contract(&contracts, 2);
        assert_eq!(base.tonemap, StackRole::HandledBy(entity(2)));
        assert_eq!(top.tonemap, StackRole::Finalizer);
        assert_eq!(base.blit, BlitDisposition::SkipForFinalizer);
        assert_eq!(top.blit, RUN_REPLACE);
        assert!(errors.is_empty());
    }

    // SDR members never run the tonemapping pass, so an SDR stack keeps its
    // blits and automatic blending.
    #[test]
    fn sdr_stack_is_solo_and_keeps_its_blits() {
        let (contracts, errors) = resolve(vec![sdr(clearing(1, 0)), sdr(compositing(2, 1))]);
        let base = contract(&contracts, 1);
        let top = contract(&contracts, 2);
        assert_eq!(base.tonemap, StackRole::Solo);
        assert_eq!(top.tonemap, StackRole::Solo);
        assert_eq!(base.blit, RUN);
        assert_eq!(top.blit, RUN);
        assert!(errors.is_empty());
    }

    // A tonemapping base under a disabled compositing camera keeps
    // per-camera behavior. Nothing is handed off or skipped.
    #[test]
    fn single_enabled_member_keeps_per_camera_behavior() {
        let (contracts, errors) = resolve(vec![clearing(1, 0), disabled(compositing(2, 1))]);
        let base = contract(&contracts, 1);
        let top = contract(&contracts, 2);
        assert_eq!(base.tonemap, StackRole::Solo);
        assert_eq!(top.tonemap, StackRole::Solo);
        assert_eq!(base.blit, RUN);
        assert_eq!(top.blit, RUN);
        assert!(errors.is_empty());
    }

    #[test]
    fn three_member_stack_hands_the_pass_to_the_last() {
        let (contracts, _) = resolve(vec![clearing(1, 0), compositing(2, 1), compositing(3, 2)]);
        assert_eq!(
            contract(&contracts, 1).tonemap,
            StackRole::HandledBy(entity(3))
        );
        assert_eq!(
            contract(&contracts, 2).tonemap,
            StackRole::HandledBy(entity(3))
        );
        assert_eq!(contract(&contracts, 3).tonemap, StackRole::Finalizer);
    }

    // The finalizer rule checks every tonemapping member after the first,
    // not just the last.
    #[test]
    fn partially_covering_middle_member_keeps_per_camera_passes() {
        let (contracts, _) = resolve(vec![compositing(1, 0), viewport(2, 1), compositing(3, 2)]);
        for raw in 1..=3 {
            assert_eq!(contract(&contracts, raw).tonemap, StackRole::Solo);
        }
    }

    #[test]
    fn viewport_split_screen_keeps_per_camera_passes() {
        // The first split screen camera clears its target. A
        // `ClearColorConfig::None` first member would trip
        // `FrameStartLoadsProcessedOutput`.
        let mut left = viewport(1, 0);
        left.loads_previous = false;
        let right = viewport(2, 1);
        let (contracts, errors) = resolve(vec![left, right]);
        let left = contract(&contracts, 1);
        let right = contract(&contracts, 2);
        assert_eq!(left.tonemap, StackRole::Solo);
        assert_eq!(right.tonemap, StackRole::Solo);
        assert_eq!(left.blit, RUN);
        assert_eq!(right.blit, RUN);
        assert!(errors.is_empty());
    }

    #[test]
    fn sorted_index_orders_roles_not_insertion_order() {
        let (contracts, _) = resolve(vec![compositing(2, 1), clearing(1, 0)]);
        assert_eq!(
            contract(&contracts, 1).tonemap,
            StackRole::HandledBy(entity(2))
        );
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Finalizer);
        assert_eq!(
            contract(&contracts, 1).blit,
            BlitDisposition::SkipForFinalizer
        );
        assert_eq!(contract(&contracts, 2).blit, RUN_REPLACE);
    }

    // A clearing member starts a new stack regardless of its tonemapping.
    // The base keeps its own pass and blit, or its output would never be
    // written to the target. The new stack has one tonemapping member, so it
    // runs per camera too.
    #[test]
    fn disabled_clearing_member_starts_a_new_stack() {
        let (contracts, errors) = resolve(vec![
            clearing(1, 0),
            disabled(clearing(2, 1)),
            compositing(3, 2),
        ]);
        for raw in 1..=3 {
            assert_eq!(contract(&contracts, raw).tonemap, StackRole::Solo);
            assert_eq!(contract(&contracts, raw).blit, RUN);
        }
        assert!(errors.is_empty());
    }

    // A disabled member that composites sits inside the stack. It is Solo,
    // but its blit is still skipped, since the finalizer's blit writes its
    // output too.
    #[test]
    fn disabled_compositing_member_below_finalizer_skips_blit() {
        let (contracts, errors) = resolve(vec![
            clearing(1, 0),
            disabled(compositing(2, 1)),
            compositing(3, 2),
        ]);
        let base = contract(&contracts, 1);
        let middle = contract(&contracts, 2);
        let finalizer = contract(&contracts, 3);
        assert_eq!(base.tonemap, StackRole::HandledBy(entity(3)));
        assert_eq!(middle.tonemap, StackRole::Solo);
        assert_eq!(finalizer.tonemap, StackRole::Finalizer);
        assert_eq!(base.blit, BlitDisposition::SkipForFinalizer);
        assert_eq!(middle.blit, BlitDisposition::SkipForFinalizer);
        assert_eq!(finalizer.blit, RUN_REPLACE);
        assert!(errors.is_empty());
    }

    // The camera above composites over the finalizer's blit.
    #[test]
    fn disabled_member_above_finalizer_keeps_alpha_blit() {
        let (contracts, _) = resolve(vec![
            clearing(1, 0),
            compositing(2, 1),
            disabled(compositing(3, 2)),
        ]);
        let base = contract(&contracts, 1);
        let finalizer = contract(&contracts, 2);
        let top = contract(&contracts, 3);
        assert_eq!(base.tonemap, StackRole::HandledBy(entity(2)));
        assert_eq!(finalizer.tonemap, StackRole::Finalizer);
        assert_eq!(top.tonemap, StackRole::Solo);
        assert_eq!(base.blit, BlitDisposition::SkipForFinalizer);
        assert_eq!(finalizer.blit, RUN_REPLACE);
        assert_eq!(top.blit, RUN);
    }

    // No member skips its blit for a `CameraOutputMode::Skip` finalizer.
    // The pass roles are unaffected.
    #[test]
    fn skip_finalizer_cancels_blit_skipping() {
        let mut finalizer = compositing(2, 1);
        finalizer.output_writes = false;
        let (contracts, _) = resolve(vec![clearing(1, 0), finalizer]);
        assert_eq!(
            contract(&contracts, 1).tonemap,
            StackRole::HandledBy(entity(2))
        );
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Finalizer);
        assert_eq!(contract(&contracts, 1).blit, RUN);
        assert_eq!(contract(&contracts, 2).blit, RUN);
    }

    // The fullscreen camera's blit rewrites regions already tonemapped
    // per camera. The roles stay Solo.
    #[test]
    fn fullscreen_camera_above_viewport_cameras_is_flagged() {
        let (contracts, errors) = resolve(vec![
            viewport(1, 0),
            viewport(2, 1),
            disabled(compositing(3, 2)),
        ]);
        for raw in 1..=3 {
            let member = contract(&contracts, raw);
            assert_eq!(member.tonemap, StackRole::Solo);
            assert_eq!(member.blit, RUN);
        }
        assert!(
            errors.contains(&StackResolutionError::FullscreenBlitOverPerCameraPasses {
                fullscreen_camera: entity(3)
            })
        );
    }

    // No warning fires here. See the fullscreen-blit check in
    // `resolve_stack`.
    #[test]
    fn viewport_above_enabled_members_is_silent() {
        let (contracts, errors) = resolve(vec![clearing(1, 0), viewport(2, 1)]);
        assert_eq!(contract(&contracts, 1).tonemap, StackRole::Solo);
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Solo);
        assert!(errors.is_empty());
    }

    #[test]
    fn tonemapping_mismatch_is_flagged_on_the_handled_member() {
        let mut base = clearing(1, 0);
        base.method = Tonemapping::AcesFitted;
        let (_, errors) = resolve(vec![base, compositing(2, 1)]);
        assert_eq!(
            errors,
            vec![StackResolutionError::TonemappingMismatch {
                member: entity(1),
                own: Tonemapping::AcesFitted,
                finalizing: Tonemapping::TonyMcMapface,
            }]
        );
    }

    #[test]
    fn frame_start_load_with_tonemapping_is_flagged() {
        let (_, errors) = resolve(vec![compositing(1, 0)]);
        assert_eq!(
            errors,
            vec![StackResolutionError::FrameStartLoadsProcessedOutput { first: entity(1) }]
        );
    }

    // Without a tonemapping pass the load is safe, so no warning fires.
    #[test]
    fn frame_start_load_without_passes_is_silent() {
        let (_, errors) = resolve(vec![disabled(compositing(1, 0))]);
        assert!(errors.is_empty());
    }

    // A clear in the middle starts a second stack. Each stack tonemaps
    // once. The second finalizer keeps the alpha blend, since it composites
    // over the first stack's output.
    #[test]
    fn clear_starts_a_second_stack() {
        let (contracts, errors) = resolve(vec![
            clearing(1, 0),
            compositing(2, 1),
            clearing(3, 2),
            compositing(4, 3),
        ]);
        assert_eq!(
            contract(&contracts, 1).tonemap,
            StackRole::HandledBy(entity(2))
        );
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Finalizer);
        assert_eq!(
            contract(&contracts, 3).tonemap,
            StackRole::HandledBy(entity(4))
        );
        assert_eq!(contract(&contracts, 4).tonemap, StackRole::Finalizer);
        assert_eq!(
            contract(&contracts, 1).blit,
            BlitDisposition::SkipForFinalizer
        );
        assert_eq!(contract(&contracts, 2).blit, RUN_REPLACE);
        assert_eq!(
            contract(&contracts, 3).blit,
            BlitDisposition::SkipForFinalizer
        );
        assert_eq!(contract(&contracts, 4).blit, RUN);
        assert!(errors.is_empty());
    }

    // A viewport clear starts a stack like any clear. As the stack's first
    // member it can be handled by a fullscreen finalizer above it.
    #[test]
    fn viewport_clear_starts_a_second_stack() {
        let mut inset = viewport(3, 2);
        inset.loads_previous = false;
        let (contracts, errors) = resolve(vec![
            clearing(1, 0),
            compositing(2, 1),
            inset,
            compositing(4, 3),
        ]);
        assert_eq!(
            contract(&contracts, 1).tonemap,
            StackRole::HandledBy(entity(2))
        );
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Finalizer);
        assert_eq!(
            contract(&contracts, 3).tonemap,
            StackRole::HandledBy(entity(4))
        );
        assert_eq!(contract(&contracts, 4).tonemap, StackRole::Finalizer);
        assert_eq!(contract(&contracts, 2).blit, RUN_REPLACE);
        assert_eq!(
            contract(&contracts, 3).blit,
            BlitDisposition::SkipForFinalizer
        );
        assert_eq!(contract(&contracts, 4).blit, RUN);
        assert!(errors.is_empty());
    }

    // A camera on another main texture of the same target sits between the
    // stack's members in the sorted order and has already written the out
    // texture, so the finalizer must blend over it.
    #[test]
    fn finalizer_above_a_camera_on_another_texture_keeps_alpha_blit() {
        let (contracts, errors) = resolve(vec![clearing(1, 0), compositing(2, 2)]);
        assert_eq!(
            contract(&contracts, 1).blit,
            BlitDisposition::SkipForFinalizer
        );
        assert_eq!(contract(&contracts, 2).blit, RUN);
        assert!(errors.is_empty());
    }

    // The same applies when the stack's first member isn't the target's
    // first camera.
    #[test]
    fn stack_starting_above_index_zero_keeps_alpha_blit() {
        let (contracts, _) = resolve(vec![clearing(1, 1), compositing(2, 2)]);
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Finalizer);
        assert_eq!(contract(&contracts, 2).blit, RUN);
    }

    // The frame-start check reads the texture's first member and its last
    // stack. The first member loads the last stack's output next frame.
    #[test]
    fn frame_start_load_is_flagged_when_the_last_stack_tonemaps() {
        let (_, errors) = resolve(vec![
            disabled(compositing(1, 0)),
            clearing(2, 1),
            compositing(3, 2),
        ]);
        assert!(errors
            .contains(&StackResolutionError::FrameStartLoadsProcessedOutput { first: entity(1) }));
    }

    // A tonemapping stack that isn't the last one leaves nothing in the
    // texture for the first member to load.
    #[test]
    fn frame_start_load_is_silent_when_only_an_earlier_stack_tonemaps() {
        let (_, errors) = resolve(vec![
            disabled(compositing(1, 0)),
            clearing(2, 1),
            compositing(3, 2),
            disabled(clearing(4, 3)),
        ]);
        assert!(errors.is_empty());
    }

    // Each stack tonemaps with its own finalizer, so the mismatch check
    // never compares across stacks.
    #[test]
    fn tonemapping_mismatch_stays_within_its_stack() {
        let mut base = clearing(1, 0);
        base.method = Tonemapping::AcesFitted;
        let mut second_base = clearing(3, 2);
        second_base.method = Tonemapping::AcesFitted;
        let mut second_top = compositing(4, 3);
        second_top.method = Tonemapping::AcesFitted;
        let (_, errors) = resolve(vec![base, compositing(2, 1), second_base, second_top]);
        assert_eq!(
            errors,
            vec![StackResolutionError::TonemappingMismatch {
                member: entity(1),
                own: Tonemapping::AcesFitted,
                finalizing: Tonemapping::TonyMcMapface,
            }]
        );
    }

    // A `CameraOutputMode::Skip` finalizer in a later stack keeps that
    // stack's blits, as in the first stack.
    #[test]
    fn skip_finalizer_in_a_later_stack_cancels_blit_skipping() {
        let mut finalizer = compositing(4, 3);
        finalizer.output_writes = false;
        let (contracts, _) = resolve(vec![
            clearing(1, 0),
            compositing(2, 1),
            clearing(3, 2),
            finalizer,
        ]);
        assert_eq!(
            contract(&contracts, 3).tonemap,
            StackRole::HandledBy(entity(4))
        );
        assert_eq!(contract(&contracts, 3).blit, RUN);
        assert_eq!(contract(&contracts, 4).blit, RUN);
    }

    #[test]
    fn separate_textures_resolve_independently() {
        let mut other = compositing(2, 0);
        other.texture = 1;
        let (contracts, _) = resolve(vec![clearing(1, 0), other]);
        assert_eq!(contract(&contracts, 1).tonemap, StackRole::Solo);
        assert_eq!(contract(&contracts, 2).tonemap, StackRole::Solo);
        assert_eq!(contract(&contracts, 1).blit, RUN);
        assert_eq!(contract(&contracts, 2).blit, RUN);
    }
}
