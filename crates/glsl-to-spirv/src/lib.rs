// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate tempfile;

use std::{fs::File, io::Write, process::Command};

pub type SpirvOutput = File;

pub fn compile(
    code: &str,
    ty: ShaderType,
    shader_defs: Option<&[String]>,
) -> Result<SpirvOutput, String> {
    compile_inner(Some((code, ty)), shader_defs)
}

// Eventually the API will look like this, with an iterator for multiple shader stages.
// However for the moment GLSLang doesn't like that, so we only pass one shader at a time.
fn compile_inner<'a, I>(shaders: I, shader_defs: Option<&[String]>) -> Result<SpirvOutput, String>
where
    I: IntoIterator<Item = (&'a str, ShaderType)>,
{
    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("compilation_output.spv");

    let mut command = Command::new(concat!(env!("OUT_DIR"), "/glslang_validator"));
    command.arg("-V");
    command.arg("-l");
    command.arg("-o").arg(&output_file);
    if let Some(shader_defs) = shader_defs {
        for def in shader_defs.iter() {
            command.arg(format!("-D {}", def));
        }
    }

    for (num, (source, ty)) in shaders.into_iter().enumerate() {
        let extension = match ty {
            ShaderType::Vertex => ".vert",
            ShaderType::Fragment => ".frag",
            ShaderType::Geometry => ".geom",
            ShaderType::TessellationControl => ".tesc",
            ShaderType::TessellationEvaluation => ".tese",
            ShaderType::Compute => ".comp",
        };

        let file_path = temp_dir.path().join(format!("{}{}", num, extension));
        File::create(&file_path)
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        command.arg(file_path);
    }

    let output = command
        .output()
        .expect("Failed to execute glslangValidator");

    if output.status.success() {
        let spirv_output = File::open(output_file).expect("failed to open SPIR-V output file");
        return Ok(spirv_output);
    }

    let error1 = String::from_utf8_lossy(&output.stdout);
    let error2 = String::from_utf8_lossy(&output.stderr);
    return Err(error1.into_owned() + &error2);
}

/// Type of shader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
    Compute,
}
