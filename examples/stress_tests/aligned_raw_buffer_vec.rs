use std::time::Duration;
use std::time::Instant;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_render::{
    render_resource::{AlignedRawBufferVec, BufferUsages, DynamicUniformBuffer, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bytemuck::NoUninit;

fn main() {
    App::new()
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            DefaultPlugins,
            BenchPlugin,
        ))
        .run();
}

struct BenchPlugin;
impl Plugin for BenchPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<BenchUniforms>();
            render_app.add_systems(
                Render,
                bench_prepare_buffers.in_set(RenderSet::PrepareResources),
            );
        }
    }
}

#[derive(Clone, Copy, NoUninit)]
#[repr(C)]
struct BenchUniform {
    a: Vec3,
    b: f32,

    c: Vec4,

    d: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}

#[derive(ShaderType)]
struct BenchUniformShaderType {
    a: Vec3,
    b: f32,
    c: Vec4,
    d: f32,
}

#[derive(Resource)]
struct BenchUniforms {
    buffer_vec: AlignedRawBufferVec<BenchUniform>,
    uniform_buffer: DynamicUniformBuffer<BenchUniformShaderType>,
}
impl FromWorld for BenchUniforms {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self {
            buffer_vec: AlignedRawBufferVec::new(BufferUsages::UNIFORM, render_device),
            uniform_buffer: DynamicUniformBuffer::default(),
        }
    }
}

fn bench_prepare_buffers(
    mut bench_uniforms: ResMut<BenchUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let capacity = 100_000;
    println!("\nprepare buffers with {} items", capacity);

    let start = Instant::now();
    bench_uniforms.buffer_vec.clear();
    bench_uniforms.buffer_vec.reserve(capacity, &render_device);
    for _ in 0..capacity {
        bench_uniforms.buffer_vec.push(BenchUniform {
            a: Vec3::ONE,
            b: 1.0,
            c: Vec4::ONE,
            d: 1.0,
            pad0: 0.0,
            pad1: 0.0,
            pad2: 0.0,
        });
    }
    bench_uniforms
        .buffer_vec
        .write_buffer(&render_device, &render_queue);
    let elapsed = start.elapsed();
    println!("AlignedRawBufferVec_push: {} micros", elapsed.as_micros());

    let start = Instant::now();
    {
        let mut writer = bench_uniforms
            .buffer_vec
            .get_writer(capacity, &render_device, &render_queue)
            .unwrap();
        for _ in 0..capacity {
            writer.write(BenchUniform {
                a: Vec3::ONE,
                b: 1.0,
                c: Vec4::ONE,
                d: 1.0,
                pad0: 0.0,
                pad1: 0.0,
                pad2: 0.0,
            });
        }
    }
    let elapsed = start.elapsed();
    println!("AlignedRawBufferVec_writer: {} micros", elapsed.as_micros());

    let start = Instant::now();
    {
        bench_uniforms.uniform_buffer.clear();
        for _ in 0..capacity {
            bench_uniforms.uniform_buffer.push(&BenchUniformShaderType {
                a: Vec3::ONE,
                b: 1.0,
                c: Vec4::ONE,
                d: 1.0,
            });
        }
        bench_uniforms
            .uniform_buffer
            .write_buffer(&render_device, &render_queue);
    }
    let elapsed = start.elapsed();
    println!("DynamicUniformBuffer_push: {} micros", elapsed.as_micros());

    let start = Instant::now();
    {
        let mut writer = bench_uniforms
            .uniform_buffer
            .get_writer(capacity, &render_device, &render_queue)
            .unwrap();
        for _ in 0..capacity {
            writer.write(&BenchUniformShaderType {
                a: Vec3::ONE,
                b: 1.0,
                c: Vec4::ONE,
                d: 1.0,
            });
        }
    }
    let elapsed = start.elapsed();
    println!(
        "DynamicUniformBuffer_writer: {} micros",
        elapsed.as_micros()
    );
}
