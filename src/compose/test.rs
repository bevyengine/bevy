#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use std::io::Write;

    use crate::compose::{Composer, ShaderLanguage};

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
                ShaderLanguage::Wgsl,
                &[],
            )
            .unwrap();

        assert_eq!(
            format!("{:?}", module),
            include_str!("tests/expected/simple_compose.txt")
        );
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
                ShaderLanguage::Wgsl,
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
                    ShaderLanguage::Wgsl,
                    &[],
                )
                .err()
                .unwrap();
            let text = error.emit_to_string(&composer);

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
                    ShaderLanguage::Wgsl,
                    &[],
                )
                .err()
                .unwrap();

            let text = error.emit_to_string(&composer);

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
                    ShaderLanguage::Wgsl,
                    &[],
                )
                .err()
                .unwrap();
            let text = error.emit_to_string(&composer);
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
                    ShaderLanguage::Wgsl,
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
}
