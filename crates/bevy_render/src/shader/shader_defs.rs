use crate::{pipeline::RenderPipelines, Texture};
use bevy_app::{EventReader, Events};
use bevy_asset::{Asset, AssetEvent, Assets, Handle, HandleUntyped};
pub use bevy_derive::ShaderDefs;
use bevy_ecs::{Changed, Local, Mut, Query, QuerySet, Res};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_utils::{HashSet, Uuid};

/// Something that can either be "defined" or "not defined". This is used to determine if a "shader def" should be considered "defined"
pub trait ShaderDef {
    fn is_defined(&self) -> bool;
}

/// A collection of "shader defs", which define compile time definitions for shaders.
pub trait ShaderDefs {
    fn shader_defs_len(&self) -> usize;
    fn get_shader_def(&self, index: usize) -> Option<(&str, bool)>;
    fn iter_shader_defs(&self) -> ShaderDefIterator;
}

/// Iterates over all [ShaderDef] items in [ShaderDefs]
pub struct ShaderDefIterator<'a> {
    shader_defs: &'a dyn ShaderDefs,
    index: usize,
}

impl<'a> ShaderDefIterator<'a> {
    pub fn new(shader_defs: &'a dyn ShaderDefs) -> Self {
        Self {
            shader_defs,
            index: 0,
        }
    }
}
impl<'a> Iterator for ShaderDefIterator<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        self.shader_defs.get_shader_def(self.index - 1)
    }
}

impl ShaderDef for bool {
    fn is_defined(&self) -> bool {
        *self
    }
}

impl ShaderDef for Option<Handle<Texture>> {
    fn is_defined(&self) -> bool {
        self.is_some()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Reflect)]
pub enum ShaderDefSource {
    Component(Uuid),
    Asset(HandleUntyped),
}

impl<T: Asset> From<&Handle<T>> for ShaderDefSource {
    fn from(h: &Handle<T>) -> Self {
        Self::Asset(h.clone_weak_untyped())
    }
}

impl From<Uuid> for ShaderDefSource {
    fn from(uuid: Uuid) -> Self {
        Self::Component(uuid)
    }
}

/// Updates [RenderPipelines] with the latest [ShaderDefs]
pub fn shader_defs_system<T>(mut query: Query<(&T, &mut RenderPipelines), Changed<T>>)
where
    T: ShaderDefs + TypeUuid + Send + Sync + 'static,
{
    query
        .iter_mut()
        .map(|(s, p)| (s, (T::TYPE_UUID).into(), p))
        .for_each(update_render_pipelines)
}

fn update_render_pipelines<T>(q: (&T, ShaderDefSource, Mut<RenderPipelines>))
where
    T: ShaderDefs + Send + Sync + 'static,
{
    let (shader_defs, src, mut render_pipelines) = q;

    let new_defs = shader_defs
        .iter_shader_defs()
        // FIX: revert macro
        .filter_map(|(def, defined)| if defined { Some(def.to_string()) } else { None })
        .collect::<Vec<_>>();
    render_pipelines.pipelines.iter_mut().for_each(|p| {
        *(p.specialization
            .shader_specialization
            .shader_defs
            .entry(src.clone())
            .or_default()) = new_defs.clone();
    });
}

// FIX: track entities or clean this up
//#[derive(Default)]
pub struct AssetShaderDefsState<T: Asset> {
    event_reader: EventReader<AssetEvent<T>>,
    //entities: HashMap<Handle<T>, HashSet<Entity>>,
}

impl<T: Asset> Default for AssetShaderDefsState<T> {
    fn default() -> Self {
        Self {
            event_reader: Default::default(),
            //entities: Default::default(),
        }
    }
}

/// Updates [RenderPipelines] with the latest [ShaderDefs] from a given asset type
pub fn asset_shader_defs_system<T>(
    mut state: Local<AssetShaderDefsState<T>>,
    assets: Res<Assets<T>>,
    events: Res<Events<AssetEvent<T>>>,
    mut queries: QuerySet<(
        Query<(&Handle<T>, &mut RenderPipelines)>,
        Query<(&Handle<T>, &mut RenderPipelines), Changed<Handle<T>>>,
    )>,
) where
    T: Default + Asset + ShaderDefs + Send + Sync + 'static,
{
    let changed = state
        .event_reader
        .iter(&events)
        .fold(HashSet::default(), |mut set, event| {
            match event {
                AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                    set.insert(handle.clone_weak());
                }
                AssetEvent::Removed { handle } => {
                    set.remove(&handle);
                }
            }
            set
        });

    // Update for changed assets.
    if changed.len() > 0 {
        queries
            .q0_mut()
            .iter_mut()
            .filter(|(h, _)| changed.contains(h))
            .filter_map(|(h, p)| assets.get(h).map(|a| (a, h.into(), p)))
            .for_each(update_render_pipelines);
    }

    // Update for changed asset handles.
    queries
        .q1_mut()
        .iter_mut()
        // Not worth?
        //.filter(|(h, _)| !changed.contains(h))
        .filter_map(|(h, p)| assets.get(h).map(|a| (a, h.into(), p)))
        .for_each(update_render_pipelines);
}

#[cfg(test)]
mod tests {
    use super::asset_shader_defs_system;
    use super::ShaderDefs;
    use crate::{self as bevy_render, pipeline::RenderPipeline, prelude::RenderPipelines};
    use bevy_app::App;
    use bevy_asset::{AddAsset, AssetPlugin, AssetServer, Assets, HandleId};
    use bevy_core::CorePlugin;
    use bevy_ecs::{Commands, ResMut};
    use bevy_reflect::{ReflectPlugin, TypeUuid};

    #[derive(Debug, Default, ShaderDefs, TypeUuid)]
    #[uuid = "3130b0bf-46a6-42f2-8556-c1a04da20b7e"]
    struct A {
        #[shader_def]
        d: bool,
    }

    fn shader_def_len(app: &App) -> usize {
        app.world
            .query::<&RenderPipelines>()
            .next()
            .unwrap()
            .pipelines[0]
            .specialization
            .shader_specialization
            .shader_defs
            .len()
    }

    #[test]
    fn empty_handle() {
        // Insert an empty asset handle, and empty render pipelines.
        let handle_id = HandleId::random::<A>();
        let setup = move |commands: &mut Commands, asset_server: ResMut<AssetServer>| {
            let h = asset_server.get_handle::<A, HandleId>(handle_id);
            let render_pipelines = RenderPipelines::from_pipelines(vec![RenderPipeline::default()]);
            commands.spawn((h, render_pipelines));
        };

        App::build()
            .add_plugin(ReflectPlugin::default())
            .add_plugin(CorePlugin::default())
            .add_plugin(AssetPlugin::default())
            .add_asset::<A>()
            .add_system(asset_shader_defs_system::<A>)
            .add_startup_system(setup)
            .set_runner(move |mut app: App| {
                app.initialize();
                app.update();
                assert_eq!(shader_def_len(&app), 0);
                {
                    let mut asset_server = app.resources.get_mut::<Assets<A>>().unwrap();
                    asset_server.set(handle_id, A { d: true });
                }

                // Asset changed events are sent post-update, so we
                // have to update twice to see the change.
                app.update();
                app.update();

                assert_eq!(shader_def_len(&app), 1);
            })
            .run();
    }
}
