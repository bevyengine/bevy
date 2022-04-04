easy_impl! {
    Vec2 cgmath::Vector2<f32> { x, y },
    Vec3 cgmath::Vector3<f32> { x, y, z },
    Vec4 cgmath::Vector4<f32> { x, y, z, w },

    IVec2 cgmath::Vector2<i32> { x, y },
    IVec3 cgmath::Vector3<i32> { x, y, z },
    IVec4 cgmath::Vector4<i32> { x, y, z, w },

    UVec2 cgmath::Vector2<u32> { x, y },
    UVec3 cgmath::Vector3<u32> { x, y, z },
    UVec4 cgmath::Vector4<u32> { x, y, z, w },

    // bool vectors are disabled due to https://github.com/LPGhatguy/crevice/issues/36
    // BVec2 cgmath::Vector2<bool> { x, y },
    // BVec3 cgmath::Vector3<bool> { x, y, z },
    // BVec4 cgmath::Vector4<bool> { x, y, z, w },

    DVec2 cgmath::Vector2<f64> { x, y },
    DVec3 cgmath::Vector3<f64> { x, y, z },
    DVec4 cgmath::Vector4<f64> { x, y, z, w },

    Mat2 cgmath::Matrix2<f32> { x, y },
    Mat3 cgmath::Matrix3<f32> { x, y, z },
    Mat4 cgmath::Matrix4<f32> { x, y, z, w },

    DMat2 cgmath::Matrix2<f64> { x, y },
    DMat3 cgmath::Matrix3<f64> { x, y, z },
    DMat4 cgmath::Matrix4<f64> { x, y, z, w },
}
