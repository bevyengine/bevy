use crate::{ResolvedScene, Scene};
use bevy_asset::{Asset, AssetServer, Handle, UntypedHandle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_reflect::TypePath;

#[derive(Asset, TypePath)]
pub struct ScenePatch {
    pub patch: Box<dyn Scene>,
    #[dependency]
    pub dependencies: Vec<UntypedHandle>,
    // TODO: consider breaking this out to prevent mutating asset events when resolved
    pub resolved: Option<ResolvedScene>,
}

impl ScenePatch {
    pub fn load<P: Scene>(assets: &AssetServer, scene: P) -> Self {
        let mut dependencies = Vec::new();
        scene.register_dependencies(&mut dependencies);
        let dependencies = dependencies
            .iter()
            .map(|i| assets.load::<ScenePatch>(i.clone()).untyped())
            .collect::<Vec<_>>();
        ScenePatch {
            patch: Box::new(scene),
            dependencies,
            resolved: None,
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct ScenePatchInstance(pub Handle<ScenePatch>);
