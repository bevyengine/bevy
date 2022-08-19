#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use std::io::Write;

    use wgpu::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BufferDescriptor, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor,
        ComputePipelineDescriptor, ShaderStages,
    };

    use crate::compose::{Composer, ShaderLanguage, ShaderType};

    #[test]
    fn simple_compose() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/simple/inc.wgsl"),
                "tests/simple/inc.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();
        let module = composer
            .make_naga_module(
                include_str!("tests/simple/top.wgsl"),
                "tests/simple/top.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        // println!("{}", wgsl);
        // let mut f = std::fs::File::create("simple_compose.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        assert_eq!(wgsl, include_str!("tests/expected/simple_compose.txt"));
    }

    #[test]
    fn duplicate_import() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/dup_import/consts.wgsl"),
                "tests/dup_import/consts.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();
        composer
            .add_composable_module(
                include_str!("tests/dup_import/a.wgsl"),
                "tests/dup_import/a.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();
        composer
            .add_composable_module(
                include_str!("tests/dup_import/b.wgsl"),
                "tests/dup_import/b.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();
        let module = composer
            .make_naga_module(
                include_str!("tests/dup_import/top.wgsl"),
                "tests/dup_import/top.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        // println!("{}", wgsl);
        // let mut f = std::fs::File::create("dup_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        assert_eq!(wgsl, include_str!("tests/expected/dup_import.txt"));
    }

    #[test]
    fn err_validation() {
        let mut composer = Composer::default();

        {
            let error = composer
                .make_naga_module(
                    include_str!("tests/error_test/wgsl_valid_err.wgsl"),
                    "tests/error_test/wgsl_valid_err.wgsl",
                    ShaderType::Wgsl,
                    &[],
                )
                .err()
                .unwrap();
            let text = error.emit_to_string(&composer);

            // println!("{}", text);
            // let mut f = std::fs::File::create("err_validation_1.txt").unwrap();
            // f.write_all(text.as_bytes()).unwrap();
            // drop(f);

            assert_eq!(text, include_str!("tests/expected/err_validation_1.txt"));
        }

        {
            composer
                .add_composable_module(
                    include_str!("tests/error_test/wgsl_valid_err.wgsl"),
                    "tests/error_test/wgsl_valid_err.wgsl",
                    ShaderLanguage::Wgsl,
                    None,
                )
                .unwrap();

            let error = composer
                .make_naga_module(
                    include_str!("tests/error_test/wgsl_valid_wrap.wgsl"),
                    "tests/error_test/wgsl_valid_wrap.wgsl",
                    ShaderType::Wgsl,
                    &[],
                )
                .err()
                .unwrap();

            let text = error.emit_to_string(&composer);

            // println!("{}", text);
            // let mut f = std::fs::File::create("err_validation_2.txt").unwrap();
            // f.write_all(text.as_bytes()).unwrap();
            // drop(f);

            assert_eq!(text, include_str!("tests/expected/err_validation_2.txt"));
        }
    }

    #[test]
    fn err_parse() {
        let mut composer = Composer::default();

        {
            let error = composer
                .make_naga_module(
                    include_str!("tests/error_test/wgsl_parse_err.wgsl"),
                    "tests/error_test/wgsl_parse_err.wgsl",
                    ShaderType::Wgsl,
                    &[],
                )
                .err()
                .unwrap();
            let text = error.emit_to_string(&composer);

            // println!("{}", text);
            // let mut f = std::fs::File::create("err_parse.txt").unwrap();
            // f.write_all(text.as_bytes()).unwrap();
            // drop(f);

            assert_eq!(text, include_str!("tests/expected/err_parse.txt"));
        }

        {
            composer
                .add_composable_module(
                    include_str!("tests/error_test/wgsl_parse_err.wgsl"),
                    "tests/error_test/wgsl_parse_err.wgsl",
                    ShaderLanguage::Wgsl,
                    None,
                )
                .unwrap();

            let error_2 = composer
                .make_naga_module(
                    include_str!("tests/error_test/wgsl_parse_wrap.wgsl"),
                    "tests/error_test/wgsl_parse_wrap.wgsl",
                    ShaderType::Wgsl,
                    &[],
                )
                .err()
                .unwrap();
            let text2 = error_2.emit_to_string(&composer);
            assert_eq!(text2, include_str!("tests/expected/err_parse.txt"));
        }
    }

    #[test]
    fn missing_import() {
        let mut composer = Composer::default();

        let error = composer
            .add_composable_module(
                include_str!("tests/error_test/include.wgsl"),
                "tests/error_test/include.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .err()
            .unwrap();
        let text = error.emit_to_string(&composer);
        // let mut f = std::fs::File::create("missing_import.txt").unwrap();
        // f.write_all(text.as_bytes()).unwrap();
        // drop(f);
        assert_eq!(text, include_str!("tests/expected/missing_import.txt"));
    }

    #[test]
    fn wgsl_call_glsl() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/glsl/module.glsl"),
                "tests/glsl/module.glsl",
                ShaderLanguage::Glsl,
                None,
            )
            .unwrap();

        let module = composer
            .make_naga_module(
                include_str!("tests/glsl/top.wgsl"),
                "tests/glsl/top.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        // unfortunately glsl variables are emitted in random order...
        // so this is better than nothing
        let mut wgsl: Vec<_> = wgsl.lines().collect();
        wgsl.sort();
        let wgsl = wgsl.join("\n");

        // let mut f = std::fs::File::create("wgsl_call_glsl.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        // assert_eq!(wgsl, include_str!("tests/expected/wgsl_call_glsl.txt"));

        // actually it's worse than that ... glsl output seems volatile over struct names
        // i suppose at least we are testing that it doesn't throw any errors ..?
        let _ = wgsl;
    }

    #[test]
    fn glsl_call_wgsl() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/glsl/module.wgsl"),
                "tests/glsl/module.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        let module = composer
            .make_naga_module(
                include_str!("tests/glsl/top.glsl"),
                "tests/glsl/top.glsl",
                ShaderType::GlslVertex,
                &[],
            )
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();
        // println!("{}", wgsl);
        // let mut f = std::fs::File::create("glsl_call_wgsl.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);
        assert_eq!(wgsl, include_str!("tests/expected/glsl_call_wgsl.txt"));
    }

    #[test]
    fn basic_glsl() {
        let mut composer = Composer::default();

        composer
            .make_naga_module(
                include_str!("tests/glsl/basic.glsl"),
                "tests/glsl/basic.glsl",
                ShaderType::GlslFragment,
                &[],
            )
            .unwrap();
    }

    #[test]
    fn wgsl_call_entrypoint() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/call_entrypoint/include.wgsl"),
                "tests/call_entrypoint/include.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        let module = composer
            .make_naga_module(
                include_str!("tests/call_entrypoint/top.wgsl"),
                "tests/call_entrypoint/top.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        // let mut f = std::fs::File::create("wgsl_call_entrypoint.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);
        assert_eq!(
            wgsl,
            include_str!("tests/expected/wgsl_call_entrypoint.txt")
        );
    }

    #[test]
    fn apply_override() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/overrides/mod.wgsl"),
                "tests/overrides/mod.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        let module = composer
            .make_naga_module(
                include_str!("tests/overrides/top.wgsl"),
                "tests/overrides/top.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        // println!("failed: {}", module.emit_to_string(&composer));

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        println!("{}", wgsl);
    }

    #[test]
    fn apply_mod_override() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/overrides/mod.wgsl"),
                "tests/overrides/mod.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        composer
            .add_composable_module(
                include_str!("tests/overrides/middle.wgsl"),
                "tests/overrides/middle.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        let module = composer
            .make_naga_module(
                include_str!("tests/overrides/top_with_middle.wgsl"),
                "tests/overrides/top_with_middle.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        // println!("failed: {}", module.emit_to_string(&composer));

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        println!("{}", wgsl);
    }

    #[test]
    fn apply_mod_override2() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/overrides/mod.wgsl"),
                "tests/overrides/mod.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        composer
            .add_composable_module(
                include_str!("tests/overrides/middle.wgsl"),
                "tests/overrides/middle.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        composer
            .add_composable_module(
                include_str!("tests/overrides/top_with_middle.wgsl"),
                "tests/overrides/top_with_middle.wgsl",
                ShaderLanguage::Wgsl,
                None,
            )
            .unwrap();

        test_shader(&mut composer, 3.0).unwrap();
    }

    // actually run a shader and extract the result
    // needs the composer to contain a module called "test_module", with a function called "entry_point" returning an f32.
    fn test_shader(composer: &mut Composer, expected: f32) -> Result<(), f32> {
        let module = composer
            .make_naga_module(
                include_str!("tests/compute_test.wgsl"),
                "tests/compute_test.wgsl",
                ShaderType::Wgsl,
                &[],
            )
            .unwrap();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .next()
            .unwrap();
        let (device, queue) = futures_lite::future::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
        )
        .unwrap();

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Naga(module),
            label: None,
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader_module,
            entry_point: "run_test",
        });

        let output_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 4,
            usage: BufferUsages::all(),
            mapped_at_creation: false,
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: Some(4.try_into().unwrap()),
                },
                count: None,
            }],
        });

        let bindgroup = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: output_buffer.as_entire_binding(),
            }],
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bindgroup, &[]);
        pass.dispatch_workgroups(1, 1, 1);

        drop(pass);

        let buffer = encoder.finish();

        queue.submit([buffer]);

        while !device.poll(wgpu::MaintainBase::Wait) {
            println!("waiting...");
        }

        output_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| ());

        while !device.poll(wgpu::MaintainBase::Wait) {
            println!("waiting...");
        }

        let view: &[u8] = &output_buffer.slice(..).get_mapped_range();
        let res = f32::from_le_bytes(view.try_into().unwrap());

        if (res - expected).abs() < res.max(expected) / 10000.0 {
            Ok(())
        } else {
            Err(res)
        }
    }
}
