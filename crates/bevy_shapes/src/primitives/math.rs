use crate::primitives::{Primitive2d, Primitive3d};
use bevy_math::{Dir2, Dir3, Dir3A};

impl Primitive2d for Dir2 {}
impl Primitive3d for Dir3 {}
impl Primitive3d for Dir3A {}
