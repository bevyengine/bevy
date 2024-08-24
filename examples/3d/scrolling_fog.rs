//! Showcases a `FogVolume`'s density texture being scrolled over time to create
//! the effect of fog moving in the wind.
//!
//! The density texture is a repeating 3d noise texture and the `density_texture_offset`
//! is moved every frame to achieve this.
//!
//! The example also utilizes the jitter option of `VolumetricFogSettings` in tandem
//! with temporal anti-aliasing to improve the visual quality of the effect.
//!
//! The camera is looking at a pillar with the sun peaking behind it. The light
//! interactions change based on the density of the fog.

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin};
use bevy::pbr::{DirectionalLightShadowMap, FogVolume, VolumetricFogSettings, VolumetricLight};
use bevy::prelude::*;
use bevy_render::texture::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};

/// Initializes the example.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Scrolling Fog".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(TemporalAntiAliasPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, scroll_fog)
        .run();
}

/// Spawns all entities into the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    // Spawn camera with temporal anti-aliasing and a VolumetricFogSettings configuration.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 2.0, 0.0)
                .looking_at(Vec3::new(-5.0, 3.5, -6.0), Vec3::Y),
            camera: Camera {
                hdr: true,
                ..default()
            },
            msaa: Msaa::Off,
            ..default()
        },
        TemporalAntiAliasBundle::default(),
        BloomSettings::default(),
        VolumetricFogSettings {
            ambient_intensity: 0.0,
            jitter: 0.5,
            ..default()
        },
    ));

    // Spawn a directional light shining at the camera with the VolumetricLight component.
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(-5.0, 5.0, -7.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        VolumetricLight,
    ));

    // Spawn ground mesh.
    commands.spawn(PbrBundle {
        transform: Transform::from_xyz(0.0, -0.5, 0.0),
        mesh: meshes.add(Cuboid::new(64.0, 1.0, 64.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            perceptual_roughness: 1.0,
            ..default()
        }),
        ..default()
    });

    // Spawn pillar standing between the camera and the sun.
    commands.spawn(PbrBundle {
        transform: Transform::from_xyz(-10.0, 4.5, -11.0),
        mesh: meshes.add(Cuboid::new(2.0, 9.0, 2.0)),
        material: materials.add(Color::BLACK),
        ..default()
    });

    // Load a repeating 3d noise texture. Make sure to set ImageAddressMode to Repeat
    // so that the texture wraps around as the density texture offset is moved along.
    // Also set ImageFilterMode to Linear so that the fog isn't pixelated.
    let noise_texture = assets.load_with_settings("volumes/fog_noise.ktx2", |settings: &mut _| {
        *settings = ImageLoaderSettings {
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                address_mode_w: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                ..default()
            }),
            ..default()
        }
    });

    // Spawn a FogVolume and use the repeating noise texture as its density texture.
    commands.spawn((
        SpatialBundle {
            visibility: Visibility::Visible,
            transform: Transform::from_xyz(0.0, 32.0, 0.0).with_scale(Vec3::splat(64.0)),
            ..default()
        },
        FogVolume {
            density_texture: Some(noise_texture),
            density_factor: 0.05,
            ..default()
        },
    ));
}

/// Moves fog density texture offset every frame.
fn scroll_fog(time: Res<Time>, mut query: Query<&mut FogVolume>) {
    for mut fog_volume in query.iter_mut() {
        fog_volume.density_texture_offset += Vec3::new(0.0, 0.0, 0.04) * time.delta_seconds();
    }
}
