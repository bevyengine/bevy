use benches::bench;
use bevy_image::{prelude::*, CompressedImageFormats, ImageSampler, ImageType};
use bevy_render::render_asset::RenderAssetUsages;
use core::hint::black_box;
use criterion::{criterion_group, Criterion};

criterion_group!(benches, ktx_decode);

fn ktx_decode(c: &mut Criterion) {
    let ktx2_uastc_buffer = include_bytes!("./assets/ktx2-uastc-srgb-mips.ktx2");
    let ktx2_zstd_uastc_buffer = include_bytes!("./assets/ktx2-zstd-uastc-srgb-mips.ktx2");
    let ktx2_rgba32_buffer = include_bytes!("./assets/ktx2-rgba32-mips.ktx2");

    // Decode once first to initialize decoder lookup tables
    Image::from_buffer(
        black_box(ktx2_zstd_uastc_buffer),
        ImageType::Extension("ktx2"),
        CompressedImageFormats::empty(),
        false,
        ImageSampler::Default,
        RenderAssetUsages::all(),
    )
    .unwrap();

    c.bench_function(bench!("raw_rgba32"), |b| {
        b.iter(|| {
            Image::from_buffer(
                black_box(ktx2_rgba32_buffer),
                ImageType::Extension("ktx2"),
                CompressedImageFormats::empty(),
                false,
                ImageSampler::Default,
                RenderAssetUsages::all(),
            )
            .unwrap();
        });
    });

    c.bench_function(bench!("uastc_decompress"), |b| {
        b.iter(|| {
            Image::from_buffer(
                black_box(ktx2_uastc_buffer),
                ImageType::Extension("ktx2"),
                CompressedImageFormats::empty(),
                false,
                ImageSampler::Default,
                RenderAssetUsages::all(),
            )
            .unwrap();
        });
    });

    c.bench_function(bench!("uastc_no_decompress"), |b| {
        b.iter(|| {
            Image::from_buffer(
                black_box(ktx2_uastc_buffer),
                ImageType::Extension("ktx2"),
                CompressedImageFormats::ASTC_LDR,
                false,
                ImageSampler::Default,
                RenderAssetUsages::all(),
            )
            .unwrap();
        });
    });

    c.bench_function(bench!("zstd_uastc_decompress"), |b| {
        b.iter(|| {
            Image::from_buffer(
                black_box(ktx2_zstd_uastc_buffer),
                ImageType::Extension("ktx2"),
                CompressedImageFormats::empty(),
                false,
                ImageSampler::Default,
                RenderAssetUsages::all(),
            )
            .unwrap();
        });
    });

    c.bench_function(bench!("zstd_uastc_no_decompress"), |b| {
        b.iter(|| {
            Image::from_buffer(
                black_box(ktx2_zstd_uastc_buffer),
                ImageType::Extension("ktx2"),
                CompressedImageFormats::ASTC_LDR,
                false,
                ImageSampler::Default,
                RenderAssetUsages::all(),
            )
            .unwrap();
        });
    });
}
