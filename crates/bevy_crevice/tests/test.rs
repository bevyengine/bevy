use bevy_crevice::glsl::GlslStruct;
use bevy_crevice::std140::AsStd140;

#[test]
fn there_and_back_again() {
    #[derive(AsStd140, Debug, PartialEq)]
    struct ThereAndBackAgain {
        view: mint::ColumnMatrix3<f32>,
        origin: mint::Vector3<f32>,
    }

    let x = ThereAndBackAgain {
        view: mint::ColumnMatrix3 {
            x: mint::Vector3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            y: mint::Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            z: mint::Vector3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        },
        origin: mint::Vector3 {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
    };
    let x_as = x.as_std140();
    assert_eq!(<ThereAndBackAgain as AsStd140>::from_std140(x_as), x);
}

#[test]
fn generate_struct_glsl() {
    #[allow(dead_code)]
    #[derive(GlslStruct)]
    struct TestGlsl {
        foo: mint::Vector3<f32>,
        bar: mint::ColumnMatrix2<f32>,
    }

    insta::assert_display_snapshot!(TestGlsl::glsl_definition());
}

#[test]
fn generate_struct_array_glsl() {
    #[allow(dead_code)]
    #[derive(GlslStruct)]
    struct TestGlsl {
        foo: [[mint::Vector3<f32>; 8]; 4],
    }

    insta::assert_display_snapshot!(TestGlsl::glsl_definition());
}
