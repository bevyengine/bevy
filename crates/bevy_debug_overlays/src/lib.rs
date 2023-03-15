mod aggregation;

use aggregation::{aggregate_gpu_timers, AggregatedGpuTimers};
use bevy_app::{App, Plugin};
use bevy_asset::AssetServer;
use bevy_ecs::{
    prelude::Component,
    query::{With, Without},
    schedule::{IntoSystemConfig, IntoSystemConfigs},
    system::{Commands, Query, Res},
};
use bevy_hierarchy::BuildChildren;
use bevy_render::{prelude::Color, render_resource::WgpuFeatures, renderer::RenderDevice};
use bevy_text::{Text, TextSection, TextStyle};
use bevy_time::common_conditions::on_timer;
use bevy_ui::{
    prelude::{NodeBundle, TextBundle},
    AlignItems, FlexDirection, Size, Style, UiRect, Val,
};
use bevy_utils::{default, Duration};
use std::fmt::Write;

pub struct DebugOverlaysPlugin;

impl Plugin for DebugOverlaysPlugin {
    fn build(&self, app: &mut App) {
        let wgpu_features = app.world.resource::<RenderDevice>().features();
        if !wgpu_features.contains(WgpuFeatures::TIMESTAMP_QUERY) {
            panic!("DebugOverlaysPlugin added but RenderPlugin::wgpu_settings did not contain WgpuFeatures::TIMESTAMP_QUERY.");
        }

        app.init_resource::<AggregatedGpuTimers>()
            .add_startup_system(setup_gpu_time_overlay)
            .add_systems(
                (
                    aggregate_gpu_timers.run_if(on_timer(Duration::from_millis(300))),
                    draw_gpu_time_overlay,
                )
                    .chain(),
            );
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
    aggregated_gpu_timers: Res<AggregatedGpuTimers>,
    mut labels: Query<&mut Text, (With<GpuTimerLabelMarker>, Without<GpuTimerDurationMarker>)>,
    mut durations: Query<&mut Text, (With<GpuTimerDurationMarker>, Without<GpuTimerLabelMarker>)>,
) {
    let mut gpu_timers = aggregated_gpu_timers.0.iter().collect::<Vec<_>>();
    gpu_timers.sort_unstable_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap().reverse());

    let mut labels = labels.single_mut();
    let mut durations = durations.single_mut();
    let labels = &mut labels.sections[0].value;
    let durations = &mut durations.sections[0].value;
    labels.clear();
    durations.clear();

    for (label, duration) in gpu_timers {
        let duration_ms = *duration * 1000.0;
        write!(labels, "{label}: \n").unwrap();
        write!(durations, "{duration_ms:.3}\n").unwrap();
    }
}
