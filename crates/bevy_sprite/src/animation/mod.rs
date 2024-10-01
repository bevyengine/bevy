pub mod clip;
pub mod player;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetApp, Assets, Handle};
use bevy_ecs::system::{ParamSet, Query, Res, ResMut};
use bevy_render::prelude::Image;
use bevy_time::Time;
use bevy_utils::tracing::warn;
use clip::{FrameContent, SpriteClip};
use player::SpriteAnimationPlayer;
use crate::TextureAtlas;

#[allow(clippy::type_complexity)]
pub fn update_sprite_animation(
    mut query: ParamSet<(
        // Unsynced animations query
        Query<(
            &mut SpriteAnimationPlayer,
            &mut Handle<Image>,
            Option<&mut TextureAtlas>,
        )>,
        // Synced animations query
        Query<(
            &Handle<SpriteAnimationPlayer>,
            &mut Handle<Image>,
            Option<&mut TextureAtlas>,
        )>,
    )>,
    time: Res<Time>,
    clips: Res<Assets<SpriteClip>>,
    mut players: ResMut<Assets<SpriteAnimationPlayer>>,
) {
    // Update unsynced animation players
    for (mut player, mut image, atlas) in query.p0().iter_mut() {
        match player.next(time.delta(), &clips) {
            Some(FrameContent::Atlas(index)) => {
                let Some(mut atlas) = atlas else {
                    continue;
                };
                atlas.index = index;
            }
            Some(FrameContent::Image(frame)) => {
                atlas.inspect(|_| warn!("You have a sprite animation clip with an image frame while using an atlas. This might not be what you expect."));
                *image = frame;
            }
            None => {}
        }
    }

    // Update synced animation players
    for (_, player) in players.iter_mut() {
        player.next(time.delta(), &clips);
    }
    // Update the frames on entities that use synced animations
    for (player, mut image, atlas) in query.p1().iter_mut().filter_map(|(player, image, atlas)| {
        let player = players.get(player)?;
        Some((player, image, atlas))
    }) {
        match &player.current_frame {
            Some(FrameContent::Atlas(index)) => {
                let Some(mut atlas) = atlas else {
                    continue;
                };
                atlas.index = *index;
            }
            Some(FrameContent::Image(frame)) => {
                atlas.inspect(|_| warn!("You have a sprite animation clip with an image frame while using an atlas. This might not be what you expect."));
                *image = frame.clone();
            }
            None => {}
        }
    }
}

pub struct SpriteAnimationPlugin;

impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SpriteClip>()
            .init_asset::<SpriteAnimationPlayer>()
            .add_systems(PostUpdate, update_sprite_animation);
    }
}

#[cfg(test)]
mod tests {
    // Transitions, clip overrides and such are not tested here.
    // We assume that if `player.rs` unit tests are correct, they have to be here.
    #[test]
    #[ignore]
    fn unsync_image_update() {}

    #[test]
    #[ignore]
    fn unsync_atlas_update() {}

    #[test]
    #[ignore]
    fn sync_image_update() {}

    #[test]
    #[ignore]
    fn sync_atlas_update() {}
}
