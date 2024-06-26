//! In this example we generate four texture atlases (sprite sheets) from a folder containing
//! individual sprites.
//!
//! The texture atlases are generated with different padding and sampling to demonstrate the
//! effect of these settings, and how bleeding issues can be resolved by padding the sprites.
//!
//! Only one padded and one unpadded texture atlas are rendered to the screen.
//! An upscaled sprite from each of the four atlases are rendered to the screen.

use bevy::{asset::LoadedFolder, prelude::*, render::texture::ImageSampler};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // fallback to nearest sampling
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Setup), load_textures)
        .add_systems(Update, check_textures.run_if(in_state(AppState::Setup)))
        .add_systems(OnEnter(AppState::Finished), setup)
        .run();
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
enum AppState {
    #[default]
    Setup,
    Finished,
}

#[derive(Resource, Default)]
struct RpgSpriteFolder(Handle<LoadedFolder>);

fn load_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    // load multiple, individual sprites from a folder
    commands.insert_resource(RpgSpriteFolder(asset_server.load_folder("textures/rpg")));
}

fn check_textures(
    mut next_state: ResMut<NextState<AppState>>,
    rpg_sprite_folder: Res<RpgSpriteFolder>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
) {
    // Advance the `AppState` once all sprite handles have been loaded by the `AssetServer`
    for event in events.read() {
        if event.is_loaded_with_dependencies(&rpg_sprite_folder.0) {
            next_state.set(AppState::Finished);
        }
    }
}

fn setup(
    mut commands: Commands,
    rpg_sprite_handles: Res<RpgSpriteFolder>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut textures: ResMut<Assets<Image>>,
) {
    let loaded_folder = loaded_folders.get(&rpg_sprite_handles.0).unwrap();

    // create texture atlases with different padding and sampling

    let (texture_atlas_linear, linear_texture) = create_texture_atlas(
        loaded_folder,
        None,
        Some(ImageSampler::linear()),
        &mut textures,
    );
    let atlas_linear_handle = texture_atlases.add(texture_atlas_linear.clone());

    let (texture_atlas_nearest, nearest_texture) = create_texture_atlas(
        loaded_folder,
        None,
        Some(ImageSampler::nearest()),
        &mut textures,
    );
    let atlas_nearest_handle = texture_atlases.add(texture_atlas_nearest);

    let (texture_atlas_linear_padded, linear_padded_texture) = create_texture_atlas(
        loaded_folder,
        Some(UVec2::new(6, 6)),
        Some(ImageSampler::linear()),
        &mut textures,
    );
    let atlas_linear_padded_handle = texture_atlases.add(texture_atlas_linear_padded.clone());

    let (texture_atlas_nearest_padded, nearest_padded_texture) = create_texture_atlas(
        loaded_folder,
        Some(UVec2::new(6, 6)),
        Some(ImageSampler::nearest()),
        &mut textures,
    );
    let atlas_nearest_padded_handle = texture_atlases.add(texture_atlas_nearest_padded);

    // setup 2d scene
    commands.spawn(Camera2dBundle::default());

    // padded textures are to the right, unpadded to the left

    // draw unpadded texture atlas
    commands.spawn(SpriteBundle {
        texture: linear_texture.clone(),
        transform: Transform {
            translation: Vec3::new(-250.0, -130.0, 0.0),
            scale: Vec3::splat(0.8),
            ..default()
        },
        ..default()
    });

    // draw padded texture atlas
    commands.spawn(SpriteBundle {
        texture: linear_padded_texture.clone(),
        transform: Transform {
            translation: Vec3::new(250.0, -130.0, 0.0),
            scale: Vec3::splat(0.8),
            ..default()
        },
        ..default()
    });

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // padding label text style
    let text_style: TextStyle = TextStyle {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };

    // labels to indicate padding

    // No padding
    create_label(
        &mut commands,
        (-250.0, 330.0, 0.0),
        "No padding",
        text_style.clone(),
    );

    // Padding
    create_label(&mut commands, (250.0, 330.0, 0.0), "Padding", text_style);

    // get handle to a sprite to render
    let vendor_handle: Handle<Image> = asset_server
        .get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png")
        .unwrap();

    // get index of the sprite in the texture atlas, this is used to render the sprite
    // the index is the same for all the texture atlases, since they are created from the same folder
    let vendor_index = texture_atlas_linear
        .get_texture_index(&vendor_handle)
        .unwrap();

    // configuration array to render sprites through iteration
    let configurations: [(&str, Handle<TextureAtlasLayout>, Handle<Image>, f32); 4] = [
        ("Linear", atlas_linear_handle, linear_texture, -350.0),
        ("Nearest", atlas_nearest_handle, nearest_texture, -150.0),
        (
            "Linear",
            atlas_linear_padded_handle,
            linear_padded_texture,
            150.0,
        ),
        (
            "Nearest",
            atlas_nearest_padded_handle,
            nearest_padded_texture,
            350.0,
        ),
    ];

    // label text style
    let sampling_label_style = TextStyle {
        font,
        font_size: 30.0,
        ..default()
    };

    let base_y = 170.0; // y position of the sprites

    for (sampling, atlas_handle, image_handle, x) in configurations {
        // render a sprite from the texture_atlas
        create_sprite_from_atlas(
            &mut commands,
            (x, base_y, 0.0),
            vendor_index,
            atlas_handle,
            image_handle,
        );

        // render a label to indicate the sampling setting
        create_label(
            &mut commands,
            (x, base_y + 110.0, 0.0), // offset to y position of the sprite
            sampling,
            sampling_label_style.clone(),
        );
    }
}

