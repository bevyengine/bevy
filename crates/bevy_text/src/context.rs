use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::resource::Resource;
use parley::FontContext;
use parley::LayoutContext;

/// Font context
#[derive(Resource, Default, Deref, DerefMut)]
pub struct FontCx(pub FontContext);

/// Text layout context
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LayoutCx(pub LayoutContext);

/// Text scaler context
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ScaleCx(pub LayoutContext);
