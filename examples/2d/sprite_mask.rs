//! Displays partial rendering of [`Sprite`]s using [`Mask`]s.

use bevy::{
    asset::AssetId,
    prelude::*,
    render::{render_resource::*, texture::ImageSampler},
    sprite::*,
    utils::HashMap,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<MaskAddressModes>()
        .add_systems(Startup, setup)
        .add_systems(Update, set_mask_address_mode)
        .add_systems(Update, orbit)
        .run();
}

fn setup(
    mut commands: Commands,
    mut mask_address_modes: ResMut<MaskAddressModes>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2dBundle::default());

    let mask_image = asset_server.load("textures/Game Icons/wrench.png");

    let mask = commands
        .spawn((
            SpriteMaskBundle {
                mask: Mask {
                    image: mask_image,
                    custom_size: Some(Vec2::splat(128.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
            Orbit {
                center: Vec3::X * -300.0,
                magnitude: Vec3::new(75.0, 300.0, 0.0),
            },
        ))
        .id();

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("textures/array_texture.png"),
            transform: Transform::from_translation(Vec3::X * -300.0),
            ..default()
        },
        Masked { mask },
    ));

    let repeating_mask_image = asset_server.load("branding/icon.png");
    (*mask_address_modes)
        .0
        .insert(repeating_mask_image.id(), AddressMode::Repeat);

    let repeating_mask = commands
        .spawn(SpriteMaskBundle {
            mask: Mask {
                image: repeating_mask_image,
                ..Default::default()
            },
            transform: Transform::from_scale(Vec3::splat(0.5)),
            ..Default::default()
        })
        .id();

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("branding/bevy_logo_dark.png"),
            ..default()
        },
        Masked {
            mask: repeating_mask,
        },
        Orbit {
            center: Vec3::X * 250.0,
            magnitude: Vec3::Y * -300.0,
        },
    ));
}

#[derive(Default, Resource)]
struct MaskAddressModes(HashMap<AssetId<Image>, AddressMode>);

fn set_mask_address_mode(
    mut events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mask_address_modes: Res<MaskAddressModes>,
) {
    // Change the `AddressMode` of the `SpriteMask` `Image` sampler once it has loaded
    for event in events.read() {
        if let &AssetEvent::LoadedWithDependencies { id } = event {
            if let Some(&address_mode) = mask_address_modes.0.get(&id) {
                if let Some(image) = images.get_mut(id) {
                    image.sampler_descriptor = ImageSampler::Descriptor({
                        SamplerDescriptor {
                            address_mode_u: address_mode,
                            address_mode_v: address_mode,
                            address_mode_w: address_mode,
                            ..ImageSampler::linear_descriptor()
                        }
                    });
                }
            }
        }
    }
}

#[derive(Component)]
struct Orbit {
    center: Vec3,
    magnitude: Vec3,
}

fn orbit(time: Res<Time>, mut orbits: Query<(&mut Transform, &Orbit)>) {
    for (mut transform, orbit) in orbits.iter_mut() {
        transform.translation = orbit.center
            + orbit.magnitude
                * Vec3::new(
                    time.elapsed_seconds().sin(),
                    time.elapsed_seconds().cos(),
                    0.0,
                );
    }
}
