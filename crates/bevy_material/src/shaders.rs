use bevy_asset::{AssetPath, AssetServer, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_render::render_resource::Shader;
use bevy_utils::HashMap;

use crate::material_pipeline::MaterialPipeline;

#[derive(Deref, DerefMut, Clone)]
pub struct Shaders<P: MaterialPipeline> {
    shaders: HashMap<P::ShaderKey, AssetPath<'static>>,
}

impl<P: MaterialPipeline> Default for Shaders<P> {
    fn default() -> Self {
        Self {
            shaders: Default::default(),
        }
    }
}

impl<P: MaterialPipeline, A: Into<AssetPath<'static>>> FromIterator<(P::ShaderKey, A)>
    for Shaders<P>
{
    fn from_iter<T: IntoIterator<Item = (P::ShaderKey, A)>>(iter: T) -> Self {
        Self {
            shaders: iter
                .into_iter()
                .map(|(key, path)| (key, path.into()))
                .collect(),
        }
    }
}

impl<P: MaterialPipeline> Shaders<P> {
    pub fn new(iter: impl IntoIterator<Item = (P::ShaderKey, AssetPath<'static>)>) -> Self {
        Self::from_iter(iter)
    }

    pub fn extend(&mut self, other: Shaders<P>) {
        self.shaders.extend(other.shaders.into_iter());
    }

    pub fn load(self, asset_server: &AssetServer) -> LoadedShaders<P> {
        self.shaders
            .into_iter()
            .map(|(key, path)| (key, asset_server.load(path)))
            .collect()
    }
}

#[derive(Deref)]
pub struct LoadedShaders<P: MaterialPipeline> {
    shaders: HashMap<P::ShaderKey, Handle<Shader>>,
}

impl<P: MaterialPipeline> FromIterator<(P::ShaderKey, Handle<Shader>)> for LoadedShaders<P> {
    fn from_iter<T: IntoIterator<Item = (P::ShaderKey, Handle<Shader>)>>(iter: T) -> Self {
        Self {
            shaders: iter
                .into_iter()
                .map(|(key, path)| (key, path.into()))
                .collect(),
        }
    }
}

impl<P: MaterialPipeline> LoadedShaders<P> {
    pub fn new(iter: impl IntoIterator<Item = (P::ShaderKey, Handle<Shader>)>) -> Self {
        Self::from_iter(iter)
    }

    pub fn extend(&mut self, other: LoadedShaders<P>) {
        self.shaders.extend(other.shaders.into_iter());
    }
}