/// Create a texture atlas with the given padding and sampling settings
/// from the individual sprites in the given folder.
fn create_texture_atlas(
    folder: &LoadedFolder,
    padding: Option<UVec2>,
    sampling: Option<ImageSampler>,
    textures: &mut ResMut<Assets<Image>>,
) -> (TextureAtlasLayout, Handle<Image>) {
    // Build a texture atlas using the individual sprites
    let mut texture_atlas_builder = TextureAtlasBuilder::default();
    texture_atlas_builder.padding(padding.unwrap_or_default());
    for handle in folder.handles.iter() {
        let id = handle.id().typed_unchecked::<Image>();
        let Some(texture) = textures.get(id) else {
            warn!(
                "{:?} did not resolve to an `Image` asset.",
                handle.path().unwrap()
            );
            continue;
        };

        texture_atlas_builder.add_texture(Some(id), texture);
    }

    let (texture_atlas_layout, texture) = texture_atlas_builder.build().unwrap();
    let texture = textures.add(texture);

    // Update the sampling settings of the texture atlas
    let image = textures.get_mut(&texture).unwrap();
    image.sampler = sampling.unwrap_or_default();

    (texture_atlas_layout, texture)
}

/// Create and spawn a sprite from a texture atlas
fn create_sprite_from_atlas(
    commands: &mut Commands,
    translation: (f32, f32, f32),
    sprite_index: usize,
    atlas_handle: Handle<TextureAtlasLayout>,
    texture: Handle<Image>,
) {
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: Vec3::new(translation.0, translation.1, translation.2),
                scale: Vec3::splat(3.0),
                ..default()
            },
            texture,
            ..default()
        },
        TextureAtlas {
            layout: atlas_handle,
            index: sprite_index,
        },
    ));
}

/// Create and spawn a label (text)
fn create_label(
    commands: &mut Commands,
    translation: (f32, f32, f32),
    text: &str,
    text_style: TextStyle,
) {
    commands.spawn(Text2dBundle {
        text: Text::from_section(text, text_style).with_justify(JustifyText::Center),
        transform: Transform {
            translation: Vec3::new(translation.0, translation.1, translation.2),
            ..default()
        },
        ..default()
    });
}
