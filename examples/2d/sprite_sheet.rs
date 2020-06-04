use bevy::{input::system::exit_on_esc_system, prelude::*};
use bevy_sprite::{SpriteSheet, SpriteSheetSprite};

fn main() {
    App::build()
        .init_resource::<State>()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .init_system(exit_on_esc_system)
        .add_system(animate_sprite_system.system())
        .run();
}

#[derive(Default)]
struct State {
    elapsed: f32,
}

fn animate_sprite_system(
    mut state: ResMut<State>,
    time: Res<Time>,
    sprite_sheets: Res<Assets<SpriteSheet>>,
    mut sprite: ComMut<SpriteSheetSprite>,
    sprite_sheet_handle: Com<Handle<SpriteSheet>>,
) {
    state.elapsed += time.delta_seconds;
    if state.elapsed > 0.1 {
        state.elapsed = 0.0;
        let sprite_sheet = sprite_sheets.get(&sprite_sheet_handle).unwrap();
        sprite.index = ((sprite.index as usize + 1) % sprite_sheet.sprites.len()) as u32;
    }
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut sprite_sheets: ResMut<Assets<SpriteSheet>>,
) {
    env_logger::init();
    let texture_handle = asset_server
        .load_sync(&mut textures, "assets/textures/character_run.png")
        .unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let sprite_sheet = SpriteSheet::from_grid(texture_handle, texture.size, 7, 1);
    let sprite_sheet_handle = sprite_sheets.add(sprite_sheet);
    command_buffer
        .build()
        .add_entity(OrthographicCameraEntity::default())
        .add_entity(SpriteSheetEntity {
            sprite_sheet: sprite_sheet_handle,
            sprite: SpriteSheetSprite {
                index: 0,
                scale: 6.0,
                position: Vec3::new(0.0, 0.0, -0.5),
            },
            ..Default::default()
        });
}
