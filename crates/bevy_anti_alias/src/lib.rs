#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

use bevy_app::Plugin;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Local, Query},
};
use bevy_render::view::{StackRole, ViewStackContract, ViewTarget};
use contrast_adaptive_sharpening::CasPlugin;
use fxaa::FxaaPlugin;
use smaa::SmaaPlugin;
use taa::TemporalAntiAliasPlugin;

pub mod contrast_adaptive_sharpening;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
pub mod dlss;
pub mod fxaa;
pub mod smaa;
pub mod taa;

/// Adds fxaa, smaa, taa, contrast aware sharpening, and optional dlss support.
#[derive(Default)]
pub struct AntiAliasPlugin;

/// A render world component for an effect that expects tonemapped input.
trait PostTonemappingEffect: Component {
    const NAME: &'static str;

    fn enabled(&self) -> bool {
        true
    }
}

/// Warns once when a camera runs a [`PostTonemappingEffect`] but a camera that
/// renders later tonemaps its output. The effect then runs on colors that
/// aren't tonemapped yet.
fn warn_effect_before_stack_tonemapping<E: PostTonemappingEffect>(
    views: Query<(Entity, &E, &ViewStackContract), With<ViewTarget>>,
    mut warned: Local<bool>,
) {
    if *warned {
        return;
    }
    for (entity, effect, contract) in &views {
        if effect.enabled() && contract.tonemap == StackRole::HandledByFinalizer {
            tracing::warn!(
                "Camera {entity} uses {}, but a camera that renders later to the same \
                target tonemaps its output, so {} runs on colors that aren't tonemapped \
                yet. Add {} to the last camera that tonemaps instead.",
                E::NAME,
                E::NAME,
                E::NAME,
            );
            *warned = true;
            return;
        }
    }
}

impl Plugin for AntiAliasPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            FxaaPlugin,
            SmaaPlugin,
            TemporalAntiAliasPlugin,
            CasPlugin,
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            dlss::DlssPlugin,
        ));
    }
}
