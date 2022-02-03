use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_crevice::glsl::{Glsl, GlslStruct};
use bevy_crevice::std140::{AsStd140, Std140};
use bevy_crevice::std430::{AsStd430, Std430};
use futures::executor::block_on;
use wgpu::util::DeviceExt;

const BASE_SHADER: &str = "#version 450

{struct_definition}

layout({layout}, set = 0, binding = 0) readonly buffer INPUT {
    {struct_name} in_data;
};

layout({layout}, set = 0, binding = 1) buffer OUTPUT {
    {struct_name} out_data;
};

void main() {
    out_data = in_data;
}";

pub fn test_round_trip_struct<T: Debug + PartialEq + AsStd140 + AsStd430 + GlslStruct>(value: T) {
    let shader_std140 = glsl_shader_for_struct::<T>("std140");
    let shader_std430 = glsl_shader_for_struct::<T>("std430");

    let context = Context::new();
    context.test_round_trip_std140(&shader_std140, &value);
    context.test_round_trip_std430(&shader_std430, &value);
}

pub fn test_round_trip_primitive<T: Debug + PartialEq + AsStd140 + AsStd430 + Glsl>(value: T) {
    let shader_std140 = glsl_shader_for_primitive::<T>("std140");
    let shader_std430 = glsl_shader_for_primitive::<T>("std430");

    let context = Context::new();
    context.test_round_trip_std140(&shader_std140, &value);
    context.test_round_trip_std430(&shader_std430, &value);
}

fn glsl_shader_for_struct<T: GlslStruct>(layout: &str) -> String {
    BASE_SHADER
        .replace("{struct_name}", T::NAME)
        .replace("{struct_definition}", &T::glsl_definition())
        .replace("{layout}", layout)
}

fn glsl_shader_for_primitive<T: Glsl>(layout: &str) -> String {
    BASE_SHADER
        .replace("{struct_name}", T::NAME)
        .replace("{struct_definition}", "")
        .replace("{layout}", layout)
}

fn compile_glsl(glsl: &str) -> String {
    match compile(glsl) {
        Ok(shader) => shader,
        Err(err) => {
            eprintln!("Bad shader: {}", glsl);
            panic!("{}", err);
        }
    }
}

struct Context<T> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    _phantom: PhantomData<*const T>,
}

impl<T> Context<T>
where
    T: Debug + PartialEq + AsStd140 + AsStd430 + Glsl,
{
    fn new() -> Self {
        let (device, queue) = setup();
        Self {
            device,
            queue,
            _phantom: PhantomData,
        }
    }

    fn test_round_trip_std140(&self, glsl_shader: &str, value: &T) {
        let mut data = Vec::new();
        data.extend_from_slice(value.as_std140().as_bytes());

        let wgsl_shader = compile_glsl(glsl_shader);
        let bytes = self.round_trip(&wgsl_shader, &data);

        let std140 = bytemuck::from_bytes::<<T as AsStd140>::Output>(&bytes);
        let output = T::from_std140(*std140);

        if value != &output {
            println!(
                "std140 value did not round-trip through wgpu successfully.\n\
                Input:  {:?}\n\
                Output: {:?}\n\n\
                GLSL shader:\n{}\n\n\
                WGSL shader:\n{}",
                value, output, glsl_shader, wgsl_shader,
            );

            panic!("wgpu round-trip failure for {}", T::NAME);
        }
    }

    fn test_round_trip_std430(&self, glsl_shader: &str, value: &T) {
        let mut data = Vec::new();
        data.extend_from_slice(value.as_std430().as_bytes());

        let wgsl_shader = compile_glsl(glsl_shader);
        let bytes = self.round_trip(&wgsl_shader, &data);

        let std430 = bytemuck::from_bytes::<<T as AsStd430>::Output>(&bytes);
        let output = T::from_std430(*std430);

        if value != &output {
            println!(
                "std430 value did not round-trip through wgpu successfully.\n\
                Input:  {:?}\n\
                Output: {:?}\n\n\
                GLSL shader:\n{}\n\n\
                WGSL shader:\n{}",
                value, output, glsl_shader, wgsl_shader,
            );

            panic!("wgpu round-trip failure for {}", T::NAME);
        }
    }

    fn round_trip(&self, shader: &str, data: &[u8]) -> Vec<u8> {
        let input_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Input Buffer"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            });

        let output_gpu_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: data.len() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let output_cpu_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: data.len() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cs_module = self
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader)),
            });

        let compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: None,
                    module: &cs_module,
                    entry_point: "main",
                });

        let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_gpu_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch(1, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_gpu_buffer,
            0,
            &output_cpu_buffer,
            0,
            data.len() as wgpu::BufferAddress,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let output_slice = output_cpu_buffer.slice(..);
        let output_future = output_slice.map_async(wgpu::MapMode::Read);

        self.device.poll(wgpu::Maintain::Wait);
        block_on(output_future).unwrap();

        let output = output_slice.get_mapped_range().to_vec();
        output_cpu_buffer.unmap();

        output
    }
}

fn setup() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();

    println!("Adapter info: {:#?}", adapter.get_info());

    block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    ))
    .unwrap()
}

fn compile(glsl_source: &str) -> anyhow::Result<String> {
    let mut parser = naga::front::glsl::Parser::default();

    let module = parser
        .parse(
            &naga::front::glsl::Options {
                stage: naga::ShaderStage::Compute,
                defines: Default::default(),
            },
            glsl_source,
        )
        .map_err(|err| anyhow::format_err!("{:?}", err))?;

    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::default(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)?;

    let wgsl = naga::back::wgsl::write_string(&module, &info)?;

    Ok(wgsl)
}
