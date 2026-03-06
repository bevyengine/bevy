//! An example showcasing how to setup a custom shader compiler

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, PipelineCache, ShaderLanguage},
    shader::{
        CompiledShader, Shader, ShaderCompileError, ShaderCompiler, ShaderDefVal, ShaderKind,
        ShaderRef,
    },
};

const VERTEX_SHADER: &str = "path/to/vertex/shader.vertex";
const FRAGMENT_SHADER: &str = "path/to/fragment/shader.fragment";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Register the custom shader asset loader and compiler before MaterialPlugin
            // so shaders are available when pipelines are built.
            CustomShaderPlugin,
            MaterialPlugin::<CustomShaderMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

struct CustomShaderCompiler;

impl ShaderCompiler for CustomShaderCompiler {
    fn compile(
        &mut self,
        shader: &Shader,
        _shader_defs: &[ShaderDefVal],
    ) -> Result<CompiledShader, ShaderCompileError> {
        let _source_text = shader.source.as_str();

        // do the compilation here
        // you can use shaderc for this

        Ok(CompiledShader::SpirV(vec![]))
    }
}

/// [`AssetLoader`] for our custom shaders files.
#[derive(Default, TypePath)]
struct CustomShaderLoader;

/// Errors that can occur when loading our custom shader lang file.
#[derive(Debug, thiserror::Error)]
enum CustomShaderLoaderError {
    #[error("IO error loading HLSL shader: {0}")]
    Io(#[from] std::io::Error),
    #[error("Shader source is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

impl AssetLoader for CustomShaderLoader {
    type Asset = Shader;
    type Settings = ();
    type Error = CustomShaderLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Shader, Self::Error> {
        let path = load_context.path().to_string();

        let mut source = String::new();
        reader.read_to_string(&mut source).await?;

        let stage = if path.contains(".vertex") {
            ShaderKind::Vertex
        } else {
            ShaderKind::Fragment
        };

        Ok(Shader::from_custom(
            source,
            ShaderLanguage::Custom("custom"),
            Some(stage),
            path,
        ))
    }

    fn extensions(&self) -> &[&str] {
        &[".vertex", ".fragment"]
    }
}

/// Plugin that registers:
/// - The [`CustomShaderLoader`] asset loader for `.vertex` & `.fragment` files.
/// - The [`CustomCompiler`] for `ShaderLanguage::Custom("custom")`.
///
/// This plugin must be added to the [`App`] before [`MaterialPlugin`].
struct CustomShaderPlugin;

impl Plugin for CustomShaderPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(CustomShaderLoader);
    }

    fn finish(&self, app: &mut App) {
        // PipelineCache is only available in the render world, which is set up after
        // regular `build`. Register the compiler here so it is ready before any
        // pipelines are compiled.
        let render_app = app.sub_app_mut(bevy::render::RenderApp);
        let mut pipeline_cache = render_app.world_mut().resource_mut::<PipelineCache>();

        pipeline_cache
            .register_shader_compiler(ShaderLanguage::Custom("custom"), CustomShaderCompiler);
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomShaderMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(CustomShaderMaterial {
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

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct CustomShaderMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

impl Material for CustomShaderMaterial {
    fn vertex_shader() -> ShaderRef {
        VERTEX_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        FRAGMENT_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
