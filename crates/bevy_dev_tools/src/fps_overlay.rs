//! Module containing logic for FPS overlay.

use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{Assets, Handle};
use bevy_color::Color;
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    component::Component,
    query::With,
    schedule::{common_conditions::resource_changed, IntoSystemConfigs},
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_hierarchy::BuildChildren;
use bevy_text::{Font, Text, TextSection, TextStyle};
use bevy_ui::{
    node_bundles::{MaterialNodeBundle, NodeBundle, TextBundle},
    FlexDirection, PositionType, Style, Val, ZIndex,
};
use bevy_utils::default;

use crate::frame_time_graph::{
    FrameTimeGraphConfigUniform, FrameTimeGraphPlugin, FrametimeGraphMaterial,
};

/// Global [`ZIndex`] used to render the fps overlay.
///
/// We use a number slightly under `i32::MAX` so you can render on top of it if you really need to.
pub const FPS_OVERLAY_ZINDEX: i32 = i32::MAX - 32;

// Used to scale the frame time graph based on the fps text size
const FRAME_TIME_GRAPH_WIDTH_SCALE: f32 = 6.0;
const FRAME_TIME_GRAPH_HEIGHT_SCALE: f32 = 2.0;

/// A plugin that adds an FPS overlay to the Bevy application.
///
/// This plugin will add the [`FrameTimeDiagnosticsPlugin`] if it wasn't added before.
///
/// Note: It is recommended to use native overlay of rendering statistics when possible for lower overhead and more accurate results.
/// The correct way to do this will vary by platform:
/// - **Metal**: setting env variable `MTL_HUD_ENABLED=1`
#[derive(Default)]
pub struct FpsOverlayPlugin {
    /// Starting configuration of overlay, this can be later be changed through [`FpsOverlayConfig`] resource.
    pub config: FpsOverlayConfig,
}

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // TODO: Use plugin dependencies, see https://github.com/bevyengine/bevy/issues/69
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }

        if !app.is_plugin_added::<FrameTimeGraphPlugin>() {
            app.add_plugins(FrameTimeGraphPlugin);
        }

        app.insert_resource(self.config.clone())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    customize_overlay.run_if(resource_changed::<FpsOverlayConfig>),
                    update_text,
                ),
            );
    }
}

/// Configuration options for the FPS overlay.
#[derive(Resource, Clone)]
pub struct FpsOverlayConfig {
    /// Configuration of text in the overlay.
    pub text_config: TextStyle,
    /// Configuration of the frame time graph
    pub frame_time_graph_config: FrameTimeGraphConfig,
}

impl Default for FpsOverlayConfig {
    fn default() -> Self {
        FpsOverlayConfig {
            text_config: TextStyle {
                font: Handle::<Font>::default(),
                font_size: 32.0,
                color: Color::WHITE,
            },
            // TODO set this to display refresh rate if possible
            frame_time_graph_config: FrameTimeGraphConfig::target_fps(60.0),
        }
    }
}

/// Configuration of the frame time graph
#[derive(Clone, Copy)]
pub struct FrameTimeGraphConfig {
    /// Is the graph visible
    pub enabled: bool,
    /// The minimum acceptable FPS
    ///
    /// Anything bellow this will show a red bar
    pub min_fps: f32,
    /// The target FPS
    ///
    /// Anything above this will show a green bar
    pub target_fps: f32,
}

impl FrameTimeGraphConfig {
    /// Constructs a default config for a given target fps
    pub fn target_fps(target_fps: f32) -> Self {
        Self {
            target_fps,
            ..Self::default()
        }
    }
}

impl Default for FrameTimeGraphConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_fps: 30.0,
            target_fps: 60.0,
        }
    }
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct FrameTimeGraph;

fn setup(
    mut commands: Commands,
    overlay_config: Res<FpsOverlayConfig>,
    mut frame_time_graph_materials: ResMut<Assets<FrametimeGraphMaterial>>,
) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // We need to make sure the overlay doesn't affect the position of other UI nodes
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // Render overlay on top of everything
            z_index: ZIndex::Global(FPS_OVERLAY_ZINDEX),
            ..default()
        })
        .with_children(|c| {
            c.spawn((
                TextBundle::from_sections([
                    TextSection::new("FPS: ", overlay_config.text_config.clone()),
                    TextSection::from_style(overlay_config.text_config.clone()),
                ]),
                FpsText,
            ));
            if overlay_config.frame_time_graph_config.enabled {
                let font_size = overlay_config.text_config.font_size;
                c.spawn((
                    MaterialNodeBundle {
                        style: Style {
                            width: Val::Px(font_size * FRAME_TIME_GRAPH_WIDTH_SCALE),
                            height: Val::Px(font_size * FRAME_TIME_GRAPH_HEIGHT_SCALE),
                            ..default()
                        },
                        material: frame_time_graph_materials.add(FrametimeGraphMaterial {
                            values: vec![],
                            config: FrameTimeGraphConfigUniform::new(
                                overlay_config.frame_time_graph_config.target_fps,
                                overlay_config.frame_time_graph_config.min_fps,
                                true,
                            ),
                        }),
                        ..default()
                    },
                    FrameTimeGraph,
                ));
            }
        });
}

fn update_text(diagnostic: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[1].value = format!("{value:.2}");
            }
        }
    }
}

fn customize_overlay(
    overlay_config: Res<FpsOverlayConfig>,
    mut query: Query<&mut Text, With<FpsText>>,
    mut graph_style: Query<&mut Style, With<FrameTimeGraph>>,
) {
    for mut text in &mut query {
        for section in text.sections.iter_mut() {
            section.style = overlay_config.text_config.clone();
        }
    }

    if let Ok(mut graph_style) = graph_style.get_single_mut() {
        if overlay_config.frame_time_graph_config.enabled {
            // Scale the frame time graph based on the font size of the overlay
            let font_size = overlay_config.text_config.font_size;
            graph_style.width = Val::Px(font_size * FRAME_TIME_GRAPH_WIDTH_SCALE);
            graph_style.height = Val::Px(font_size * FRAME_TIME_GRAPH_HEIGHT_SCALE);

            graph_style.display = bevy_ui::Display::DEFAULT;
        } else {
            graph_style.display = bevy_ui::Display::None;
        }
    }
}
