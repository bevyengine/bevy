#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use std::io::Write;

    use crate::compose::{Composer, ShaderLanguage, ShaderType};

    #[test]
    fn simple_compose() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/simple/inc.wgsl"),
                "tests/simple/inc.wgsl",
                ShaderLanguage::Wgsl,
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
            )
            .unwrap();
        composer
            .add_composable_module(
                include_str!("tests/dup_import/a.wgsl"),
                "tests/dup_import/a.wgsl",
                ShaderLanguage::Wgsl,
            )
            .unwrap();
        composer
            .add_composable_module(
                include_str!("tests/dup_import/b.wgsl"),
                "tests/dup_import/b.wgsl",
                ShaderLanguage::Wgsl,
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

        // unforuntaly glsl variables are emitted in random order...
        // so this is better than nothing
        let mut wgsl: Vec<_> = wgsl.lines().collect();
        wgsl.sort();
        let wgsl = wgsl.join("\n");

        // let mut f = std::fs::File::create("wgsl_call_glsl.txt").unwrap();
        // f.write_all(wgsl.as_bytes()).unwrap();
        // drop(f);
        assert_eq!(wgsl, include_str!("tests/expected/wgsl_call_glsl.txt"));
    }

    #[test]
    fn glsl_call_wgsl() {
        let mut composer = Composer::default();

        composer
            .add_composable_module(
                include_str!("tests/glsl/module.wgsl"),
                "tests/glsl/module.wgsl",
                ShaderLanguage::Wgsl,
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
}
