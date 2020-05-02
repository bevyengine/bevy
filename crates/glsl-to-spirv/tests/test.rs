// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate glsl_to_spirv;

#[test]
fn test1() {
    let shader = r#"
#version 330

layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4(1.0);
}
"#;

    glsl_to_spirv::compile(shader, glsl_to_spirv::ShaderType::Fragment, None).unwrap();
}
