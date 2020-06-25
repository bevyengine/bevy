use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(animate_sprite_system.system())
        .run();
}

fn animate_sprite_system(
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut timer: ComMut<Timer>,
    mut sprite: ComMut<TextureAtlasSprite>,
    texture_atlas_handle: Com<Handle<TextureAtlas>>,
) {
    if timer.finished {
        let texture_atlas = texture_atlases.get(&texture_atlas_handle).unwrap();
        sprite.index = ((sprite.index as usize + 1) % texture_atlas.textures.len()) as u32;
        timer.reset();
    }
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server
        .load_sync(
            &mut textures,
            "assets/textures/rpg/chars/gabe/gabe-idle-run.png",
        )
        .unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let texture_atlas = TextureAtlas::from_grid(texture_handle, texture.size, 7, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    command_buffer
        .build()
        .entity_with(OrthographicCameraComponents::default())
        .entity_with(SpriteSheetComponents {
            texture_atlas: texture_atlas_handle,
            scale: Scale(6.0),
            ..Default::default()
        })
        .with(Timer::from_seconds(0.1));
}
