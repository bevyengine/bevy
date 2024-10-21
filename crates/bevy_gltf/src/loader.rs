use serde::{Deserialize, Serialize};

use bevy_asset::{io::Reader, AssetLoader, LoadContext};
use bevy_render::{
    mesh::MeshVertexAttribute, render_asset::RenderAssetUsages, texture::CompressedImageFormats,
};
use bevy_utils::HashMap;

use crate::{Gltf, GltfError};

/// Loads glTF files with all of their data as their corresponding bevy representations.
pub struct GltfLoader {
    /// List of compressed image formats handled by the loader.
    pub supported_compressed_formats: CompressedImageFormats,
    /// Custom vertex attributes that will be recognized when loading a glTF file.
    ///
    /// Keys must be the attribute names as found in the glTF data, which must start with an underscore.
    /// See [this section of the glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview)
    /// for additional details on custom attributes.
    pub custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;
    type Settings = GltfLoaderSettings;
    type Error = GltfError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &GltfLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Gltf, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let file_name = load_context
            .asset_path()
            .path()
            .to_str()
            .ok_or(GltfError::Gltf(gltf::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Gltf file name invalid",
            ))))?
            .to_string();

        Gltf::load_gltf(self, &file_name, &bytes, load_context, settings).await
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

/// Specifies optional settings for processing gltfs at load time. By default, all recognized contents of
/// the gltf will be loaded.
///
/// # Example
///
/// To load a gltf but exclude the cameras, replace a call to `asset_server.load("my.gltf")` with
/// ```no_run
/// # use bevy_asset::{AssetServer, Handle};
/// # use bevy_gltf::*;
/// # let asset_server: AssetServer = panic!();
/// let gltf_handle: Handle<Gltf> = asset_server.load_with_settings(
///     "my.gltf",
///     |s: &mut GltfLoaderSettings| {
///         s.load_cameras = false;
///     }
/// );
/// ```
#[derive(Serialize, Deserialize)]
pub struct GltfLoaderSettings {
    /// If empty, the gltf mesh nodes will be skipped.
    ///
    /// Otherwise, nodes will be loaded and retained in RAM/VRAM according to the active flags.
    pub load_meshes: RenderAssetUsages,
    /// If empty, the gltf materials will be skipped.
    ///
    /// Otherwise, materials will be loaded and retained in RAM/VRAM according to the active flags.
    pub load_materials: RenderAssetUsages,
    /// If true, the loader will spawn cameras for gltf camera nodes.
    pub load_cameras: bool,
    /// If true, the loader will spawn lights for gltf light nodes.
    pub load_lights: bool,
    /// If true, the loader will include the root of the gltf root node.
    pub include_source: bool,
}

