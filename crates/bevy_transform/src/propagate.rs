use bevy_hierarchy::Propagate;

use crate::{GlobalTransform, Transform};

impl Propagate for Transform {
    type Computed = GlobalTransform;
    type Payload = GlobalTransform;

    const ALWAYS_PROPAGATE: bool = false;

    #[inline]
    fn compute_root(computed: &mut Self::Computed, local: &Self) {
        *computed = GlobalTransform::from(*local);
    }

    #[inline]
    fn compute(computed: &mut Self::Computed, payload: &Self::Payload, local: &Self) {
        *computed = payload.mul_transform(*local);
    }

    #[inline]
    fn payload(computed: &Self::Computed) -> Self::Payload {
        *computed
    }
}
