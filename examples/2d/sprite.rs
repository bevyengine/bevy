use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut texture_storage = resources.get_mut::<AssetStorage<Texture>>().unwrap();
    let texture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/branding/icon.png");
    let texture = Texture::load(TextureType::Png(texture_path.to_string()));
    let texture_handle = texture_storage.add(texture);
    let mut color_materials = resources.get_mut::<AssetStorage<ColorMaterial>>().unwrap();

    world
        .build()
        .add_entity(Camera2dEntity::default())
        .add_entity(SpriteEntity {
            rect: Rect {
                position: Vec2::new(300.0, 300.0),
                z_index: 0.5,
                ..Default::default()
            },
            material: color_materials.add(texture_handle.into()),
            ..Default::default()
        });
}
