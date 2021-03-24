use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::{Commands, Entity, Query, Res, With};
use bevy_math::Vec2;
use bevy_render::{camera::Camera, draw::OutsideFrustum};
use bevy_transform::components::Transform;
use bevy_window::Windows;

use crate::{Sprite, TextureAtlas, TextureAtlasSprite};

struct Rect {
    position: Vec2,
    size: Vec2,
}

impl Rect {
    #[inline]
    pub fn is_intersecting(&self, other: Rect) -> bool {
        self.position.distance(other.position) < (self.get_radius() + other.get_radius())
    }

    #[inline]
    pub fn get_radius(&self) -> f32 {
        let half_size = self.size / Vec2::splat(2.0);
        (half_size.x.powf(2.0) + half_size.y.powf(2.0)).sqrt()
    }
}

pub fn sprites(
    mut commands: Commands,
    windows: Res<Windows>,
    cameras: Query<&Transform, With<Camera>>,
    culled_sprites: Query<&OutsideFrustum, With<Sprite>>,
    sprites: Query<(Entity, &Transform, &Sprite)>,
) {
    let window_size = if let Some(window) = windows.get_primary() {
        Vec2::new(window.width(), window.height())
    } else {
        return;
    };

    for camera_transform in cameras.iter() {
        let camera_size = window_size * camera_transform.scale.truncate();

        let rect = Rect {
            position: camera_transform.translation.truncate(),
            size: camera_size,
        };

        for (entity, drawable_transform, sprite) in sprites.iter() {
            let sprite_rect = Rect {
                position: drawable_transform.translation.truncate(),
                size: sprite.size,
            };

            if rect.is_intersecting(sprite_rect) {
                if culled_sprites.get(entity).is_ok() {
                    commands.entity(entity).remove::<OutsideFrustum>();
                }
            } else if culled_sprites.get(entity).is_err() {
                commands.entity(entity).insert(OutsideFrustum);
            }
        }
    }
}

pub fn atlases(
    mut commands: Commands,
    windows: Res<Windows>,
    textures: Res<Assets<TextureAtlas>>,
    cameras: Query<&Transform, With<Camera>>,
    culled_sprites: Query<&OutsideFrustum, With<TextureAtlasSprite>>,
    sprites: Query<(
        Entity,
        &Transform,
        &TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    let window = windows.get_primary().unwrap();
    let window_size = Vec2::new(window.width(), window.height());

    for camera_transform in cameras.iter() {
        let camera_size = window_size * camera_transform.scale.truncate();

        let rect = Rect {
            position: camera_transform.translation.truncate(),
            size: camera_size,
        };

        for (entity, drawable_transform, sprite, atlas_handle) in sprites.iter() {
            if let Some(atlas) = textures.get(atlas_handle) {
                if let Some(sprite) = atlas.textures.get(sprite.index as usize) {
                    let size = Vec2::new(sprite.width(), sprite.height());

                    let sprite_rect = Rect {
                        position: drawable_transform.translation.truncate(),
                        size,
                    };

                    if rect.is_intersecting(sprite_rect) {
                        if culled_sprites.get(entity).is_ok() {
                            commands.entity(entity).remove::<OutsideFrustum>();
                        }
                    } else if culled_sprites.get(entity).is_err() {
                        commands.entity(entity).insert(OutsideFrustum);
                    }
                }
            }
        }
    }
}
