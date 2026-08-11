//! Showcases wireframe rendering for 2d meshes.
//!
//! Wireframes currently do not work when using webgl or webgpu.
//! Supported platforms:
//! - DX12
//! - Vulkan
//! - Metal
//!
//! This is a native only feature.

use bevy::{
    color::palettes::basic::{GREEN, RED, WHITE},
    feathers::{theme::UiTheme, FeathersPlugins},
    prelude::*,
    render::{render_resource::WgpuFeatures, settings::WgpuSettings, RenderPlugin},
    sprite_render::{
        NoWireframe2d, Wireframe2d, Wireframe2dColor, Wireframe2dConfig, Wireframe2dPlugin,
    },
    ui_widgets::{radio_self_update, ValueChange},
};

use radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }
                .into(),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            Wireframe2dPlugin::default(),
            FeathersPlugins,
        ))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        .insert_resource(Wireframe2dConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe2d`. Meshes with `Wireframe2d` will always have a wireframe,
            // regardless of the global configuration.
            global: true,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `Wireframe2dColor` component.
            default_color: WHITE.into(),
        })
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .add_systems(Startup, setup)
        .add_observer(update_radio_button)
        .add_observer(radio_self_update)
        .run();
}

/// Whether the global wireframe setting is on or off.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum GlobalWireframeSetting {
    #[default]
    On,
    Off,
}

/// Whether the global color setting is red or white.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum GlobalColorSetting {
    #[default]
    Red,
    White,
}

/// Whether the color of the circle wireframe is red or green.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum ColorCircleWireframeSetting {
    #[default]
    Red,
    Green,
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Children [
            (
                feathers_option_buttons("Toggle global",
                    &[
                        (GlobalWireframeSetting::On, "ON"),
                        (GlobalWireframeSetting::Off, "OFF"),
                    ], 0)
            ),
            (
                feathers_option_buttons("Change global color",
                    &[
                        (GlobalColorSetting::Red, "RED"),
                        (GlobalColorSetting::White, "WHITE"),
                    ], 1)
            ),
            (
                feathers_option_buttons("Change color of the circle wireframe",
                    &[
                        (ColorCircleWireframeSetting::Red, "RED"),
                        (ColorCircleWireframeSetting::Green, "GREEN"),
                    ], 1)
            ),
        ]
    });
    // Triangle: Never renders a wireframe
    commands.spawn((
        Mesh2d(meshes.add(Triangle2d::new(
            Vec2::new(0.0, 50.0),
            Vec2::new(-50.0, -50.0),
            Vec2::new(50.0, -50.0),
        ))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_xyz(-150.0, 0.0, 0.0),
        NoWireframe2d,
    ));
    // Rectangle: Follows global wireframe setting
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(100.0, 100.0))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    // Circle: Always renders a wireframe
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(50.0))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_xyz(150.0, 0.0, 0.0),
        Wireframe2d,
        // This lets you configure the wireframe color of this entity.
        // If not set, this will use the color in `WireframeConfig`
        Wireframe2dColor {
            color: GREEN.into(),
        },
    ));

    commands.spawn(Camera2d);
}

/// This system lets you toggle various wireframe settings
fn update_radio_button(
    event: On<ValueChange<Entity>>,
    toggle_global_query: Query<&RadioButtonOptionValue<GlobalWireframeSetting>>,
    global_color_query: Query<&RadioButtonOptionValue<GlobalColorSetting>>,
    color_circle_query: Query<&RadioButtonOptionValue<ColorCircleWireframeSetting>>,
    mut config: ResMut<Wireframe2dConfig>,
    mut wireframe_colors: Query<&mut Wireframe2dColor>,
) {
    // Toggle showing a wireframe on all meshes
    if let Ok(RadioButtonOptionValue(toggle_global)) = toggle_global_query.get(event.value) {
        config.global = matches!(toggle_global, GlobalWireframeSetting::On);
    }

    // Toggle the global wireframe color
    if let Ok(RadioButtonOptionValue(global_color)) = global_color_query.get(event.value) {
        config.default_color = match global_color {
            GlobalColorSetting::Red => RED.into(),
            GlobalColorSetting::White => WHITE.into(),
        };
    }

    // Toggle the color of a wireframe using `Wireframe2dColor` and not the global color
    if let Ok(RadioButtonOptionValue(color_circle)) = color_circle_query.get(event.value) {
        for mut color in &mut wireframe_colors {
            color.color = match color_circle {
                ColorCircleWireframeSetting::Red => RED.into(),
                ColorCircleWireframeSetting::Green => GREEN.into(),
            };
        }
    }
}
