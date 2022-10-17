//! Uses two windows to visualize a 3D model from different angles.

use std::f32::consts::PI;

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        camera::RenderTarget,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{PrepareAssetLabel, RenderAssets},
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::{ExtractedWindows, RenderLayers, WindowSystem},
        RenderApp, RenderStage,
    },
    window::{PresentMode, WindowId},
};

#[derive(Clone, Resource)]
struct WindowTexture {
    window_id: WindowId,
    render_texture: Handle<Image>,
}

impl ExtractResource for WindowTexture {
    type Source = WindowTexture;

    fn extract_resource(source: &WindowTexture) -> Self {
        source.clone()
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(bevy::window::close_on_esc)
        .add_plugin(ExtractResourcePlugin::<WindowTexture>::default());
    if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_system_to_stage(
            RenderStage::Prepare,
            prepare_window_texture
                .after(PrepareAssetLabel::AssetPrepare)
                .before(WindowSystem::Prepare),
        );
    }
    app.run();
}

fn prepare_window_texture(
    window_texture: Res<WindowTexture>,
    gpu_images: Res<RenderAssets<Image>>,
    mut extracted_windows: ResMut<ExtractedWindows>,
) {
    if let Some(window) = extracted_windows.get_mut(&window_texture.window_id) {
        window.swap_chain_texture = Some(
            gpu_images
                .get(&window_texture.render_texture)
                .unwrap()
                .texture_view
                .clone(),
        );
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut windows: ResMut<Windows>,
) {
    let window_id = WindowId::new();
    windows.add(Window::new_virtual(
        window_id,
        &WindowDescriptor {
            width: 800.,
            height: 600.,
            present_mode: PresentMode::AutoNoVsync,
            title: "Second window".to_string(),
            ..default()
        },
        800,
        600,
        1.0,
        None,
    ));

    let size = Extent3d {
        width: 800,
        height: 600,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);
    commands.insert_resource(WindowTexture {
        window_id,
        render_texture: image_handle.clone(),
    });

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 4.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // The cube that will be rendered to the texture.
    commands.spawn((
        PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..default()
        },
        first_pass_layer,
    ));

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
            camera: Camera {
                // render before the "main pass" camera
                priority: -1,
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        first_pass_layer,
    ));

    let cube_size = 4.0;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // Main pass cube, with material containing the rendered first pass texture.
    commands.spawn(PbrBundle {
        mesh: cube_handle,
        material: material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 1.5)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // The main pass camera.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let window_id = WindowId::new();
    windows.add(Window::new_virtual(
        window_id,
        &WindowDescriptor {
            width: 800.,
            height: 600.,
            present_mode: PresentMode::AutoNoVsync,
            title: "Second window".to_string(),
            ..default()
        },
        800,
        600,
        1.0,
        None,
    ));
}
