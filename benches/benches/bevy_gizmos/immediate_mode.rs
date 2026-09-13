use benches::bench;
use bevy_app::{App, Update};
use bevy_asset::AssetPlugin;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_gizmos::{prelude::Gizmos, GizmoPlugin};
use bevy_math::Vec3;
use core::time::Duration;
use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};

criterion_group!(benches, system_count_scaling);

#[derive(Resource)]
struct LinesPerSystem(usize);

fn draw_lines(mut gizmos: Gizmos, lines_per_system: Res<LinesPerSystem>) {
    for _ in 0..lines_per_system.0 {
        gizmos.line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Color::WHITE,
        );
    }
}

fn build_app(system_count: usize, lines_per_system: usize) -> App {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(GizmoPlugin);
    app.insert_resource(LinesPerSystem(lines_per_system));

    for _ in 0..system_count {
        app.add_systems(Update, draw_lines);
    }

    app.finish();
    app.cleanup();
    app
}

// Measures the impact of having multiple systems drawing gizmos
fn system_count_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("system_count_scaling"));
    group.warm_up_time(Duration::from_millis(1000));
    group.measurement_time(Duration::from_secs(10));

    for total_lines in [10_000usize, 100_000] {
        group.throughput(Throughput::Elements(total_lines as u64));

        for system_count in [1usize, 10, 100] {
            let lines_per_system = total_lines / system_count;
            group.bench_with_input(
                BenchmarkId::new(total_lines.to_string(), system_count),
                &(system_count, lines_per_system),
                |b, &(system_count, lines_per_system)| {
                    let mut app = build_app(system_count, lines_per_system);
                    app.update();

                    b.iter(|| {
                        app.update();
                    });
                },
            );
        }
    }

    group.finish();
}
