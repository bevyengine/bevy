use bevy::{
    asset::LoadState,
    math::Vec3,
    prelude::{
        App, AssetServer, Assets, Commands, HandleUntyped, Res, ResMut, State, SystemSet, Transform,
    },
    render2::{camera::OrthographicCameraBundle, image::Image},
    sprite2::{
        AtlasSprite, ImageAtlas, ImageAtlasBuilder, PipelinedAtlasSpriteBundle,
        PipelinedSpriteBundle,
    },
    PipelinedDefaultPlugins,
};

/// In this example we generate a new [`ImageAtlas`] (sprite sheet) from a folder containing
/// individual images.
fn main() {
    App::new()
        .init_resource::<RpgImageHandles>()
        .add_plugins(PipelinedDefaultPlugins)
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_enter(AppState::Setup).with_system(load_images))
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_images))
        .add_system_set(SystemSet::on_enter(AppState::Finished).with_system(setup))
        .run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Setup,
    Finished,
}

#[derive(Default)]
struct RpgImageHandles {
    handles: Vec<HandleUntyped>,
}

fn load_images(mut rpg_sprite_handles: ResMut<RpgImageHandles>, asset_server: Res<AssetServer>) {
    rpg_sprite_handles.handles = asset_server.load_folder("textures/rpg").unwrap();
}

fn check_images(
    mut state: ResMut<State<AppState>>,
    rpg_image_handles: ResMut<RpgImageHandles>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded =
        asset_server.get_group_load_state(rpg_image_handles.handles.iter().map(|handle| handle.id))
    {
        state.set(AppState::Finished).unwrap();
    }
}

fn setup(
    mut commands: Commands,
    rpg_image_handles: Res<RpgImageHandles>,
    asset_server: Res<AssetServer>,
    mut image_atlases: ResMut<Assets<ImageAtlas>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut atlas_builder = ImageAtlasBuilder::default();
    for handle in rpg_image_handles.handles.iter() {
        let image = images.get(handle).unwrap();
        atlas_builder.add_image(handle.clone_weak().typed::<Image>(), image);
    }

    let image_atlas = atlas_builder.finish(&mut images).unwrap();
    let image_atlas_source = image_atlas.source_image.clone();
    let vendor_handle = asset_server.get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png");
    let vendor_index = image_atlas.get_region_index(&vendor_handle).unwrap();
    let atlas_handle = image_atlases.add(image_atlas);

    // set up a scene to display our texture atlas
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    // draw a sprite from the atlas
    commands.spawn_bundle(PipelinedAtlasSpriteBundle {
        transform: Transform {
            translation: Vec3::new(150.0, 0.0, 0.0),
            scale: Vec3::splat(4.0),
            ..Default::default()
        },
        sprite: AtlasSprite {
            region_index: vendor_index as u32,
            color: Default::default(),
            flip_x: true,
            flip_y: true,
        },
        image_atlas: atlas_handle,
        ..Default::default()
    });
    // draw the atlas itself
    commands.spawn_bundle(PipelinedSpriteBundle {
        image: image_atlas_source,
        transform: Transform::from_xyz(-300.0, 0.0, 0.0),
        ..Default::default()
    });
}
