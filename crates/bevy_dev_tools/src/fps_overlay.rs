//! Module containing logic for FPS overlay.

use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{Assets, Handle};
use bevy_color::Color;
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Local,
    query::{With, Without},
    resource::Resource,
    schedule::{common_conditions::resource_changed, IntoScheduleConfigs},
    system::{Commands, Query, Res, ResMut, Single},
};
use bevy_picking::Pickable;
use bevy_render::storage::ShaderStorageBuffer;
use bevy_text::{Font, TextColor, TextFont, TextSpan};
use bevy_time::Time;
use bevy_ui::{
    widget::{Text, TextUiWriter},
    FlexDirection, GlobalZIndex, Node, PositionType, Val,
};
use bevy_ui_render::prelude::MaterialNode;
use core::time::Duration;

use crate::frame_time_graph::{
    FrameTimeGraphConfigUniform, FrameTimeGraphPlugin, FrametimeGraphMaterial,
};

/// [`GlobalZIndex`] used to render the fps overlay.
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
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        if !app.is_plugin_added::<FrameTimeGraphPlugin>() {
            app.add_plugins(FrameTimeGraphPlugin);
        }

        app.insert_resource(self.config.clone())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (toggle_display, customize_overlay)
                        .run_if(resource_changed::<FpsOverlayConfig>),
                    update_text,
                ),
            );
    }
}

/// Configuration options for the FPS overlay.
#[derive(Resource, Clone)]
pub struct FpsOverlayConfig {
    /// Configuration of text in the overlay.
    pub text_config: TextFont,
    /// Color of text in the overlay.
    pub text_color: Color,
    /// Displays the FPS overlay if true.
    pub enabled: bool,
    /// The period after which the FPS overlay re-renders.
    ///
    /// Defaults to once every 100 ms.
    pub refresh_interval: Duration,
    /// Configuration of the frame time graph
    pub frame_time_graph_config: FrameTimeGraphConfig,
}

impl Default for FpsOverlayConfig {
    fn default() -> Self {
        FpsOverlayConfig {
            text_config: TextFont {
                font: Handle::<Font>::default(),
                font_size: 32.0,
                ..Default::default()
            },
            text_color: Color::WHITE,
            enabled: true,
            refresh_interval: Duration::from_millis(100),
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
    /// Anything below this will show a red bar
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
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands
        .spawn((
            Node {
                // We need to make sure the overlay doesn't affect the position of other UI nodes
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            // Render overlay on top of everything
            GlobalZIndex(FPS_OVERLAY_ZINDEX),
            Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("FPS: "),
                overlay_config.text_config.clone(),
                TextColor(overlay_config.text_color),
                FpsText,
                Pickable::IGNORE,
            ))
            .with_child((TextSpan::default(), overlay_config.text_config.clone()));

            let font_size = overlay_config.text_config.font_size;
            p.spawn((
                Node {
                    width: Val::Px(font_size * FRAME_TIME_GRAPH_WIDTH_SCALE),
                    height: Val::Px(font_size * FRAME_TIME_GRAPH_HEIGHT_SCALE),
                    display: if overlay_config.frame_time_graph_config.enabled {
                        bevy_ui::Display::DEFAULT
                    } else {
                        bevy_ui::Display::None
                    },
                    ..Default::default()
                },
                Pickable::IGNORE,
                MaterialNode::from(frame_time_graph_materials.add(FrametimeGraphMaterial {
                    values: buffers.add(ShaderStorageBuffer {
                        // Initialize with dummy data because the default (`data: None`) will
                        // cause a panic in the shader if the frame time graph is constructed
                        // with `enabled: false`.
                        data: Some(vec![0, 0, 0, 0]),
                        ..Default::default()
                    }),
                    config: FrameTimeGraphConfigUniform::new(
                        overlay_config.frame_time_graph_config.target_fps,
                        overlay_config.frame_time_graph_config.min_fps,
                        true,
                    ),
                })),
                FrameTimeGraph,
            ));
        });
}

fn update_text(
    diagnostic: Res<DiagnosticsStore>,
    query: Query<Entity, With<FpsText>>,
    mut writer: TextUiWriter,
    time: Res<Time>,
    config: Res<FpsOverlayConfig>,
    mut time_since_rerender: Local<Duration>,
) {
    *time_since_rerender += time.delta();
    if *time_since_rerender >= config.refresh_interval {
        *time_since_rerender = Duration::ZERO;
        for entity in &query {
            if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS)
                && let Some(value) = fps.smoothed()
            {
                *writer.text(entity, 1) = format!("{value:.2}");
            }
        }
    }
}

fn customize_overlay(
    overlay_config: Res<FpsOverlayConfig>,
    query: Query<Entity, With<FpsText>>,
    mut writer: TextUiWriter,
) {
    for entity in &query {
        writer.for_each_font(entity, |mut font| {
            *font = overlay_config.text_config.clone();
        });
        writer.for_each_color(entity, |mut color| color.0 = overlay_config.text_color);
    }
}

fn toggle_display(
    overlay_config: Res<FpsOverlayConfig>,
    mut text_node: Single<&mut Node, (With<FpsText>, Without<FrameTimeGraph>)>,
    mut graph_node: Single<&mut Node, (With<FrameTimeGraph>, Without<FpsText>)>,
) {
    if overlay_config.enabled {
        text_node.display = bevy_ui::Display::DEFAULT;
    } else {
        text_node.display = bevy_ui::Display::None;
    }

    if overlay_config.frame_time_graph_config.enabled {
        // Scale the frame time graph based on the font size of the overlay
        let font_size = overlay_config.text_config.font_size;
        graph_node.width = Val::Px(font_size * FRAME_TIME_GRAPH_WIDTH_SCALE);
        graph_node.height = Val::Px(font_size * FRAME_TIME_GRAPH_HEIGHT_SCALE);

        graph_node.display = bevy_ui::Display::DEFAULT;
    } else {
        graph_node.display = bevy_ui::Display::None;
    }
}
