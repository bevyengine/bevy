use crate::{
    bundle::Bundle,
    prelude::Trigger,
    system::{In, System},
};

/// System input
pub trait SystemInput: Sized {
    /// The outer input type that is defined as the first argument to systems,
    /// similar to [`SystemParam`](crate::system::SystemParam)s.
    type Param<'i>: SystemInput;
    /// The inner input type that is passed to system run functions.
    type Inner<'i>;

    /// Converts `self` into a `'static` version of [`SystemInput::Param`].
    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static;

    /// Converts a [`SystemInput::Param`] into a [`SystemInput::Inner`].
    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_>;

    /// Converts a [`SystemInput::Inner`] into a [`SystemInput::Param`].
    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_>;
}

/// Shorthand way to get the [`System::In`] for a [`System`] as a [`SystemInput::Param`].
pub type SystemInParam<'a, S> = <<S as System>::In as SystemInput>::Param<'a>;
/// Shorthand way to get the [`System::In`] for a [`System`] as a [`SystemInput::Inner`].
pub type SystemIn<'a, S> = <<S as System>::In as SystemInput>::Inner<'a>;

impl SystemInput for () {
    type Param<'i> = ();
    type Inner<'i> = ();

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
    }

    fn to_inner(_this: Self::Param<'_>) -> Self::Inner<'_> {}

    fn to_param(_this: Self::Inner<'_>) -> Self::Param<'_> {}
}

impl<T: 'static> SystemInput for In<T> {
    type Param<'i> = In<T>;
    type Inner<'i> = T;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this.0
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        In(this)
    }
}

pub struct InRef<'i, T>(pub &'i T);

impl<T: 'static> SystemInput for InRef<'_, T> {
    type Param<'i> = InRef<'i, T>;
    type Inner<'i> = &'i T;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this.0
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        InRef(this)
    }
}

pub struct InMut<'a, T>(pub &'a mut T);

impl<T: 'static> SystemInput for InMut<'_, T> {
    type Param<'i> = InMut<'i, T>;
    type Inner<'i> = &'i mut T;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this.0
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        InMut(this)
    }
}

impl<E: 'static, B: Bundle> SystemInput for Trigger<'_, E, B> {
    type Param<'i> = Trigger<'i, E, B>;
    type Inner<'i> = Trigger<'i, E, B>;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}
