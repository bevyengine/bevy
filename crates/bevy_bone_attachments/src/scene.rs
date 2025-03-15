//! Types to help attaching a scene to an entity

use alloc::vec::Vec;
use bevy_animation::{AnimationTarget, AnimationTargetId};
use bevy_asset::Handle;
use bevy_ecs::{
    bundle::Bundle,
    hierarchy::Children,
    observer::Trigger,
    relationship::RelatedSpawnerCommands,
    system::{Commands, EntityCommands, Query},
};
use bevy_platform_support::collections::{hash_map::Entry, HashMap};
use bevy_scene::{Scene, SceneInstanceReady, SceneRoot};

use crate::prelude::AttachedTo;

/// Extension trait for [`EntityCommands`] to allow attaching a [`Scene`] to an [`Entity`](bevy_ecs::entity::Entity).
pub trait SceneAttachmentExt {
    /// Attaches a [`Scene`] to an [`Entity`](bevy_ecs::entity::Entity).
    fn attach_scene(&mut self, scene: Handle<Scene>) -> &mut Self;

    /// Attaches a [`Scene`] to an [`Entity`](bevy_ecs::entity::Entity) and inserts an extra [`Bundle`]
    /// on the attachment.
    fn attach_scene_with_extras(&mut self, scene: Handle<Scene>, extras: impl Bundle) -> &mut Self;
}

impl<'a> SceneAttachmentExt for EntityCommands<'a> {
    #[inline]
    fn attach_scene(&mut self, scene: Handle<Scene>) -> &mut EntityCommands<'a> {
        self.attach_scene_with_extras(scene, ())
    }

    #[inline]
    fn attach_scene_with_extras(
        &mut self,
        scene: Handle<Scene>,
        extras: impl Bundle,
    ) -> &mut EntityCommands<'a> {
        self.with_related(|spawner: &mut RelatedSpawnerCommands<AttachedTo>| {
            spawner
                .spawn((SceneRoot(scene), extras))
                .observe(scene_attachment_ready);
        })
    }
}

fn scene_attachment_ready(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    scene_attachments: Query<&AttachedTo>,
    children: Query<&Children>,
    animation_targets: Query<&AnimationTarget>,
    animation_target_ids: Query<&AnimationTargetId>,
) {
    let Ok(parent) = scene_attachments.get(trigger.target()) else {
        unreachable!("AttachedTo must be available on SceneInstanceReady.");
    };

    let mut duplicate_target_ids_on_parent_hierarchy = Vec::new();
    let mut target_ids = HashMap::new();
    for child in children.iter_descendants(**parent) {
        if child == trigger.target() {
            continue;
        }

        if let Ok(animation_target) = animation_targets.get(child) {
            match target_ids.entry(animation_target.id) {
                Entry::Vacant(vacancy) => {
                    vacancy.insert(animation_target.player);
                }
                Entry::Occupied(_) => {
                    duplicate_target_ids_on_parent_hierarchy.push(animation_target.id);
                }
            }
        }
    }
    if !duplicate_target_ids_on_parent_hierarchy.is_empty() {
        tracing::warn!(
            "There where nodes with duplicate AnimationTargetId on the hierarchy if {}, using the first appearance. {:?}",
            **parent,
            duplicate_target_ids_on_parent_hierarchy
        );
    }

    let mut count = 0;
    let mut unmatched_animation_target_id = Vec::new();
    for child in children.iter_descendants(trigger.target()) {
        if let Ok(animation_target_id) = animation_target_ids.get(child) {
            if let Some(player) = target_ids.get(animation_target_id) {
                commands.entity(child).insert(AnimationTarget {
                    id: *animation_target_id,
                    player: *player,
                });
                count += 1;
            } else {
                unmatched_animation_target_id.push(animation_target_id);
            }
        }
    }
    if !unmatched_animation_target_id.is_empty() {
        tracing::warn!(
            "There where nodes with unmatched AnimationTargetId on the hierarchy if {}, this may cause bone attachment to not update correctly. {:?}",
            trigger.target(),
            unmatched_animation_target_id
        );
    }
    tracing::debug!(
        "Attachment {} matched {} nodes with parent.",
        trigger.target(),
        count
    );

    commands.entity(trigger.target());
}