impl Default for GltfLoaderSettings {
    fn default() -> Self {
        Self {
            load_meshes: RenderAssetUsages::default(),
            load_materials: RenderAssetUsages::default(),
            load_cameras: true,
            load_lights: true,
            include_source: false,
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{Gltf, GltfAssetLabel, GltfNode, GltfSkin};
    use bevy_app::App;
    use bevy_asset::{
        io::{
            memory::{Dir, MemoryAssetReader},
            AssetSource, AssetSourceId,
        },
        AssetApp, AssetPlugin, AssetServer, Assets, Handle, LoadState,
    };
    use bevy_core::TaskPoolPlugin;
    use bevy_ecs::{system::Resource, world::World};
    use bevy_log::LogPlugin;
    use bevy_render::mesh::{skinning::SkinnedMeshInverseBindposes, MeshPlugin};
    use bevy_scene::ScenePlugin;

    fn test_app(dir: Dir) -> App {
        let mut app = App::new();
        let reader = MemoryAssetReader { root: dir };
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build().with_reader(move || Box::new(reader.clone())),
        )
        .add_plugins((
            LogPlugin::default(),
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
            MeshPlugin,
            crate::GltfPlugin::default(),
        ));

        app.finish();
        app.cleanup();

        app
    }

    const LARGE_ITERATION_COUNT: usize = 10000;

    fn run_app_until(app: &mut App, mut predicate: impl FnMut(&mut World) -> Option<()>) {
        for _ in 0..LARGE_ITERATION_COUNT {
            app.update();
            if predicate(app.world_mut()).is_some() {
                return;
            }
        }

        panic!("Ran out of loops to return `Some` from `predicate`");
    }

    fn load_gltf_into_app(gltf_path: &str, gltf: &str) -> App {
        #[expect(unused)]
        #[derive(Resource)]
        struct GltfHandle(Handle<Gltf>);

        let dir = Dir::default();
        dir.insert_asset_text(Path::new(gltf_path), gltf);
        let mut app = test_app(dir);
        app.update();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<Gltf> = asset_server.load(gltf_path.to_string());
        let handle_id = handle.id();
        app.insert_resource(GltfHandle(handle));
        app.update();
        run_app_until(&mut app, |_world| {
            let load_state = asset_server.get_load_state(handle_id).unwrap();
            match load_state {
                LoadState::Loaded => Some(()),
                LoadState::Failed(err) => panic!("{err}"),
                _ => None,
            }
        });
        app
    }

    #[test]
    fn single_node() {
        let gltf_path = "test.gltf";
        let app = load_gltf_into_app(
            gltf_path,
            r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "TestSingleNode"
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#,
        );
        let asset_server = app.world().resource::<AssetServer>();
        let handle = asset_server.load(gltf_path);
        let gltf_root_assets = app.world().resource::<Assets<Gltf>>();
        let gltf_node_assets = app.world().resource::<Assets<GltfNode>>();
        let gltf_root = gltf_root_assets.get(&handle).unwrap();
        assert!(gltf_root.nodes.len() == 1, "Single node");
        assert!(
            gltf_root.named_nodes.contains_key("TestSingleNode"),
            "Named node is in named nodes"
        );
        let gltf_node = gltf_node_assets
            .get(gltf_root.named_nodes.get("TestSingleNode").unwrap())
            .unwrap();
        assert_eq!(gltf_node.name, "TestSingleNode", "Correct name");
        assert_eq!(gltf_node.index, 0, "Correct index");
        assert_eq!(gltf_node.children.len(), 0, "No children");
        assert_eq!(gltf_node.asset_label(), GltfAssetLabel::Node(0));
    }

    #[test]
    fn node_hierarchy_no_hierarchy() {
        let gltf_path = "test.gltf";
        let app = load_gltf_into_app(
            gltf_path,
            r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "l1"
        },
        {
            "name": "l2"
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#,
        );
        let asset_server = app.world().resource::<AssetServer>();
        let handle = asset_server.load(gltf_path);
        let gltf_root_assets = app.world().resource::<Assets<Gltf>>();
        let gltf_node_assets = app.world().resource::<Assets<GltfNode>>();
        let gltf_root = gltf_root_assets.get(&handle).unwrap();
        let result = gltf_root
            .nodes
            .iter()
            .map(|h| gltf_node_assets.get(h).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 0);
        assert_eq!(result[1].name, "l2");
        assert_eq!(result[1].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_simple_hierarchy() {
        let gltf_path = "test.gltf";
        let app = load_gltf_into_app(
            gltf_path,
            r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "l1",
            "children": [1]
        },
        {
            "name": "l2"
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#,
        );
        let asset_server = app.world().resource::<AssetServer>();
        let handle = asset_server.load(gltf_path);
        let gltf_root_assets = app.world().resource::<Assets<Gltf>>();
        let gltf_node_assets = app.world().resource::<Assets<GltfNode>>();
        let gltf_root = gltf_root_assets.get(&handle).unwrap();
        let result = gltf_root
            .nodes
            .iter()
            .map(|h| gltf_node_assets.get(h).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[1].name, "l2");
        assert_eq!(result[1].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_hierarchy() {
        let gltf_path = "test.gltf";
        let app = load_gltf_into_app(
            gltf_path,
            r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "l1",
            "children": [1]
        },
        {
            "name": "l2",
            "children": [2]
        },
        {
            "name": "l3",
            "children": [3, 4, 5]
        },
        {
            "name": "l4",
            "children": [6]
        },
        {
            "name": "l5"
        },
        {
            "name": "l6"
        },
        {
            "name": "l7"
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#,
        );
        let asset_server = app.world().resource::<AssetServer>();
        let handle = asset_server.load(gltf_path);
        let gltf_root_assets = app.world().resource::<Assets<Gltf>>();
        let gltf_node_assets = app.world().resource::<Assets<GltfNode>>();
        let gltf_root = gltf_root_assets.get(&handle).unwrap();
        let result = gltf_root
            .nodes
            .iter()
            .map(|h| gltf_node_assets.get(h).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(result.len(), 7);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[1].name, "l2");
        assert_eq!(result[1].children.len(), 1);
        assert_eq!(result[2].name, "l3");
        assert_eq!(result[2].children.len(), 3);
        assert_eq!(result[3].name, "l4");
        assert_eq!(result[3].children.len(), 1);
        assert_eq!(result[4].name, "l5");
        assert_eq!(result[4].children.len(), 0);
        assert_eq!(result[5].name, "l6");
        assert_eq!(result[5].children.len(), 0);
        assert_eq!(result[6].name, "l7");
        assert_eq!(result[6].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_cyclic() {
        let gltf_path = "test.gltf";
        let gltf_str = r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "l1",
            "children": [1]
        },
        {
            "name": "l2",
            "children": [0]
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#;

        let dir = Dir::default();
        dir.insert_asset_text(Path::new(gltf_path), gltf_str);
        let mut app = test_app(dir);
        app.update();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<Gltf> = asset_server.load(gltf_path);
        let handle_id = handle.id();
        app.update();
        run_app_until(&mut app, |_world| {
            let load_state = asset_server.get_load_state(handle_id).unwrap();
            if load_state.is_failed() {
                Some(())
            } else {
                None
            }
        });
        let load_state = asset_server.get_load_state(handle_id).unwrap();
        assert!(load_state.is_failed());
    }

    #[test]
    fn node_hierarchy_missing_node() {
        let gltf_path = "test.gltf";
        let gltf_str = r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "l1",
            "children": [2]
        },
        {
            "name": "l2"
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#;

        let dir = Dir::default();
        dir.insert_asset_text(Path::new(gltf_path), gltf_str);
        let mut app = test_app(dir);
        app.update();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<Gltf> = asset_server.load(gltf_path);
        let handle_id = handle.id();
        app.update();
        run_app_until(&mut app, |_world| {
            let load_state = asset_server.get_load_state(handle_id).unwrap();
            if load_state.is_failed() {
                Some(())
            } else {
                None
            }
        });
        let load_state = asset_server.get_load_state(handle_id).unwrap();
        assert!(load_state.is_failed());
    }

    #[test]
    fn skin_node() {
        let gltf_path = "test.gltf";
        let app = load_gltf_into_app(
            gltf_path,
            r#"
{
    "asset": {
        "version": "2.0"
    },
    "nodes": [
        {
            "name": "skinned",
            "skin": 0,
            "children": [1, 2]
        },
        {
            "name": "joint1"
        },
        {
            "name": "joint2"
        }
    ],
    "skins": [
        {
            "inverseBindMatrices": 0,
            "joints": [1, 2]
        }
    ],
    "buffers": [
        {
            "uri" : "data:application/gltf-buffer;base64,AACAPwAAAAAAAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAAAAAAAAAAAAACAPwAAgD8AAAAAAAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIC/AAAAAAAAgD8=",
            "byteLength" : 128
        }
    ],
    "bufferViews": [
        {
            "buffer": 0,
            "byteLength": 128
        }
    ],
    "accessors": [
        {
            "bufferView" : 0,
            "componentType" : 5126,
            "count" : 2,
            "type" : "MAT4"
        }
    ],
    "scene": 0,
    "scenes": [{ "nodes": [0] }]
}
"#,
        );
        let asset_server = app.world().resource::<AssetServer>();
        let handle = asset_server.load(gltf_path);
        let gltf_root_assets = app.world().resource::<Assets<Gltf>>();
        let gltf_node_assets = app.world().resource::<Assets<GltfNode>>();
        let gltf_skin_assets = app.world().resource::<Assets<GltfSkin>>();
        let gltf_inverse_bind_matrices = app
            .world()
            .resource::<Assets<SkinnedMeshInverseBindposes>>();
        let gltf_root = gltf_root_assets.get(&handle).unwrap();

        assert_eq!(gltf_root.skins.len(), 1);
        assert_eq!(gltf_root.nodes.len(), 3);

        let skin = gltf_skin_assets.get(&gltf_root.skins[0]).unwrap();
        assert_eq!(skin.joints.len(), 2);
        assert_eq!(skin.joints[0], gltf_root.nodes[1]);
        assert_eq!(skin.joints[1], gltf_root.nodes[2]);
        assert!(gltf_inverse_bind_matrices.contains(&skin.inverse_bind_matrices));

        let skinned_node = gltf_node_assets.get(&gltf_root.nodes[0]).unwrap();
        assert_eq!(skinned_node.name, "skinned");
        assert_eq!(skinned_node.children.len(), 2);
        assert_eq!(skinned_node.skin.as_ref(), Some(&gltf_root.skins[0]));
    }
}
