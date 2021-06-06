use std::{cell::Cell, ptr::NonNull};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Asset, AssetServer, Handle};

///////////////////////////////////////////////////////////////////////////////

thread_local! {
    static ASSET_SERVER: Cell<Option<NonNull<AssetServer>>> = Cell::new(None);
}

impl<T: Asset> Serialize for Handle<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let path = ASSET_SERVER.with(|server| {
            server.get().and_then(|ptr| {
                // SAFETY: The thread local [`ASSET_SERVER`] can only
                // be set by a valid [`AssetServer`] instance that will
                // also make sure to set it back to [`None`] once it's no longer is valid
                let server = unsafe { ptr.as_ref() };
                //  TODO: `get_handle_path` does absolutely nothing issue #1290
                server.get_handle_path(self).map(|asset_path| {
                    let mut path = asset_path.path().to_string_lossy().to_string();
                    if let Some(label) = asset_path.label() {
                        path.push('#');
                        path.push_str(label);
                    }
                    path
                })
            })
        });

        path.serialize(serializer)
    }
}

impl<'de, T: Asset> Deserialize<'de> for Handle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<String>::deserialize(deserializer)?
            .and_then(|path| {
                ASSET_SERVER.with(|server| {
                    server.get().map(|ptr| {
                        // SAFETY: The thread local [`ASSET_SERVER`] can only
                        // be set by a valid [`AssetServer`] instance that will
                        // also make sure to set it to back [`None`] once it's no longer is valid
                        let server = unsafe { ptr.as_ref() };
                        server.load(path.as_str())
                    })
                })
            })
            .unwrap_or_default())
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
            key.replace(NonNull::new(self as *const _ as *mut _));
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
            key.replace(NonNull::new(self as *const _ as *mut _));
            let result = T::deserialize(deserializer);
            key.replace(None);
            result
        })
    }
}
