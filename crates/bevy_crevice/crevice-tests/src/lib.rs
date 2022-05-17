#![cfg(test)]

#[cfg(feature = "wgpu-validation")]
mod gpu;

#[cfg(feature = "wgpu-validation")]
use gpu::{test_round_trip_primitive, test_round_trip_struct};

#[cfg(not(feature = "wgpu-validation"))]
fn test_round_trip_struct<T>(_value: T) {}

#[cfg(not(feature = "wgpu-validation"))]
fn test_round_trip_primitive<T>(_value: T) {}

#[macro_use]
mod util;

use bevy_crevice::glsl::GlslStruct;
use bevy_crevice::std140::AsStd140;
use bevy_crevice::std430::AsStd430;
use mint::{ColumnMatrix2, ColumnMatrix3, ColumnMatrix4, Vector2, Vector3, Vector4};

#[test]
fn two_f32() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct TwoF32 {
        x: f32,
        y: f32,
    }

    assert_std140!((size = 16, align = 16) TwoF32 {
        x: 0,
        y: 4,
    });

    assert_std430!((size = 8, align = 4) TwoF32 {
        x: 0,
        y: 4,
    });

    test_round_trip_struct(TwoF32 { x: 5.0, y: 7.0 });
}

#[test]
fn vec2() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct UseVec2 {
        one: Vector2<f32>,
    }

    assert_std140!((size = 16, align = 16) UseVec2 {
        one: 0,
    });

    test_round_trip_struct(UseVec2 {
        one: [1.0, 2.0].into(),
    });
}

#[test]
fn mat2_bare() {
    type Mat2 = ColumnMatrix2<f32>;

    assert_std140!((size = 32, align = 16) Mat2 {
        x: 0,
        y: 16,
    });

    assert_std430!((size = 16, align = 8) Mat2 {
        x: 0,
        y: 8,
    });

    // Naga doesn't work with std140 mat2 values.
    // https://github.com/gfx-rs/naga/issues/1400

    // test_round_trip_primitive(Mat2 {
    //     x: [1.0, 2.0].into(),
    //     y: [3.0, 4.0].into(),
    // });
}

#[test]
fn mat3_bare() {
    type Mat3 = ColumnMatrix3<f32>;

    assert_std140!((size = 48, align = 16) Mat3 {
        x: 0,
        y: 16,
        z: 32,
    });

    // Naga produces invalid HLSL for mat3 value.
    // https://github.com/gfx-rs/naga/issues/1466

    // test_round_trip_primitive(Mat3 {
    //     x: [1.0, 2.0, 3.0].into(),
    //     y: [4.0, 5.0, 6.0].into(),
    //     z: [7.0, 8.0, 9.0].into(),
    // });
}

#[test]
fn mat4_bare() {
    type Mat4 = ColumnMatrix4<f32>;

    assert_std140!((size = 64, align = 16) Mat4 {
        x: 0,
        y: 16,
        z: 32,
        w: 48,
    });

    test_round_trip_primitive(Mat4 {
        x: [1.0, 2.0, 3.0, 4.0].into(),
        y: [5.0, 6.0, 7.0, 8.0].into(),
        z: [9.0, 10.0, 11.0, 12.0].into(),
        w: [13.0, 14.0, 15.0, 16.0].into(),
    });
}

#[test]
fn mat3() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct TestData {
        one: ColumnMatrix3<f32>,
    }

    // Naga produces invalid HLSL for mat3 value.
    // https://github.com/gfx-rs/naga/issues/1466

    // test_round_trip_struct(TestData {
    //     one: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]].into(),
    // });
}

#[test]
fn dvec4() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct UsingDVec4 {
        doubles: Vector4<f64>,
    }

    assert_std140!((size = 32, align = 32) UsingDVec4 {
        doubles: 0,
    });

    // Naga does not appear to support doubles.
    // https://github.com/gfx-rs/naga/issues/1272

    // test_round_trip_struct(UsingDVec4 {
    //     doubles: [1.0, 2.0, 3.0, 4.0].into(),
    // });
}

#[test]
fn four_f64() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct FourF64 {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    }

    assert_std140!((size = 32, align = 16) FourF64 {
        x: 0,
        y: 8,
        z: 16,
        w: 24,
    });

    // Naga does not appear to support doubles.
    // https://github.com/gfx-rs/naga/issues/1272

    // test_round_trip_struct(FourF64 {
    //     x: 5.0,
    //     y: 7.0,
    //     z: 9.0,
    //     w: 11.0,
    // });
}

