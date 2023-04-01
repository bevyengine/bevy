use bevy_app::{App, Plugin, Startup, Update};
use bevy_asset::AssetServer;
use bevy_ecs::{
    prelude::Component,
    query::{With, Without},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res},
};
use bevy_hierarchy::BuildChildren;
use bevy_render::{
    prelude::Color,
    render_resource::WgpuFeatures,
    renderer::{GpuTimerScopes, RenderDevice},
};
use bevy_text::{Text, TextSection, TextStyle};
use bevy_time::common_conditions::on_timer;
use bevy_ui::{
    prelude::{NodeBundle, TextBundle},
    AlignItems, FlexDirection, Size, Style, UiRect, Val,
};
use bevy_utils::{default, Duration};
use std::{env, fmt::Write, ops::Div};

/// Displays an overlay showing how long GPU operations take.
///
/// # Setup
///
/// To time GPU operation(s), wrap them in [`bevy_render::renderer::RenderContext`]`::begin_debug_scope()` and `end_debug_scope()`.
///
/// Ensure you add [`WgpuFeatures`]`::TIMESTAMP_QUERY` to `RenderPlugin::wgpu_settings`.
///
/// Ensure you add this plugin after `DefaultPlugins`.
///
/// # Stability
///
/// To ensure you get accurate results, try to close all unnecessary programs and background tasks.
///
/// To ensure you get stable results, run your program through the appropriate GPU locking script.
/// Example: `./tools/nvidia-gpu-lock.sh cargo run --example 3d_scene`.
pub struct DebugOverlaysPlugin {
    pub ui_update_frequency: Duration,
}

impl Plugin for DebugOverlaysPlugin {
    fn build(&self, app: &mut App) {
        let wgpu_features = app.world.resource::<RenderDevice>().features();
        if !wgpu_features.contains(WgpuFeatures::TIMESTAMP_QUERY) {
            panic!("DebugOverlaysPlugin added but RenderPlugin::wgpu_settings did not contain WgpuFeatures::TIMESTAMP_QUERY.");
        }

        if env::var("GPU_LOCKED").unwrap_or_default() != "magic" {
            panic!("GPU clocks were not locked. Unable to get stable profiling results. Please rerun via the locking script.")
        }

        app.add_systems(Startup, setup_gpu_time_overlay)
            .add_systems(
                Update,
                draw_gpu_time_overlay.run_if(on_timer(self.ui_update_frequency)),
            );
    }
}

impl Default for DebugOverlaysPlugin {
    fn default() -> Self {
        Self {
            ui_update_frequency: Duration::from_millis(100),
        }
    }
}

#[derive(Component)]
struct GpuTimerLabelMarker;

#[derive(Component)]
struct GpuTimerDurationMarker;

fn setup_gpu_time_overlay(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::width(Val::Percent(100.0)),
                align_items: AlignItems::Start,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    background_color: Color::rgba(0.10, 0.10, 0.10, 0.8).into(),
                    style: Style {
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_sections([
                        TextSection::new(
                            "GPU Time",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 16.0,
                                color: Color::WHITE,
                            },
                        ),
                        TextSection::new(
                            " (ms)\n",
                            TextStyle {
                                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                font_size: 10.0,
                                color: Color::WHITE,
                            },
                        ),
                    ]));
                    parent.spawn(NodeBundle::default()).with_children(|parent| {
                        let style = TextStyle {
                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                            font_size: 12.0,
                            color: Color::WHITE,
                        };
                        parent.spawn((
                            GpuTimerLabelMarker,
                            TextBundle::from_section("", style.clone()),
                        ));
                        parent.spawn((GpuTimerDurationMarker, TextBundle::from_section("", style)));
                    });
                });
        });
}

fn draw_gpu_time_overlay(
    gpu_timers: Res<GpuTimerScopes>,
    mut labels: Query<&mut Text, (With<GpuTimerLabelMarker>, Without<GpuTimerDurationMarker>)>,
    mut durations: Query<&mut Text, (With<GpuTimerDurationMarker>, Without<GpuTimerLabelMarker>)>,
) {
    let mut labels = labels.single_mut();
    let mut durations = durations.single_mut();
    let labels = &mut labels.sections[0].value;
    let durations = &mut durations.sections[0].value;
    labels.clear();
    durations.clear();

    let gpu_timers = gpu_timers.get();
    let mut aggregated_timers: Vec<AggregatedGpuTimer> = Vec::new();

    for frame in gpu_timers.iter() {
        for timer in frame {
            let timer_duration = timer.time.end - timer.time.start;
            match aggregated_timers
                .iter_mut()
                .find(|a| a.label == timer.label)
            {
                Some(a) => {
                    a.mean_duration += timer_duration / 20.0;
                    a.durations.push(timer_duration);
                }
                None => aggregated_timers.push(AggregatedGpuTimer {
                    label: &timer.label,
                    mean_duration: timer_duration / 20.0,
                    durations: vec![timer_duration],
                    // nested: Vec::new(),
                }),
            }
        }
    }

    aggregated_timers.sort_unstable_by(|a1, a2| {
        a1.mean_duration
            .partial_cmp(&a2.mean_duration)
            .unwrap()
            .reverse()
    });

    for timer in aggregated_timers {
        let std_dev = timer
            .durations
            .iter()
            .map(|d| (d - timer.mean_duration).powf(2.0))
            .sum::<f64>()
            .div(20.0)
            .sqrt();

        writeln!(labels, "{}  ", timer.label).unwrap();
        writeln!(
            durations,
            "{:.3} (±{:.2})",
            timer.mean_duration * 1000.0,
            std_dev * 1000.0
        )
        .unwrap();
    }
}

struct AggregatedGpuTimer<'a> {
    label: &'a str,
    mean_duration: f64,
    durations: Vec<f64>,
    // nested: Vec<Self>,
}
