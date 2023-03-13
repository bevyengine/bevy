use bevy_app::{App, Plugin};
use bevy_asset::AssetServer;
use bevy_ecs::{
    prelude::Component,
    query::With,
    schedule::{IntoSystemConfig, IntoSystemConfigs},
    system::{Commands, Query, Res, ResMut, Resource},
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
    AlignItems, Size, Style, UiRect, Val,
};
use bevy_utils::{default, Duration, HashMap};

pub struct DebugOverlaysPlugin;

impl Plugin for DebugOverlaysPlugin {
    fn build(&self, app: &mut App) {
        let wgpu_features = app.world.resource::<RenderDevice>().features();
        if !wgpu_features.contains(WgpuFeatures::TIMESTAMP_QUERY) {
            panic!("DebugOverlaysPlugin added but RenderPlugin::wgpu_settings did not contain WgpuFeatures::TIMESTAMP_QUERY.");
        }

        app.init_resource::<AggregatedGpuTimers>()
            .add_startup_system(setup_ui)
            .add_systems(
                (
                    aggregate_gpu_timers.run_if(on_timer(Duration::from_secs(1))),
                    draw_node_gpu_time_overlay,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
struct GpuTimerOverlayUIMarker;

fn setup_ui(mut commands: Commands) {
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
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((GpuTimerOverlayUIMarker, TextBundle::default()));
                });
        });
}

#[derive(Resource, Default)]
struct AggregatedGpuTimers(HashMap<String, f64>);

// TODO: Handle nesting
fn aggregate_gpu_timers(
    gpu_timers: Res<GpuTimerScopes>,
    mut aggregated_gpu_timers: ResMut<AggregatedGpuTimers>,
) {
    let mut stack = gpu_timers.take();
    while let Some(gpu_timer) = stack.pop() {
        let average = aggregated_gpu_timers.0.entry(gpu_timer.label).or_default();
        let duration = gpu_timer.time.end - gpu_timer.time.start;
        *average = (*average * 0.1) + (duration * 0.9);

        for gpu_timer in gpu_timer.nested_scopes {
            stack.push(gpu_timer);
        }
    }
}

fn draw_node_gpu_time_overlay(
    aggregated_gpu_timers: Res<AggregatedGpuTimers>,
    asset_server: Res<AssetServer>,
    mut ui: Query<&mut Text, With<GpuTimerOverlayUIMarker>>,
) {
    let mut gpu_timers = aggregated_gpu_timers.0.iter().collect::<Vec<_>>();
    gpu_timers.sort_unstable_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap().reverse());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 12.0,
        color: Color::WHITE,
    };

    let mut ui = ui.single_mut();
    ui.sections.clear();
    ui.sections.push(TextSection::new(
        "GPU Time\n",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 16.0,
            color: Color::WHITE,
        },
    ));

    for (label, duration) in gpu_timers {
        let label = match label.rsplit_once("::") {
            Some((_, label)) => label,
            None => label,
        };
        let label = label.strip_suffix("Node").unwrap_or(label);
        let text = format!("{label}: {:.3}ms\n", *duration * 1000.0);
        ui.sections.push(TextSection::new(text, text_style.clone()));
    }
}
