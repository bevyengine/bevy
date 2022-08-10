use naga_oil::compose::{Composer, ComposerError, ShaderLanguage};

fn test() -> Result<naga::Module, ComposerError> {
    let mut composer = Composer::non_validating();

    let mut load_composable = |source: String| -> () {
        match composer.add_composable_module(source, ShaderLanguage::Wgsl) {
            Ok(_module) => {
                // println!("{} -> {:#?}", module.name, module)
            }
            Err(e) => {
                println!("{} -> {:#?}", "?", e)
            }
        }
    };

    load_composable(include_str!("bevy_pbr_wgsl/utils.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/mesh_view_types.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_view_bindings.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/pbr_types.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/pbr_bindings.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/skinning.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_types.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_bindings.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_vertex_output.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/clustered_forward.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/pbr_lighting.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/shadows.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/pbr_functions.wgsl").to_string());

    match composer.make_naga_module(
        include_str!("bevy_pbr_wgsl/pbr.wgsl").to_string(),
        ShaderLanguage::Wgsl,
        &[],
    ) {
        Ok(module) => {
            // println!("shader: {:#?}", module);
            // let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default()).validate(&module).unwrap();
            // let _wgsl = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::EXPLICIT_TYPES).unwrap();
            // println!("wgsl: \n\n{}", wgsl);
            Ok(module)
        }
        Err(e) => {
            println!("shader: {:#?}", e);
            Err(e)
        }
    }
}

fn test2() {
    let mut composer = Composer::non_validating();

    let mut load_composable = |source: String| -> () {
        match composer.add_composable_module(source, ShaderLanguage::Wgsl) {
            Ok(_module) => {
                // println!("{} -> {:#?}", module.name, module)
            }
            Err(e) => {
                println!("{} -> {:#?}", "?", e)
            }
        }
    };

    load_composable(include_str!("bevy_pbr_wgsl/utils.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/mesh_view_types.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_view_bindings.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/pbr_types.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/pbr_bindings.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/skinning.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_types.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_bindings.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/mesh_vertex_output.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/pbr_lighting.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/shadows.wgsl").to_string());
    load_composable(include_str!("bevy_pbr_wgsl/clustered_forward.wgsl").to_string());

    load_composable(include_str!("bevy_pbr_wgsl/pbr_functions.wgsl").to_string());

    for _ in 0..1000 {
        let shader = match composer.make_naga_module(
            include_str!("bevy_pbr_wgsl/pbr.wgsl").to_string(),
            ShaderLanguage::Wgsl,
            &[],
        ) {
            Ok(module) => {
                // println!("shader: {:#?}", module);
                // let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default()).validate(&module).unwrap();
                // let _wgsl = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::EXPLICIT_TYPES).unwrap();
                // println!("wgsl: \n\n{}", wgsl);
                Ok(module)
            }
            Err(e) => {
                println!("shader: {:#?}", e);
                Err(e)
            }
        };

        if shader.unwrap().types.iter().next().is_none() {
            println!("ouch");
        }
    }
}

fn main() {
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let pbr = test().unwrap();
        if pbr.types.iter().next().is_none() {
            println!("ouch");
        }
    }
    let end = std::time::Instant::now();
    println!("1000 full builds: {:?}", end - start);

    let start = std::time::Instant::now();
    test2();
    let end = std::time::Instant::now();
    println!("1000 final builds: {:?}", end - start);
}
