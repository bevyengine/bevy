use crate::{
    core_2d::graph::Core2d,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{Camera, CameraProjection, CameraRenderGraph, OrthographicProjection, Projection},
    extract_component::ExtractComponent,
    primitives::Frustum,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// A 2D camera component. Enables the 2D render graph for a [`Camera`].
#[derive(Component, Default, Reflect, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component, Default)]
#[require(
    Camera,
    DebandDither,
    CameraRenderGraph(|| CameraRenderGraph::new(Core2d)),
    Projection(|| Projection::Orthographic(OrthographicProjection::default_2d())),
    Frustum(|| OrthographicProjection::default_2d().compute_frustum(&GlobalTransform::from(Transform::default()))),
    Tonemapping(|| Tonemapping::None),
)]
pub struct Camera2d;
