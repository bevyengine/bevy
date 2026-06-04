use bevy_app::{App, AppLabel};
use bevy_asset::{Asset, AssetApp, AssetEvent, AssetId, Assets, RenderAssetUsages};
use bevy_ecs::prelude::*;
use bevy_reflect::TypePath;
use bevy_render::{
    extract_plugin::ExtractPlugin,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    RenderApp,
};
use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use std::time::{Duration, Instant};

#[derive(Asset, TypePath, Clone, Debug)]
struct DummyAsset;

struct DummyRenderAsset;

impl RenderAsset for DummyRenderAsset {
    type SourceAsset = DummyAsset;
    type Param = ();

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        _source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        _param: &mut bevy_ecs::system::SystemParamItem<Self::Param>,
        _previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(DummyRenderAsset)
    }
}

fn extract_render_asset_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_render_asset");

    // Test different payload sizes
    for size in [10, 100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("allocations", size), &size, |b, &size| {
            // --- ONCE PER BENCHMARK SCALE ---
            let mut app = App::new();

            app.add_plugins(bevy_asset::AssetPlugin::default());
            app.init_asset::<DummyAsset>();
            app.add_plugins(ExtractPlugin::default());
            app.add_plugins(RenderAssetPlugin::<DummyRenderAsset>::default());

            app.finish();
            app.cleanup();

            let mut handles = Vec::with_capacity(size);
            {
                let mut assets = app.world_mut().resource_mut::<Assets<DummyAsset>>();
                for _ in 0..size {
                    handles.push(assets.add(DummyAsset));
                }
            }

            // Run one initial update to flush any startup systems
            app.update();

            b.iter_custom(|iters| {
                let mut total = Duration::default();

                for _ in 0..iters {
                    // Send N messages to trigger extraction logic
                    for handle in &handles {
                        app.world_mut()
                            .write_message(AssetEvent::Modified { id: handle.id() });
                    }

                    let bevy_app::SubApps { main, sub_apps } = app.sub_apps_mut();
                    let render_app = sub_apps
                        .get_mut(&RenderApp.intern())
                        .expect("RenderApp should exist");
                    let render_world = render_app.world_mut();

                    // Measuring the extract call
                    let start = Instant::now();
                    bevy_render::extract_plugin::extract(main.world_mut(), render_world);
                    total += Instant::now().duration_since(start);

                    // Run a standard app update to allow Bevy's internal systems to flush/clear the message queues.
                    app.update();
                }
                total
            });
        });
    }
    group.finish();
}

criterion_group!(benches, extract_render_asset_bench);
