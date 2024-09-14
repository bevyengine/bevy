use crate::{bundle::Bundle, prelude::Trigger, system::In};

/// System input
pub trait SystemInput: Sized {
    /// The input type that is passed to system run functions.
    type In<'a>: SystemInput;
}

impl SystemInput for () {
    type In<'a> = ();
}

impl SystemInput for bool {
    type In<'a> = bool;
}

impl<T: 'static> SystemInput for &'_ T {
    type In<'a> = &'a T;
}

impl<T: 'static> SystemInput for &'_ mut T {
    type In<'a> = &'a mut T;
}

impl<T: 'static> SystemInput for Option<T> {
    type In<'a> = Option<T>;
}

impl<T> SystemInput for In<T> {
    type In<'a> = In<T>;
}

impl<E: 'static, B: Bundle> SystemInput for Trigger<'_, E, B> {
    type In<'a> = Trigger<'a, E, B>;
}
