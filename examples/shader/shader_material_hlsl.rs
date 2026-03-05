//! A custom material that uses an HLSL shader compiled at runtime with `shaderc`.

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, PipelineCache, ShaderLanguage},
    shader::{
        CompiledShader, Shader, ShaderCompileError, ShaderCompiler, ShaderDefVal, ShaderRef,
        ShaderStage,
    },
};

// Required shaderc lib to be available on your system
use shaderc_dyn as shaderc;

const VERTEX_HLSL: &str = "shaders/custom_material.vert.hlsl";
const FRAGMENT_HLSL: &str = "shaders/custom_material.frag.hlsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Register the HLSL asset loader and compiler before MaterialPlugin
            // so shaders are available when pipelines are built.
            HlslShaderPlugin,
            MaterialPlugin::<HlslMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

/// A [`ShaderCompiler`] that compiles HLSL source to SPIR-V using `shaderc`.
struct ShadercHlslCompiler {
    compiler: shaderc::Compiler,
}

#[derive(Debug, thiserror::Error)]
enum HlslShaderCompilerError {
    #[error("Failed to initialize shaderc compiler: {0}")]
    Initialize(shaderc::Error),
    #[error("Failed to create shaderc compile options: {0}")]
    CreateOptions(shaderc::Error),
}

impl ShadercHlslCompiler {
    fn new() -> Result<Self, HlslShaderCompilerError> {
        Ok(Self {
            compiler: shaderc::Compiler::new().map_err(HlslShaderCompilerError::CreateOptions)?,
        })
    }

    fn options() -> Result<shaderc::CompileOptions<'static>, HlslShaderCompilerError> {
        let mut options =
            shaderc::CompileOptions::new().map_err(HlslShaderCompilerError::Initialize)?;

        options.set_source_language(shaderc::SourceLanguage::HLSL);
        options.set_target_env(
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_1 as u32,
        );
        options.set_optimization_level(shaderc::OptimizationLevel::Performance);

        Ok(options)
    }
}

impl ShaderCompiler for ShadercHlslCompiler {
    fn compile(
        &mut self,
        shader: &Shader,
        _shader_defs: &[ShaderDefVal],
    ) -> Result<CompiledShader, ShaderCompileError> {
        let source_text = shader.source.as_str();

        // Read the pipeline stage from the shader source.
        let shader_kind = match shader.source.stage() {
            Some(ShaderStage::Vertex) => shaderc::ShaderKind::Vertex,
            Some(ShaderStage::Fragment) => shaderc::ShaderKind::Fragment,
            Some(ShaderStage::Compute) => shaderc::ShaderKind::Compute,
            None => {
                return Err(ShaderCompileError {
                    message: "HLSL shaders require a pipeline stage. \
                              Use Shader::from_custom_with_stage to set one."
                        .to_string(),
                });
            }
        };

        let artifact = self
            .compiler
            .compile_into_spirv(
                source_text,
                shader_kind,
                "hlsl_shader",
                "main",
                Some(&Self::options().map_err(|e| ShaderCompileError {
                    message: format!("shaderc options error: {e}"),
                })?),
            )
            .map_err(|e| ShaderCompileError {
                message: format!("shaderc compilation error: {e}"),
            })?;

        Ok(CompiledShader::SpirV(artifact.as_binary_u8().to_vec()))
    }
}

/// [`AssetLoader`] for `.vert.hlsl` and `.frag.hlsl` files.
#[derive(Default, TypePath)]
struct HlslShaderLoader;

/// Errors that can occur when loading an HLSL shader.
#[derive(Debug, thiserror::Error)]
enum HlslShaderLoaderError {
    #[error("IO error loading HLSL shader: {0}")]
    Io(#[from] std::io::Error),
    #[error("HLSL shader source is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

impl AssetLoader for HlslShaderLoader {
    type Asset = Shader;
    type Settings = ();
    type Error = HlslShaderLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Shader, Self::Error> {
        let path = load_context.path().to_string();

        let mut source = String::new();
        reader.read_to_string(&mut source).await?;

        let stage = if path.contains(".vert.") {
            ShaderStage::Vertex
        } else {
            ShaderStage::Fragment
        };

        Ok(Shader::from_custom(
            source,
            ShaderLanguage::Custom("hlsl".into()),
            Some(stage),
            path,
        ))
    }

    fn extensions(&self) -> &[&str] {
        &["hlsl"]
    }
}

/// Plugin that registers:
/// - The [`HlslShaderLoader`] asset loader for `.hlsl` files.
/// - The [`ShadercHlslCompiler`] for `ShaderLanguage::Custom("hlsl")`.
///
/// This plugin must be added to the [`App`] before [`MaterialPlugin`].
struct HlslShaderPlugin;

impl Plugin for HlslShaderPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(HlslShaderLoader);
    }

    fn finish(&self, app: &mut App) {
        // PipelineCache is only available in the render world, which is set up after
        // regular `build`. Register the compiler here so it is ready before any
        // pipelines are compiled.
        let render_app = app.sub_app_mut(bevy::render::RenderApp);
        let mut pipeline_cache = render_app.world_mut().resource_mut::<PipelineCache>();

        match ShadercHlslCompiler::new() {
            Ok(compiler) => {
                pipeline_cache.register_shader_compiler(ShaderLanguage::Custom("hlsl"), compiler)
            }
            Err(err) => {
                bevy::log::error!("Failed to create ShadercHlslCompiler compiler: {err:?}")
            }
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HlslMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(HlslMaterial {
            color: LinearRgba::BLUE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Blend,
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// A simple material that reads a uniform color and multiplies it by a texture sample
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct HlslMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

impl Material for HlslMaterial {
    fn vertex_shader() -> ShaderRef {
        VERTEX_HLSL.into()
    }

    fn fragment_shader() -> ShaderRef {
        FRAGMENT_HLSL.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
