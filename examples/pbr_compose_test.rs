use std::borrow::Cow;

use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, ComposerError, NagaModuleDescriptor,
};
#[allow(unused_variables, dead_code)]

fn init_composer() -> Composer {
    let mut composer = Composer::default();

    let mut load_composable = |source: &str, file_path: &str| {
        match composer.add_composable_module(ComposableModuleDescriptor {
            source,
            file_path,
            ..Default::default()
        }) {
            Ok(_module) => {
                // println!("{} -> {:#?}", module.name, module)
            }
            Err(e) => {
                println!("? -> {e:#?}")
            }
        }
    };

    load_composable(
        include_str!("bevy_pbr_wgsl/utils.wgsl"),
        "examples/bevy_pbr_wgsl/utils.wgsl",
    );

    load_composable(
        include_str!("bevy_pbr_wgsl/mesh_view_types.wgsl"),
        "examples/bevy_pbr_wgsl/mesh_view_types.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/mesh_view_bindings.wgsl"),
        "examples/bevy_pbr_wgsl/mesh_view_bindings.wgsl",
    );

    load_composable(
        include_str!("bevy_pbr_wgsl/pbr_types.wgsl"),
        "examples/bevy_pbr_wgsl/pbr_types.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/pbr_bindings.wgsl"),
        "examples/bevy_pbr_wgsl/pbr_bindings.wgsl",
    );

    load_composable(
        include_str!("bevy_pbr_wgsl/skinning.wgsl"),
        "examples/bevy_pbr_wgsl/skinning.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/mesh_types.wgsl"),
        "examples/bevy_pbr_wgsl/mesh_types.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/mesh_bindings.wgsl"),
        "examples/bevy_pbr_wgsl/mesh_bindings.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/mesh_vertex_output.wgsl"),
        "examples/bevy_pbr_wgsl/mesh_vertex_output.wgsl",
    );

    load_composable(
        include_str!("bevy_pbr_wgsl/clustered_forward.wgsl"),
        "examples/bevy_pbr_wgsl/clustered_forward.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/pbr_lighting.wgsl"),
        "examples/bevy_pbr_wgsl/pbr_lighting.wgsl",
    );
    load_composable(
        include_str!("bevy_pbr_wgsl/shadows.wgsl"),
        "examples/bevy_pbr_wgsl/shadows.wgsl",
    );

    load_composable(
        include_str!("bevy_pbr_wgsl/pbr_functions.wgsl"),
        "examples/bevy_pbr_wgsl/pbr_functions.wgsl",
    );

    composer
}

// rebuild composer every time
fn test_compose_full() -> Result<naga::Module, ComposerError> {
    let mut composer = init_composer();

    match composer.make_naga_module(NagaModuleDescriptor {
        source: include_str!("bevy_pbr_wgsl/pbr.wgsl"),
        file_path: "examples/bevy_pbr_wgsl/pbr.wgsl",
        shader_defs: [("VERTEX_UVS".to_owned(), Default::default())].into(),
        ..Default::default()
    }) {
        Ok(module) => {
            // println!("shader: {:#?}", module);
            // let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default()).validate(&module).unwrap();
            // let _wgsl = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::EXPLICIT_TYPES).unwrap();
            // println!("wgsl: \n\n{}", wgsl);
            Ok(module)
        }
        Err(e) => {
            println!("{}", e.emit_to_string(&composer));
            Err(e)
        }
    }
}

// make naga module from initialized composer
fn test_compose_final_module(n: usize, composer: &mut Composer) {
    let mut shader;
    for _ in 0..n {
        shader = match composer.make_naga_module(NagaModuleDescriptor {
            source: include_str!("bevy_pbr_wgsl/pbr.wgsl"),
            file_path: "examples/bevy_pbr_wgsl/pbr.wgsl",
            shader_defs: [("VERTEX_UVS".to_owned(), Default::default())].into(),
            ..Default::default()
        }) {
            Ok(module) => {
                // println!("shader: {:#?}", module);
                // let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default()).validate(&module).unwrap();
                // let _wgsl = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::EXPLICIT_TYPES).unwrap();
                // println!("wgsl: \n\n{}", wgsl);
                Ok(module)
            }
            Err(e) => {
                println!("error: {e:#?}");
                Err(e)
            }
        };

        if shader.as_ref().unwrap().types.iter().next().is_none() {
            println!("ouch");
        }
    }
}

// make shader module from string
fn test_wgsl_string_compile(n: usize) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .enumerate_adapters(wgpu::Backends::all())
        .next()
        .unwrap();
    let device = futures_lite::future::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
    )
    .unwrap()
    .0;

    for _ in 0..n {
        let _desc = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Wgsl(
                include_str!("bevy_pbr_wgsl/output_VERTEX_UVS.wgsl").into(),
            ),
            label: None,
        });
    }
}

// make shader module from composed naga
fn test_composer_compile(n: usize, composer: &mut Composer) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .enumerate_adapters(wgpu::Backends::all())
        .next()
        .unwrap();
    let device = futures_lite::future::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
    )
    .unwrap()
    .0;

    for _ in 0..n {
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("bevy_pbr_wgsl/pbr.wgsl"),
                file_path: "examples/bevy_pbr_wgsl/pbr.wgsl",
                shader_defs: [("VERTEX_UVS".to_owned(), Default::default())].into(),
                ..Default::default()
            })
            .unwrap();
        let _desc = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
            label: None,
        });
    }
}

fn main() {
    println!("running 1000 full composer builds (no caching)");
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let pbr = test_compose_full().unwrap();
        if pbr.types.iter().next().is_none() {
            println!("ouch");
        }
    }
    let end = std::time::Instant::now();
    println!("1000 full builds: {:?}", end - start);

    let mut composer = init_composer();

    println!("running 10000 composer final builds");
    let start = std::time::Instant::now();
    test_compose_final_module(10000, &mut composer);
    let end = std::time::Instant::now();
    println!("10000 final builds: {:?}", end - start);

    println!("running 10000 wgpu string compiles");
    let start = std::time::Instant::now();
    test_wgsl_string_compile(10000);
    let end = std::time::Instant::now();
    println!("10000 string compiles: {:?}", end - start);

    println!("running 10000 composer builds + wgpu module compiles");
    let start = std::time::Instant::now();
    test_composer_compile(10000, &mut composer);
    let end = std::time::Instant::now();
    println!("10000 module compiles: {:?}", end - start);
}
