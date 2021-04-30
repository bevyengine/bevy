use bevy::prelude::*;

use std::num::NonZeroU8;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(TextureFiltering::Anisotropic)
        .add_startup_system(setup_plane.system())
        .add_startup_system(setup_ui.system())
        .add_system(swing_camera.system())
        .add_system(switch_filter_mode.system())
        .run();
}

struct MainCamera;
struct ModeText;

struct MainAssets {
    texture: Handle<Texture>,
    material: Handle<StandardMaterial>,
}

enum TextureFiltering {
    Nearest,
    Linear,
    NearestMipmap,
    LinearMipmap,
    Anisotropic,
}

fn setup_plane(
    mut commands: Commands,
    mode: Res<TextureFiltering>,
    mut textures: ResMut<Assets<Texture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tex_size = 256;
    let mut tex = generate_texture(tex_size, -0.8, 0.156);
    mode.apply(&mut tex);
    let tex = textures.add(tex);

    let quad_size = Vec2::new(20.0, 200.0);
    let quad = meshes.add(Mesh::from(shape::Quad {
        size: quad_size,
        flip: false,
        uv_scale: quad_size,
    }));

    let material = materials.add(StandardMaterial {
        albedo_texture: Some(tex.clone()),
        unlit: true,
        ..Default::default()
    });

    commands.insert_resource(MainAssets {
        texture: tex,
        material: material.clone(),
    });

    commands
        .spawn(PbrBundle {
            mesh: quad,
            material,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, -quad_size.y / 4.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 2.0),
                ..Default::default()
            },
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .spawn(PerspectiveCameraBundle::default())
        .with(MainCamera);
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>, mode: Res<TextureFiltering>) {
    commands
        .spawn(UiCameraBundle::default())
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "(space to change)".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 32.0,
                            color: Color::ALICE_BLUE,
                        },
                    },
                    TextSection {
                        value: " Current Mode: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 32.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: mode.screen_text().to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                            font_size: 32.0,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .with(ModeText);
}

