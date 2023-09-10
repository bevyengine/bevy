#[cfg(test)]
#[allow(clippy::module_inception)]
mod test {
    #[allow(unused_imports)]
    use std::io::Write;
    use std::{borrow::Cow, collections::HashMap};

    use wgpu::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BufferDescriptor, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor,
        ComputePipelineDescriptor, Features, ShaderStages,
    };

    use crate::compose::{
        get_preprocessor_data, ComposableModuleDescriptor, Composer, ImportDefinition,
        NagaModuleDescriptor, ShaderDefValue, ShaderLanguage, ShaderType,
    };

    macro_rules! output_eq {
        ($result:ident, $path:expr) => {
            assert_eq!(
                $result.replace("\r", ""),
                include_str!($path).replace("\r", "")
            )
        };
    }

    #[test]
    fn simple_compose() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/simple/inc.wgsl"),
                file_path: "tests/simple/inc.wgsl",
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/simple/top.wgsl"),
                file_path: "tests/simple/top.wgsl",
                ..Default::default()
            })
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

        output_eq!(wgsl, "tests/expected/simple_compose.txt");
    }

    #[test]
    fn big_shaderdefs() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/big_shaderdefs/mod.wgsl"),
                file_path: "tests/big_shaderdefs/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        let defs = (1..=67)
            .map(|i| (format!("a{i}"), ShaderDefValue::Bool(true)))
            .collect::<HashMap<_, _>>();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/big_shaderdefs/top.wgsl"),
                file_path: "tests/big_shaderdefs/top.wgsl",
                shader_defs: defs,
                ..Default::default()
            })
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
        // let mut f = std::fs::File::create("big_shaderdefs.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/big_shaderdefs.txt");
    }

    #[test]
    fn duplicate_import() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/dup_import/consts.wgsl"),
                file_path: "tests/dup_import/consts.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/dup_import/a.wgsl"),
                file_path: "tests/dup_import/a.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/dup_import/b.wgsl"),
                file_path: "tests/dup_import/b.wgsl",
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/dup_import/top.wgsl"),
                file_path: "tests/dup_import/top.wgsl",
                ..Default::default()
            })
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
        let mut wgsl: Vec<_> = wgsl.lines().collect();
        wgsl.sort();
        let wgsl = wgsl.join("\n");

        // println!("{}", wgsl);
        // let mut f = std::fs::File::create("dup_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/dup_import.txt");
    }

    #[test]
    fn err_validation() {
        let mut composer = Composer::default();

        {
            let error = composer
                .make_naga_module(NagaModuleDescriptor {
                    source: include_str!("tests/error_test/wgsl_valid_err.wgsl"),
                    file_path: "tests/error_test/wgsl_valid_err.wgsl",
                    ..Default::default()
                })
                .err()
                .unwrap();
            let text = error.emit_to_string(&composer);

            // println!("{}", text);
            // let mut f = std::fs::File::create("err_validation_1.txt").unwrap();
            // f.write_all(text.as_bytes()).unwrap();
            // drop(f);

            output_eq!(text, "tests/expected/err_validation_1.txt");
        }

        {
            composer
                .add_composable_module(ComposableModuleDescriptor {
                    source: include_str!("tests/error_test/wgsl_valid_err.wgsl"),
                    file_path: "tests/error_test/wgsl_valid_err.wgsl",
                    ..Default::default()
                })
                .unwrap();

            let error = composer
                .make_naga_module(NagaModuleDescriptor {
                    source: include_str!("tests/error_test/wgsl_valid_wrap.wgsl"),
                    file_path: "tests/error_test/wgsl_valid_wrap.wgsl",
                    ..Default::default()
                })
                .err()
                .unwrap();

            let text = error.emit_to_string(&composer);

            // println!("{}", text);
            // let mut f = std::fs::File::create("err_validation_2.txt").unwrap();
            // f.write_all(text.as_bytes()).unwrap();
            // drop(f);

            output_eq!(text, "tests/expected/err_validation_2.txt");
        }
    }

    #[test]
    fn err_parse() {
        let mut composer = Composer::default();

        {
            let error = composer
                .make_naga_module(NagaModuleDescriptor {
                    source: include_str!("tests/error_test/wgsl_parse_err.wgsl"),
                    file_path: "tests/error_test/wgsl_parse_err.wgsl",
                    ..Default::default()
                })
                .err()
                .unwrap();
            let text = error.emit_to_string(&composer);

            // println!("{}", text);
            // let mut f = std::fs::File::create("err_parse.txt").unwrap();
            // f.write_all(text.as_bytes()).unwrap();
            // drop(f);

            output_eq!(text, "tests/expected/err_parse.txt");
        }

        {
            composer
                .add_composable_module(ComposableModuleDescriptor {
                    source: include_str!("tests/error_test/wgsl_parse_err.wgsl"),
                    file_path: "tests/error_test/wgsl_parse_err.wgsl",
                    ..Default::default()
                })
                .unwrap();

            let error_2 = composer
                .make_naga_module(NagaModuleDescriptor {
                    source: include_str!("tests/error_test/wgsl_parse_wrap.wgsl"),
                    file_path: "tests/error_test/wgsl_parse_wrap.wgsl",
                    ..Default::default()
                })
                .err()
                .unwrap();
            let text2 = error_2.emit_to_string(&composer);
            output_eq!(text2, "tests/expected/err_parse.txt");
        }
    }

    #[test]
    fn missing_import_in_module() {
        let mut composer = Composer::default();

        let error = composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/error_test/include.wgsl"),
                file_path: "tests/error_test/include.wgsl",
                ..Default::default()
            })
            .err()
            .unwrap();
        let text = error.emit_to_string(&composer);
        // let mut f = std::fs::File::create("missing_import.txt").unwrap();
        // f.write_all(text.as_bytes()).unwrap();
        // drop(f);
        output_eq!(text, "tests/expected/missing_import.txt");
    }

    #[test]
    fn missing_import_in_shader() {
        let mut composer = Composer::default();

        let error = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/error_test/include.wgsl"),
                file_path: "tests/error_test/include.wgsl",
                ..Default::default()
            })
            .err()
            .unwrap();
        let text = error.emit_to_string(&composer);
        // let mut f = std::fs::File::create("missing_import.txt").unwrap();
        // f.write_all(text.as_bytes()).unwrap();
        // drop(f);
        output_eq!(text, "tests/expected/missing_import.txt");
    }

    #[test]
    fn wgsl_call_glsl() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/glsl/module.glsl"),
                file_path: "tests/glsl/module.glsl",
                language: ShaderLanguage::Glsl,
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/glsl/top.wgsl"),
                file_path: "tests/glsl/top.wgsl",
                ..Default::default()
            })
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
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/glsl/module.wgsl"),
                file_path: "tests/glsl/module.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/glsl/top.glsl"),
                file_path: "tests/glsl/top.glsl",
                shader_type: ShaderType::GlslVertex,
                ..Default::default()
            })
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
        output_eq!(wgsl, "tests/expected/glsl_call_wgsl.txt");
    }

    #[test]
    fn basic_glsl() {
        let mut composer = Composer::default();

        composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/glsl/basic.glsl"),
                file_path: "tests/glsl/basic.glsl",
                shader_type: ShaderType::GlslFragment,
                ..Default::default()
            })
            .unwrap();
    }

    #[test]
    fn wgsl_call_entrypoint() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/call_entrypoint/include.wgsl"),
                file_path: "tests/call_entrypoint/include.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/call_entrypoint/top.wgsl"),
                file_path: "tests/call_entrypoint/top.wgsl",
                ..Default::default()
            })
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
        output_eq!(wgsl, "tests/expected/wgsl_call_entrypoint.txt");
    }

    #[test]
    fn apply_override() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/overrides/mod.wgsl"),
                file_path: "tests/overrides/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/overrides/top.wgsl"),
                file_path: "tests/overrides/top.wgsl",
                ..Default::default()
            })
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

        // todo test properly - the redirect returns the functions in random order so can't rely on string repr
        println!("{wgsl}");
    }

    #[cfg(feature = "test_shader")]
    #[test]
    fn apply_mod_override() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/overrides/mod.wgsl"),
                file_path: "tests/overrides/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/overrides/middle.wgsl"),
                file_path: "tests/overrides/middle.wgsl",
                ..Default::default()
            })
            .unwrap();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/overrides/top_with_middle.wgsl"),
                file_path: "tests/overrides/top_with_middle.wgsl",
                ..Default::default()
            })
            .unwrap();

        // this test doesn't work any more.
        // overrides only work if the composer realises the module is required.
        // not we can't just blindly import any `#import`ed items because that would break:
        //      #import a::b
        //      a::b::c::d();
        // the path would be interpreted as a module when it may actually
        // be only a fragment of a path to a module.
        // so either i need to add another directive (#import_overrides)
        // or we just limit overrides to modules included via the additional_modules
        // in `Composer::make_naga_module` and `Composer::add_composable_module`

        // assert_eq!(test_shader(&mut composer), 3.0);
    }

    #[cfg(feature = "test_shader")]
    #[test]
    fn additional_import() {
        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/add_imports/overridable.wgsl"),
                file_path: "tests/add_imports/overridable.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/add_imports/plugin.wgsl"),
                file_path: "tests/add_imports/plugin.wgsl",
                as_name: Some("plugin".to_owned()),
                ..Default::default()
            })
            .unwrap();

        // test as shader
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/add_imports/top.wgsl"),
                file_path: "tests/add_imports/top.wgsl",
                additional_imports: &[ImportDefinition {
                    import: "plugin".to_owned(),
                    ..Default::default()
                }],
                ..Default::default()
            })
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

        // let mut f = std::fs::File::create("additional_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/additional_import.txt");

        // test as module
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/add_imports/top.wgsl"),
                file_path: "tests/add_imports/top.wgsl",
                as_name: Some("test_module".to_owned()),
                additional_imports: &[ImportDefinition {
                    import: "plugin".to_owned(),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();

        assert_eq!(test_shader(&mut composer), 2.0);
    }

    #[test]
    fn invalid_override() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/overrides/mod.wgsl"),
                file_path: "tests/overrides/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer.make_naga_module(NagaModuleDescriptor {
            source: include_str!("tests/overrides/top_invalid.wgsl"),
            file_path: "tests/overrides/top_invalid.wgsl",
            ..Default::default()
        });

        #[cfg(feature = "override_any")]
        {
            let module = module.unwrap();
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

            // todo test properly - the redirect returns the functions in random order so can't rely on string repr
            println!("{}", wgsl);
        }

        #[cfg(not(feature = "override_any"))]
        {
            let err = module.err().unwrap();
            let err = err.emit_to_string(&composer);
            // let mut f = std::fs::File::create("invalid_override_base.txt").unwrap();
            // f.write_all(err.as_bytes()).unwrap();
            // drop(f);
            output_eq!(err, "tests/expected/invalid_override_base.txt");
        }
    }

    #[test]
    fn import_in_decl() {
        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/const_in_decl/consts.wgsl"),
                file_path: "tests/const_in_decl/consts.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/const_in_decl/bind.wgsl"),
                file_path: "tests/const_in_decl/bind.wgsl",
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/const_in_decl/top.wgsl"),
                file_path: "tests/const_in_decl/top.wgsl",
                ..Default::default()
            })
            .unwrap();

        // println!("{:#?}", module);

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
        // let mut f = std::fs::File::create("import_in_decl.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/import_in_decl.txt");
    }

    #[test]
    fn glsl_const_import() {
        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/glsl_const_import/consts.glsl"),
                file_path: "tests/glsl_const_import/consts.glsl",
                language: ShaderLanguage::Glsl,
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/glsl_const_import/top.glsl"),
                file_path: "tests/glsl_const_import/top.glsl",
                shader_type: ShaderType::GlslFragment,
                ..Default::default()
            })
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
        // let mut f = std::fs::File::create("glsl_const_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/glsl_const_import.txt");
    }

    #[test]
    fn glsl_wgsl_const_import() {
        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/glsl_const_import/consts.glsl"),
                file_path: "tests/glsl_const_import/consts.glsl",
                language: ShaderLanguage::Glsl,
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/glsl_const_import/top.wgsl"),
                file_path: "tests/glsl_const_import/top.wgsl",
                ..Default::default()
            })
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
        // let mut f = std::fs::File::create("glsl_wgsl_const_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/glsl_wgsl_const_import.txt");
    }
    #[test]
    fn wgsl_glsl_const_import() {
        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/glsl_const_import/consts.wgsl"),
                file_path: "tests/glsl_const_import/consts.wgsl",
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/glsl_const_import/top.glsl"),
                file_path: "tests/glsl_const_import/top.glsl",
                shader_type: ShaderType::GlslFragment,
                ..Default::default()
            })
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
        // let mut f = std::fs::File::create("wgsl_glsl_const_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/wgsl_glsl_const_import.txt");
    }

    #[test]
    fn item_import_test() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/item_import/consts.wgsl"),
                file_path: "tests/item_import/consts.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/item_import/top.wgsl"),
                file_path: "tests/item_import/top.wgsl",
                ..Default::default()
            })
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
        let mut wgsl = wgsl.lines().collect::<Vec<_>>();
        wgsl.sort();
        let wgsl = wgsl.join("\n");

        // let mut f = std::fs::File::create("item_import_test.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/item_import_test.txt");
    }

    #[test]
    fn bad_identifiers() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/const.wgsl"),
                file_path: "tests/invalid_identifiers/const.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/fn.wgsl"),
                file_path: "tests/invalid_identifiers/fn.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/global.wgsl"),
                file_path: "tests/invalid_identifiers/global.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/struct_member.wgsl"),
                file_path: "tests/invalid_identifiers/struct_member.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/struct.wgsl"),
                file_path: "tests/invalid_identifiers/struct.wgsl",
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/top_valid.wgsl"),
                file_path: "tests/invalid_identifiers/top_valid.wgsl",
                ..Default::default()
            })
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
        let mut wgsl: Vec<_> = wgsl.lines().collect();
        wgsl.sort();
        let wgsl = wgsl.join("\n");

        // let mut f = std::fs::File::create("bad_identifiers.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/bad_identifiers.txt");

        composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/invalid_identifiers/top_invalid.wgsl"),
                file_path: "tests/invalid_identifiers/top_invalid.wgsl",
                ..Default::default()
            })
            .err()
            .unwrap();
    }

    #[test]
    fn dup_struct_import() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/dup_struct_import/struct.wgsl"),
                file_path: "tests/dup_struct_import/struct.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/dup_struct_import/a.wgsl"),
                file_path: "tests/dup_struct_import/a.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/dup_struct_import/b.wgsl"),
                file_path: "tests/dup_struct_import/b.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/dup_struct_import/top.wgsl"),
                file_path: "tests/dup_struct_import/top.wgsl",
                ..Default::default()
            })
            .unwrap();

        // println!("{}", module.emit_to_string(&composer));
        // assert!(false);

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
        let mut wgsl: Vec<_> = wgsl.lines().collect();
        wgsl.sort();
        let wgsl = wgsl.join("\n");

        // let mut f = std::fs::File::create("dup_struct_import.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/dup_struct_import.txt");
    }

    #[test]
    fn item_sub_point() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/item_sub_point/mod.wgsl"),
                file_path: "tests/item_sub_point/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/item_sub_point/top.wgsl"),
                file_path: "tests/item_sub_point/top.wgsl",
                ..Default::default()
            })
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

        // let mut f = std::fs::File::create("item_sub_point.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/item_sub_point.txt");
    }

    #[test]
    fn conditional_import() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/conditional_import/mod_a.wgsl"),
                file_path: "tests/conditional_import/mod_a.wgsl",
                ..Default::default()
            })
            .unwrap();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/conditional_import/mod_b.wgsl"),
                file_path: "tests/conditional_import/mod_b.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module_a = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/conditional_import/top.wgsl"),
                file_path: "tests/conditional_import/top.wgsl",
                shader_defs: HashMap::from_iter([("USE_A".to_owned(), ShaderDefValue::Bool(true))]),
                ..Default::default()
            })
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module_a)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module_a,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        // let mut f = std::fs::File::create("conditional_import_a.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/conditional_import_a.txt");

        let module_b = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/conditional_import/top.wgsl"),
                file_path: "tests/conditional_import/top.wgsl",
                ..Default::default()
            })
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module_b)
        .unwrap();
        let wgsl = naga::back::wgsl::write_string(
            &module_b,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .unwrap();

        // let mut f = std::fs::File::create("conditional_import_b.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/conditional_import_b.txt");
    }

    #[cfg(feature = "test_shader")]
    #[test]
    fn rusty_imports() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/rusty_imports/mod_a_b_c.wgsl"),
                file_path: "tests/rusty_imports/mod_a_b_c.wgsl",
                ..Default::default()
            })
            .unwrap();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/rusty_imports/mod_a_x.wgsl"),
                file_path: "tests/rusty_imports/mod_a_x.wgsl",
                ..Default::default()
            })
            .unwrap();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/rusty_imports/top.wgsl"),
                file_path: "tests/rusty_imports/top.wgsl",
                ..Default::default()
            })
            .unwrap();

        assert_eq!(test_shader(&mut composer), 36.0);
    }

    #[test]
    fn test_bevy_path_imports() {
        let (_, mut imports, _) =
            get_preprocessor_data(include_str!("tests/bevy_path_imports/skill.wgsl"));
        imports.iter_mut().for_each(|import| {
            import.items.sort();
        });
        imports.sort_by(|a, b| a.import.cmp(&b.import));
        assert_eq!(
            imports,
            vec![
                ImportDefinition {
                    import: "\"shaders/skills/hit.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/lightning.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/lightning_ring.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/magic_arrow.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/orb.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/railgun_trail.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/shared.wgsl\"".to_owned(),
                    items: vec![
                        "Vertex".to_owned(),
                        "VertexOutput".to_owned(),
                        "VertexOutput".to_owned(),
                    ],
                },
                ImportDefinition {
                    import: "\"shaders/skills/slash.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
                ImportDefinition {
                    import: "\"shaders/skills/sound.wgsl\"".to_owned(),
                    items: vec!["frag".to_owned(), "vert".to_owned(),],
                },
            ]
        );
    }

    #[test]
    fn test_quoted_import_dup_name() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/quoted_dup/mod.wgsl"),
                file_path: "tests/quoted_dup/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/quoted_dup/top.wgsl"),
                file_path: "tests/quoted_dup/top.wgsl",
                ..Default::default()
            })
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

        // let mut f = std::fs::File::create("test_quoted_import_dup_name.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/test_quoted_import_dup_name.txt");
    }

    #[test]
    fn use_shared_global() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/use_shared_global/mod.wgsl"),
                file_path: "tests/use_shared_global/mod.wgsl",
                ..Default::default()
            })
            .unwrap();
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/use_shared_global/top.wgsl"),
                file_path: "tests/use_shared_global/top.wgsl",
                ..Default::default()
            })
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

        // let mut f = std::fs::File::create("use_shared_global.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);

        output_eq!(wgsl, "tests/expected/use_shared_global.txt");
    }

    #[cfg(feature = "test_shader")]
    #[test]
    fn effective_defs() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("tests/effective_defs/mod.wgsl"),
                file_path: "tests/effective_defs/mod.wgsl",
                ..Default::default()
            })
            .unwrap();

        for (defs, expected) in [
            (
                vec![("DEF_THREE".to_owned(), ShaderDefValue::Bool(false))],
                0.0,
            ),
            (
                vec![
                    ("DEF_ONE".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_THREE".to_owned(), ShaderDefValue::Bool(false)),
                ],
                1.0,
            ),
            (
                vec![
                    ("DEF_TWO".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_THREE".to_owned(), ShaderDefValue::Bool(false)),
                ],
                2.0,
            ),
            (
                vec![
                    ("DEF_ONE".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_TWO".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_THREE".to_owned(), ShaderDefValue::Bool(false)),
                ],
                3.0,
            ),
            (
                vec![("DEF_THREE".to_owned(), ShaderDefValue::Bool(true))],
                4.0,
            ),
            (
                vec![
                    ("DEF_ONE".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_THREE".to_owned(), ShaderDefValue::Bool(true)),
                ],
                5.0,
            ),
            (
                vec![
                    ("DEF_TWO".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_THREE".to_owned(), ShaderDefValue::Bool(true)),
                ],
                6.0,
            ),
            (
                vec![
                    ("DEF_ONE".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_TWO".to_owned(), ShaderDefValue::Bool(true)),
                    ("DEF_THREE".to_owned(), ShaderDefValue::Bool(true)),
                ],
                7.0,
            ),
        ] {
            composer
                .add_composable_module(ComposableModuleDescriptor {
                    source: include_str!("tests/effective_defs/top.wgsl"),
                    file_path: "tests/effective_defs/top.wgsl",
                    shader_defs: HashMap::from_iter(defs),
                    ..Default::default()
                })
                .unwrap();

            assert_eq!(test_shader(&mut composer), expected);
        }
    }

    // actually run a shader and extract the result
    // needs the composer to contain a module called "test_module", with a function called "entry_point" returning an f32.
    fn test_shader(composer: &mut Composer) -> f32 {
        let module = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("tests/compute_test.wgsl"),
                file_path: "tests/compute_test.wgsl",
                ..Default::default()
            })
            .unwrap();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .next()
            .unwrap();
        let (device, queue) = futures_lite::future::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: Features::MAPPABLE_PRIMARY_BUFFERS,
                ..Default::default()
            },
            None,
        ))
        .unwrap();

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
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
            usage: BufferUsages::MAP_READ | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
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

        f32::from_le_bytes(view.try_into().unwrap())
    }
}
