//! Demonstrates the use of [`UiMaterials`](UiMaterial) and how to change material values

use bevy::{
    color::palettes::css::DARK_BLUE, prelude::*, reflect::TypePath, render::render_resource::*,
    shader::ShaderRef,
};

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
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            let banner_scale_factor = 0.5;
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: px(905.0 * banner_scale_factor),
                    height: px(363.0 * banner_scale_factor),
                    border: UiRect::all(px(20)),
                    ..default()
                },
                MaterialNode(ui_materials.add(CustomUiMaterial {
                    color: LinearRgba::WHITE.to_f32_array().into(),
                    slider: Vec4::splat(0.5),
                    color_texture: asset_server.load("branding/banner.png"),
                    border_color: LinearRgba::WHITE.to_f32_array().into(),
                })),
                BorderRadius::all(px(20)),
                // UI material nodes can have outlines and shadows like any other UI node
                Outline {
                    width: px(2),
                    offset: px(100),
                    color: DARK_BLUE.into(),
                },
            ));
        });
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomUiMaterial {
    /// Color multiplied with the image
    #[uniform(0)]
    color: Vec4,
    /// Represents how much of the image is visible
    /// Goes from 0 to 1
    /// A `Vec4` is used here because Bevy with webgl2 requires that uniforms are 16-byte aligned but only the first component is read.
    #[uniform(1)]
    slider: Vec4,
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
    q: Query<&MaterialNode<CustomUiMaterial>>,
    time: Res<Time>,
) {
    let duration = 2.0;
    for handle in &q {
        if let Some(material) = materials.get_mut(handle) {
            // rainbow color effect
            let new_color = Color::hsl((time.elapsed_secs() * 60.0) % 360.0, 1., 0.5);
            let border_color = Color::hsl((time.elapsed_secs() * 60.0) % 360.0, 0.75, 0.75);
            material.color = new_color.to_linear().to_vec4();
            material.slider.x =
                ((time.elapsed_secs() % (duration * 2.0)) - duration).abs() / duration;
            material.border_color = border_color.to_linear().to_vec4();
        }
    }
}
