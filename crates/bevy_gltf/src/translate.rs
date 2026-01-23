use crate::{GltfAssetLabel, GltfMaterial};
use bevy_asset::{LoadContext, UntypedHandle};
use bevy_ecs::{prelude::*, resource::Resource};
use bevy_platform::sync::Arc;

/// Utility for letting renderers translate `GltfMaterial` into their own material type. The renderer
/// should add this as a resource during `Plugin::build`.
#[derive(Resource, Clone)]
pub struct GltfMaterialTranslator {
    /// Create a material asset from a `GltfMaterial` and a label.
    pub load_material: Arc<
        dyn Fn(&GltfMaterial, &GltfAssetLabel, &mut LoadContext) -> Result<UntypedHandle, BevyError>
            + Send
            + Sync,
    >,
    /// Insert a material component using a label that was previously passed to `load_material`.
    pub insert_material: Arc<
        dyn Fn(&GltfAssetLabel, &mut LoadContext, &mut EntityWorldMut) -> Result<(), BevyError>
            + Send
            + Sync,
    >,
}
