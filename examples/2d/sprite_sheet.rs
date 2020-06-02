use bevy::prelude::*;
use bevy::input::system::exit_on_esc_system;
use bevy_sprite::{Rect, SpriteSheet};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .init_system(exit_on_esc_system)
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut sprite_sheets: ResMut<Assets<SpriteSheet>>,
) {
    let texture_handle = asset_server.load("assets/branding/icon.png").unwrap();
    let sprite_sheet = SpriteSheet {
        texture: texture_handle,
        sprites: vec![
            Rect {
                min: Vec2::new(0.0, 0.0),  
                max: Vec2::new(1.0, 1.0),  
            }
        ]
    };
    let sprite_sheet_handle = sprite_sheets.add(sprite_sheet);
    command_buffer
        .build()
        .add_entity(OrthographicCameraEntity::default())
        .add_entity(SpriteSheetEntity {
            sprite_sheet: sprite_sheet_handle,
            ..Default::default()
        });
}
