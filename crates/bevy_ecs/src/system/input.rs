use crate::{bundle::Bundle, prelude::Trigger, system::In};

/// System input
pub trait SystemInput: 'static {
    /// Smuggled associated type
    type In<'a>;
}

impl SystemInput for () {
    type In<'a> = ();
}

impl<T: 'static> SystemInput for In<T> {
    type In<'a> = T;
}

impl<T> SystemInput for &'static T {
    type In<'a> = &'a T;
}

impl<T> SystemInput for &'static mut T {
    type In<'a> = &'a mut T;
}

impl<E, B: Bundle> SystemInput for Trigger<'static, E, B> {
    type In<'a> = Trigger<'a, E, B>;
}
