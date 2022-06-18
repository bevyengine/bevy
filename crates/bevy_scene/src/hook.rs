//! Systems to insert components on loaded scenes.
//!
//! Please see the [`SceneHook`] documentation for detailed examples.

use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::{Without, World},
    system::{Commands, EntityCommands, Query, Res},
    world::EntityRef,
};

use crate::{SceneInstance, SceneSpawner};

/// Marker Component for scenes that were hooked.
#[derive(Component, Debug)]
#[non_exhaustive]
pub struct SceneHooked;

/// Add this as a component to any entity to trigger `hook`'s
/// [`Hook::hook_entity`] method when the scene is loaded.
///
/// You can use it to add your own non-serializable components to entites
/// present in a scene file.
///
/// A typical usage is adding animation, physics collision data or marker
/// components to a scene spawned from a file format that do not support it.
///
/// # Example
///
///  ```rust
/// # use bevy_ecs::{system::Res, component::Component, system::Commands};
/// # use bevy_asset::AssetServer;
/// # use bevy_utils::default;
/// use bevy_scene::{Hook, SceneHook, SceneBundle};
/// # #[derive(Component)]
/// # struct Name; impl Name { fn as_str(&self) -> &str { todo!() } }
/// enum PileType { Drawing }
///
/// #[derive(Component)]
/// struct Pile(PileType);
///
/// #[derive(Component)]
/// struct Card;
///
/// fn load_scene(mut cmds: Commands, asset_server: Res<AssetServer>) {
///     cmds.spawn_bundle(SceneBundle {
///         scene: asset_server.load("scene.glb#Scene0"),
///         hook: SceneHook::new_fn(|entity, cmds| {
///             match entity.get::<Name>().map(|t|t.as_str()) {
///                 Some("Pile") => cmds.insert(Pile(PileType::Drawing)),
///                 Some("Card") => cmds.insert(Card),
///                 _ => cmds,
///             };
///         }),
///         ..default()
///     });
/// }
/// ```
#[derive(Component, Default)]
pub struct SceneHook {
    hook: Option<Box<dyn Hook>>,
}
impl<T: Hook> From<T> for SceneHook {
    fn from(hook: T) -> Self {
        Self::new(hook)
    }
}
impl SceneHook {
    /// Add a hook to a scene, to run for each entities when the scene is
    /// loaded, closures implement `Hook`.
    ///
    ///  You can also implement [`Hook`] on your own types and provide one. Note
    ///  that strictly speaking, you might as well pass a closure. Please check
    ///  the [`Hook`] trait documentation for details.
    pub fn new<T: Hook>(hook: T) -> Self {
        Self {
            hook: Some(Box::new(hook)),
        }
    }

    /// Same as [`Self::new`] but with type bounds to make it easier to
    /// use a closure.
    pub fn new_fn<F: Fn(&EntityRef, &mut EntityCommands) + Send + Sync + 'static>(hook: F) -> Self {
        Self::new(hook)
    }

    /// Add a closure with component parameter as hook.
    ///
    /// This is useful if you only care about a specific component to identify
    /// individual entities of your scene, rather than every possible components.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bevy_ecs::{
    ///     world::EntityRef, component::Component,
    ///     system::{Commands, Res, EntityCommands}
    /// # };
    /// # use bevy_asset::{AssetServer, Handle};
    /// # use bevy_utils::default;
    /// # use bevy_scene::Scene;
    /// use bevy_scene::{Hook, SceneHook, SceneBundle};
    /// # #[derive(Component)] struct Name;
    /// # type DeckData = Scene;
    /// #[derive(Clone)]
    /// struct DeckAssets { player: Handle<DeckData>, oppo: Handle<DeckData> }
    ///
    /// fn hook(decks: &DeckAssets, name: &Name, cmds: &mut EntityCommands) {}
    /// fn load_scene(mut cmds: Commands, decks: Res<DeckAssets>, assets: Res<AssetServer>) {
    ///     let decks = decks.clone();
    ///     cmds.spawn_bundle(SceneBundle {
    ///         scene: assets.load("scene.glb#Scene0"),
    ///         hook: SceneHook::new_comp(move |name, cmds| hook(&decks, name, cmds)),
    ///         ..default()
    ///     });
    /// }
    /// ```
    pub fn new_comp<C, F>(hook: F) -> Self
    where
        F: Fn(&C, &mut EntityCommands) + Send + Sync + 'static,
        C: Component,
    {
        let hook = move |e: &EntityRef, cmds: &mut EntityCommands| match e.get::<C>() {
            Some(comp) => hook(comp, cmds),
            None => {}
        };
        Self::new(hook)
    }
}

/// Handle adding components to entites named in a loaded scene.
///
/// The [`hook_entity`][Hook::hook_entity] method is called once per Entity
/// added in a scene in the [`run_hooks`] system.
pub trait Hook: Send + Sync + 'static {
    /// Add [`Component`]s or do anything with entity in the spawned scene
    /// refered by `entity_ref`.
    ///
    /// This runs once for all entities in the spawned scene, once loaded.
    fn hook_entity(&self, entity_ref: &EntityRef, commands: &mut EntityCommands);
}

/// Run once [`SceneHook`]s added to [`SceneBundle`](crate::SceneBundle) or
/// [`DynamicSceneBundle`](crate::DynamicSceneBundle) when the scenes are loaded.
pub fn run_hooks(
    unloaded_instances: Query<(Entity, &SceneInstance, &SceneHook), Without<SceneHooked>>,
    scene_manager: Res<SceneSpawner>,
    world: &World,
    mut cmds: Commands,
) {
    for (entity, instance, hooked) in unloaded_instances.iter() {
        if let Some(entities) = scene_manager.iter_instance_entities(**instance) {
            if let Some(hook) = hooked.hook.as_deref() {
                for entity_ref in entities.filter_map(|e| world.get_entity(e)) {
                    let mut cmd = cmds.entity(entity_ref.id());
                    hook.hook_entity(&entity_ref, &mut cmd);
                }
            }
            cmds.entity(entity).insert(SceneHooked);
        }
    }
}
impl<F: Fn(&EntityRef, &mut EntityCommands) + Send + Sync + 'static> Hook for F {
    fn hook_entity(&self, entity_ref: &EntityRef, commands: &mut EntityCommands) {
        (self)(entity_ref, commands);
    }
}
