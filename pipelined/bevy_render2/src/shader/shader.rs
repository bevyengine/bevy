use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_reflect::{TypeUuid, Uuid};
use bevy_utils::{tracing::error, BoxedFuture};
use naga::{valid::ModuleInfo, Module};
use std::{borrow::Cow, marker::Copy};
use thiserror::Error;
use wgpu::{ShaderFlags, ShaderModuleDescriptor, ShaderSource};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ShaderId(Uuid);

impl ShaderId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ShaderId(Uuid::new_v4())
    }
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error(transparent)]
    WgslParse(#[from] naga::front::wgsl::ParseError),
    #[error(transparent)]
    SpirVParse(#[from] naga::front::spv::Error),
    #[error(transparent)]
    Validation(#[from] naga::valid::ValidationError),
}

/// A shader, as defined by its [ShaderSource] and [ShaderStage]
#[derive(Debug, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub enum Shader {
    Wgsl(Cow<'static, str>),
    SpirV(Vec<u8>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

pub struct ShaderReflection {
    pub module: Module,
    pub module_info: ModuleInfo,
}

impl ShaderReflection {
    pub fn get_spirv(&self) -> Result<Vec<u32>, naga::back::spv::Error> {
        naga::back::spv::write_vec(
            &self.module,
            &self.module_info,
            &naga::back::spv::Options {
                flags: naga::back::spv::WriterFlags::empty(),
                ..naga::back::spv::Options::default()
            },
        )
    }
}

impl Shader {
    pub fn reflect(&self) -> Result<ShaderReflection, ShaderReflectError> {
        let module = match &self {
            // TODO: process macros here
            Shader::Wgsl(source) => naga::front::wgsl::parse_str(&source)?,
            Shader::SpirV(source) => naga::front::spv::parse_u8_slice(
                &source,
                &naga::front::spv::Options {
                    adjust_coordinate_space: false,
                    ..naga::front::spv::Options::default()
                },
            )?,
        };
        let module_info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::default(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)?;

        Ok(ShaderReflection {
            module,
            module_info,
        })
    }

    pub fn from_wgsl(source: impl Into<Cow<'static, str>>) -> Shader {
        Shader::Wgsl(source.into())
    }

    pub fn from_spirv(source: Vec<u8>) -> Shader {
        Shader::SpirV(source)
    }
}

#[derive(Default)]
pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let shader = match ext {
                "spv" => Shader::from_spirv(Vec::from(bytes)),
                "wgsl" => Shader::from_wgsl(String::from_utf8(Vec::from(bytes))?),
                _ => panic!("unhandled extension: {}", ext),
            };

            load_context.set_default_asset(LoadedAsset::new(shader));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["spv", "wgsl"]
    }
}

impl<'a> From<&'a Shader> for ShaderModuleDescriptor<'a> {
    fn from(shader: &'a Shader) -> Self {
        ShaderModuleDescriptor {
            flags: ShaderFlags::default(),
            label: None,
            source: match shader {
                Shader::Wgsl(source) => ShaderSource::Wgsl(source.clone()),
                Shader::SpirV(_) => {
                    // TODO: we can probably just transmute the u8 array to u32?
                    let x = shader.reflect().unwrap();
                    let spirv = x.get_spirv().unwrap();
                    ShaderSource::SpirV(Cow::Owned(spirv))
                }
            },
        }
    }
}
