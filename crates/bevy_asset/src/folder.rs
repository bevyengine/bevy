use std::any::TypeId;

use crate::{self as bevy_asset, Assets, Handle};
use crate::{Asset, UntypedHandle};
use bevy_ecs::system::Res;
use bevy_reflect::TypePath;

/// A "loaded folder" containing handles for all assets stored in a given [`AssetPath`].
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedFolder {
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}

/// loads assets of type T from a given [`LoadedFolder`] handle. Returns None if the folder inaccessible. 
/// 
/// [`LoadedFolder`]: crate::LoadedFolder
pub fn load_assets_in<T: Asset>(
    folders: &Res<Assets<LoadedFolder>>,
    folder_handle: &Handle<LoadedFolder>
) -> Option<Vec<Handle<T>>>{
    let typeid = TypeId::of::<T>();

     if let Some(folder) = folders.get(folder_handle) {
        let handles: Vec<Handle<T>> = folder
        .handles
        .clone()
        .into_iter()
        .filter(|handle| handle.type_id() == typeid)
        .map(|handle| handle.typed::<T>())
        .collect::<Vec<_>>();
        Some(handles)
     } else {
         None
     }
}