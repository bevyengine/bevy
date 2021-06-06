use std::cell::Cell;

use bevy_reflect::Uuid;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Asset, AssetServer, Handle, HandleId, HandleUntyped};

///////////////////////////////////////////////////////////////////////////////

thread_local! {
    static ASSET_SERVER: Cell<Option<AssetServer>> = Cell::new(None);
}

#[derive(Serialize, Deserialize)]
enum AssetRef {
    Default,
    /// Used for static handles like the `PBR_PIPELINE_HANDLE` or a embedded assets
    Local(Uuid, u64),
    /// Loads form a file
    External(String),
}

impl<T: Asset> Serialize for Handle<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let path = ASSET_SERVER
            .with(|cell| {
                let server = cell.replace(None);
                let path = server.as_ref().and_then(|server| {
                    // TODO: `get_handle_path` does absolutely nothing issue #1290
                    server
                        .get_handle_path(self)
                        .map(|asset_path| {
                            let mut path = asset_path.path().to_string_lossy().to_string();
                            if let Some(label) = asset_path.label() {
                                path.push('#');
                                path.push_str(label);
                            }
                            path
                        })
                        .and_then(|path| Some(AssetRef::External(path)))
                });
                cell.replace(server);
                path
            })
            .unwrap_or_else(|| match &self.id {
                HandleId::Id(type_uuid, id) => AssetRef::Local(*type_uuid, *id),
                HandleId::AssetPathId(_) => AssetRef::Default,
            });

        path.serialize(serializer)
    }
}

impl<'de, T: Asset> Deserialize<'de> for Handle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match AssetRef::deserialize(deserializer)? {
            AssetRef::Default => Ok(Handle::default()),
            AssetRef::Local(type_uuid, id) => {
                Ok(HandleUntyped::weak_from_u64(type_uuid, id).typed())
            }
            AssetRef::External(path) => ASSET_SERVER.with(|cell| {
                let server = cell.replace(None);
                let handle = server
                    .as_ref()
                    .map(|server| server.load(path.as_str()))
                    .unwrap_or_default();
                cell.replace(server);
                Ok(handle)
            }),
        }
    }
}

impl AssetServer {
    pub fn serialize_with_asset_refs<S, T>(
        &self,
        serializer: S,
        value: &T,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        ASSET_SERVER.with(|key| {
            key.replace(Some(self.clone()));
            let result = value.serialize(serializer);
            key.replace(None);
            result
        })
    }

    pub fn deserialize_with_asset_refs<'de, D, T>(&self, deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        ASSET_SERVER.with(|key| {
            key.replace(Some(self.clone()));
            let result = T::deserialize(deserializer);
            key.replace(None);
            result
        })
    }
}