#[test]
fn two_vec3() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct TwoVec3 {
        one: Vector3<f32>,
        two: Vector3<f32>,
    }

    print_std140!(TwoVec3);
    print_std430!(TwoVec3);

    assert_std140!((size = 32, align = 16) TwoVec3 {
        one: 0,
        two: 16,
    });

    assert_std430!((size = 32, align = 16) TwoVec3 {
        one: 0,
        two: 16,
    });

    test_round_trip_struct(TwoVec3 {
        one: [1.0, 2.0, 3.0].into(),
        two: [4.0, 5.0, 6.0].into(),
    });
}

#[test]
fn two_vec4() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct TwoVec4 {
        one: Vector4<f32>,
        two: Vector4<f32>,
    }

    assert_std140!((size = 32, align = 16) TwoVec4 {
        one: 0,
        two: 16,
    });

    test_round_trip_struct(TwoVec4 {
        one: [1.0, 2.0, 3.0, 4.0].into(),
        two: [5.0, 6.0, 7.0, 8.0].into(),
    });
}

#[test]
fn vec3_then_f32() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct Vec3ThenF32 {
        one: Vector3<f32>,
        two: f32,
    }

    assert_std140!((size = 16, align = 16) Vec3ThenF32 {
        one: 0,
        two: 12,
    });

    test_round_trip_struct(Vec3ThenF32 {
        one: [1.0, 2.0, 3.0].into(),
        two: 4.0,
    });
}

#[test]
fn mat3_padding() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct Mat3Padding {
        // Three rows of 16 bytes (3x f32 + 4 bytes padding)
        one: mint::ColumnMatrix3<f32>,
        two: f32,
    }

    assert_std140!((size = 64, align = 16) Mat3Padding {
        one: 0,
        two: 48,
    });

    // Naga produces invalid HLSL for mat3 value.
    // https://github.com/gfx-rs/naga/issues/1466

    // test_round_trip_struct(Mat3Padding {
    //     one: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]].into(),
    //     two: 10.0,
    // });
}

#[test]
fn padding_after_struct() {
    #[derive(AsStd140)]
    struct TwoF32 {
        x: f32,
    }

    #[derive(AsStd140)]
    struct PaddingAfterStruct {
        base_value: TwoF32,
        // There should be 8 bytes of padding inserted here.
        small_field: f32,
    }

    assert_std140!((size = 32, align = 16) PaddingAfterStruct {
        base_value: 0,
        small_field: 16,
    });
}

#[test]
fn proper_offset_calculations_for_differing_member_sizes() {
    #[derive(AsStd140)]
    struct Foo {
        x: f32,
    }

    #[derive(AsStd140)]
    struct Bar {
        first: Foo,
        second: Foo,
    }

    #[derive(AsStd140)]
    struct Outer {
        leading: Bar,
        trailing: Foo,
    }

    // Offset  Size  Contents
    // 0       4     Bar.leading.first.x
    // 4       12    [padding]
    // 16      4     Bar.leading.second.x
    // 20      12    [padding]
    // 32      4     Bar.trailing.x
    // 36      12    [padding]
    //
    // Total size is 48.

    assert_std140!((size = 48, align = 16) Outer {
        leading: 0,
        trailing: 32,
    });
}

#[test]
fn array_strides_small_value() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430)]
    struct ArrayOfSmallValues {
        inner: [f32; 4],
    }

    assert_std140!((size = 64, align = 16) ArrayOfSmallValues {
        inner: 0,
    });

    assert_std430!((size = 16, align = 4) ArrayOfSmallValues {
        inner: 0,
    });
}

#[test]
fn array_strides_vec3() {
    #[derive(Debug, PartialEq, AsStd140, AsStd430, GlslStruct)]
    struct ArrayOfVector3 {
        inner: [Vector3<f32>; 4],
    }

    assert_std140!((size = 64, align = 16) ArrayOfVector3 {
        inner: 0,
    });

    assert_std430!((size = 64, align = 16) ArrayOfVector3 {
        inner: 0,
    });

    test_round_trip_struct(ArrayOfVector3 {
        inner: [
            [0.0, 1.0, 2.0].into(),
            [3.0, 4.0, 5.0].into(),
            [6.0, 7.0, 8.0].into(),
            [9.0, 10.0, 11.0].into(),
        ],
    })
}
