use crate::{impl_property, AsProperties, Properties, Property};
use glam::{Mat3, Mat4, Quat, Vec2, Vec3};
use std::any::Any;

impl_property!(Vec2);
impl_property!(Vec3);
impl_property!(Mat3);
impl_property!(Mat4);
impl_property!(Quat);
