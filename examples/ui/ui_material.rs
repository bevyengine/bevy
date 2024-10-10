//! Demonstrates the use of [`UiMaterials`](UiMaterial) and how to change material values

use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/custom_ui_material.wgsl";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UiMaterialPlugin::<CustomUiMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, animate)
        .run();
}

fn setup(
    mut commands: Commands,
    mut ui_materials: ResMut<Assets<CustomUiMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera so we can see UI
    commands.spawn(Camera2d);

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            let banner_scale_factor = 0.5;
            parent.spawn(MaterialNodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(905.0 * banner_scale_factor),
                    height: Val::Px(363.0 * banner_scale_factor),
                    border: UiRect::all(Val::Px(10.)),
                    ..default()
                },
                material: UiMaterialHandle(ui_materials.add(CustomUiMaterial {
                    color: LinearRgba::WHITE.to_f32_array().into(),
                    slider: 0.5,
                    color_texture: asset_server.load("branding/banner.png"),
                    border_color: LinearRgba::WHITE.to_f32_array().into(),
                })),
                ..default()
            });
        });
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomUiMaterial {
    /// Color multiplied with the image
    #[uniform(0)]
    color: Vec4,
    /// Represents how much of the image is visible
    /// Goes from 0 to 1
    #[uniform(1)]
    slider: f32,
    /// Image used to represent the slider
    #[texture(2)]
    #[sampler(3)]
    color_texture: Handle<Image>,
    /// Color of the image's border
    #[uniform(4)]
    border_color: Vec4,
}

impl UiMaterial for CustomUiMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

// Fills the slider slowly over 2 seconds and resets it
// Also updates the color of the image to a rainbow color
fn animate(
    mut materials: ResMut<Assets<CustomUiMaterial>>,
    q: Query<&UiMaterialHandle<CustomUiMaterial>>,
    time: Res<Time>,
) {
    let duration = 2.0;
    for handle in &q {
        if let Some(material) = materials.get_mut(handle) {
            // rainbow color effect
            let new_color = Color::hsl((time.elapsed_seconds() * 60.0) % 360.0, 1., 0.5);
            let border_color = Color::hsl((time.elapsed_seconds() * 60.0) % 360.0, 0.75, 0.75);
            material.color = new_color.to_linear().to_vec4();
            material.slider =
                ((time.elapsed_seconds() % (duration * 2.0)) - duration).abs() / duration;
            material.border_color = border_color.to_linear().to_vec4();
        }
    }
}