fn swing_camera(t: Res<Time>, mut q: Query<&mut Transform, With<MainCamera>>) {
    let secs = t.seconds_since_startup();

    let translation = Vec3::new(secs.sin() as f32, 1.5 + (secs * 0.3).sin() as f32, 5.0);

    for mut transform in q.iter_mut() {
        *transform = Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

fn switch_filter_mode(
    mut mode: ResMut<TextureFiltering>,
    mut textures: ResMut<Assets<Texture>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    main: Res<MainAssets>,
    kbd: Res<Input<KeyCode>>,
    mut q_text: Query<&mut Text, With<ModeText>>,
) {
    if kbd.just_pressed(KeyCode::Space) {
        let next = mode.cycle_next();

        // apply the next filtermode config to the texture
        let tex = textures.get_mut(&main.texture).unwrap();
        next.apply(tex);

        // Bevy does not seem to detect the modification,
        // unless we also mutate the material
        let mat = mats.get_mut(&main.material).unwrap();
        mat.albedo_texture = Some(main.texture.clone());

        // update the on-screen text
        for mut text in q_text.iter_mut() {
            text.sections[2].value = next.screen_text().to_string();
        }

        *mode = next;
    }
}

/// Procedurally-generate a texture of a julia fractal
fn generate_texture(size: u32, cx: f32, cy: f32) -> Texture {
    use std::iter;

    // This is a procedurally-generated texture, so we can generate the image
    // directly at any size we want. Generating each mipmap level like this
    // should produce the prettiest-looking results.
    let mut mipmaps = Vec::new();
    let mut mip_size = size;

    // Limiting the smallest mipmap size to 4x4 helps avoid the texture looking
    // plain white/grey when viewed at very far away distances.
    while mip_size >= 4 {
        // copied from the wgpu-rs mipmap example:
        // https://github.com/gfx-rs/wgpu-rs/blob/master/examples/mipmap/main.rs
        let texels = (0..mip_size * mip_size)
            .flat_map(|id| {
                let mut x = 4.0 * (id % mip_size) as f32 / (mip_size - 1) as f32 - 2.0;
                let mut y = 2.0 * (id / mip_size) as f32 / (mip_size - 1) as f32 - 1.0;
                let mut count = 0;
                while count < 0xFF && x * x + y * y < 4.0 {
                    let old_x = x;
                    x = x * x - y * y + cx;
                    y = 2.0 * old_x * y + cy;
                    count += 1;
                }
                iter::once(0xFF - (count * 2) as u8)
                    .chain(iter::once(0xFF - (count * 5) as u8))
                    .chain(iter::once(0xFF - (count * 13) as u8))
                    .chain(iter::once(std::u8::MAX))
            })
            .collect();

        mipmaps.push(texels);
        mip_size /= 2;
    }

    use bevy::render::texture::{
        AddressMode, Extent3d, SamplerDescriptor, TextureDimension, TextureFormat,
    };

    let sampler = SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
        ..Default::default()
    };

    let data = mipmaps.remove(0);

    Texture {
        data,
        mipmaps,
        size: Extent3d::new(size, size, 1),
        max_mip_level: None,
        format: TextureFormat::Rgba8UnormSrgb,
        dimension: TextureDimension::D2,
        sampler,
    }

    // Alternatively, if we hadn't generated our own mipmaps above, we could
    // have asked Bevy to make them for us by downscaling the base image level:
    //texture.generate_mipmaps(None, Some(4));

    // You can try to remove the while loop earlier (to only generate one image)
    // and uncomment the line above, if you want to see how it looks and compare
}

impl TextureFiltering {
    fn apply(&self, tex: &mut Texture) {
        match self {
            TextureFiltering::Nearest => {
                tex.max_mip_level = Some(0);
                tex.sampler.mag_filter = FilterMode::Nearest;
                tex.sampler.min_filter = FilterMode::Nearest;
                tex.sampler.mipmap_filter = FilterMode::Nearest;
                tex.sampler.anisotropy_clamp = None;
            }
            TextureFiltering::Linear => {
                tex.max_mip_level = Some(0);
                tex.sampler.mag_filter = FilterMode::Linear;
                tex.sampler.min_filter = FilterMode::Linear;
                tex.sampler.mipmap_filter = FilterMode::Nearest;
                tex.sampler.anisotropy_clamp = None;
            }
            TextureFiltering::NearestMipmap => {
                tex.max_mip_level = None;
                tex.sampler.mag_filter = FilterMode::Linear;
                tex.sampler.min_filter = FilterMode::Linear;
                tex.sampler.mipmap_filter = FilterMode::Nearest;
                tex.sampler.anisotropy_clamp = None;
            }
            TextureFiltering::LinearMipmap => {
                tex.max_mip_level = None;
                tex.sampler.mag_filter = FilterMode::Linear;
                tex.sampler.min_filter = FilterMode::Linear;
                tex.sampler.mipmap_filter = FilterMode::Linear;
                tex.sampler.anisotropy_clamp = None;
            }
            TextureFiltering::Anisotropic => {
                tex.max_mip_level = None;
                tex.sampler.mag_filter = FilterMode::Linear;
                tex.sampler.min_filter = FilterMode::Linear;
                tex.sampler.mipmap_filter = FilterMode::Linear;
                tex.sampler.anisotropy_clamp = NonZeroU8::new(16);
            }
        }
    }

    fn screen_text(&self) -> &'static str {
        match self {
            TextureFiltering::Nearest => "Nearest (no mipmaps)",
            TextureFiltering::Linear => "Linear (no mipmaps)",
            TextureFiltering::NearestMipmap => "Linear with nearest mipmap",
            TextureFiltering::LinearMipmap => "Linear with linear mipmaps",
            TextureFiltering::Anisotropic => "16x Anisotropic",
        }
    }

    fn cycle_next(&self) -> TextureFiltering {
        match self {
            TextureFiltering::Nearest => TextureFiltering::Anisotropic,
            TextureFiltering::Linear => TextureFiltering::Nearest,
            TextureFiltering::NearestMipmap => TextureFiltering::Linear,
            TextureFiltering::LinearMipmap => TextureFiltering::NearestMipmap,
            TextureFiltering::Anisotropic => TextureFiltering::LinearMipmap,
        }
    }
}

impl Default for TextureFiltering {
    fn default() -> TextureFiltering {
        TextureFiltering::Anisotropic
    }
}
