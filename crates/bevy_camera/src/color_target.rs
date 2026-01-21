use alloc::sync::Arc;
use bevy_asset::Handle;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{component::Component, entity::Entity};
use bevy_image::Image;
use bevy_reflect::Reflect;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The main color target used by camera in most render passes.
///
/// 1. In main passes, objects are rendered to `main_a` (or `main_b`, depends on `main_target_flag`). If `multisampled` texture is provided, then MSAA will be enabled and resolved to `main_a`.
/// 2. In post process, `main_b` should be provided as `main_a` need to be written to `main_a` and then swapped during `post_process_write`.
/// 3. Finally, in upscaling pass, the current main color target is written to [`RenderTarget`](crate::RenderTarget).
#[derive(Component, Debug, Clone)]
pub struct MainColorTarget {
    pub main_a: Handle<Image>,
    pub main_b: Option<Handle<Image>>,
    pub multisampled: Option<Handle<Image>>,
    pub main_target_flag: Option<Arc<AtomicUsize>>,
}

impl MainColorTarget {
    pub fn new(
        main_a: Handle<Image>,
        main_b: Option<Handle<Image>>,
        multisampled: Option<Handle<Image>>,
    ) -> Self {
        let main_target = main_b.as_ref().map(|_| Arc::new(AtomicUsize::new(0)));
        Self {
            main_a,
            main_b,
            multisampled,
            main_target_flag: main_target,
        }
    }

    pub fn current_target(&self) -> &Handle<Image> {
        if let Some(main_target) = &self.main_target_flag
            && main_target.load(Ordering::SeqCst) == 1
        {
            self.main_b.as_ref().unwrap()
        } else {
            &self.main_a
        }
    }

    pub fn other_target(&self) -> Option<&Handle<Image>> {
        let Some(main_target) = &self.main_target_flag else {
            return None;
        };
        Some(if main_target.load(Ordering::SeqCst) == 1 {
            &self.main_a
        } else {
            self.main_b.as_ref().unwrap()
        })
    }
}

/// Add this component to camera to opt-out auto configuring [`WithMainColorTarget`].
///
/// Specifically, opt-out spawning separate [`MainColorTarget`] for each camera and syncing it with [`Hdr`], [`Msaa`] and [`CameraMainTextureUsages`], otherwise these 3 componets have no effect.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct NoAutoConfiguredMainColorTarget;

/// Link this camera to a [`MainColorTarget`] entity.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target  = MainColorTargetCameras)]
#[reflect(Component)]
pub struct WithMainColorTarget(pub Entity);

/// The cameras that are using this [`MainColorTarget`].
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship  = WithMainColorTarget, linked_spawn)]
#[reflect(Component)]
pub struct MainColorTargetCameras(Vec<Entity>);
