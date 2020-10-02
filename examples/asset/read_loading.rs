use bevy::prelude::*;

/// This example illustrates various ways to load assets
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // You can use `include_bytes!` to load a file as an `&[u8]` at compile time. This will bundle the asset in the binary.
    // It can then be loaded synchronously:
    let bytes = include_bytes!("../../assets/branding/icon.png");
    let texture_handle = asset_server
        .load_sync_from(&mut textures, &mut bytes.as_ref())
        .unwrap();

    // Or you can use any object implementing `std::io::Read`.
    // To load it asynchronously, you then need to wrap it in a `Box`:
    let file = std::fs::File::open("assets/branding/icon.png").unwrap();
    let texture_handle_2 = asset_server.load_from(Box::new(file)).unwrap();

    commands
        .spawn(Camera2dComponents::default())
        .spawn(SpriteComponents {
            material: materials.add(texture_handle.into()),
            transform: Transform::default().with_translation(Vec3::new(-300., 0., 0.)),
            ..Default::default()
        })
        .spawn(SpriteComponents {
            material: materials.add(texture_handle_2.into()),
            transform: Transform::default().with_translation(Vec3::new(300., 0., 0.)),
            ..Default::default()
        });
}
