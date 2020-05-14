use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture = Texture::load(TextureType::Png("assets/branding/icon.png".to_string()));
    let texture_handle = textures.add(texture);

    command_buffer
        .build()
        .add_entity(Camera2dEntity::default())
        .add_entity(SpriteEntity {
            rect: Rect {
                position: Vec2::new(300.0, 300.0),
                z_index: 0.5,
                ..Default::default()
            },
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}
