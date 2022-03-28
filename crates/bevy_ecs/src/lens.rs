use std::marker::PhantomData;

pub trait Lens {
    type In: 'static;
    type Out: 'static;

    fn get(input: &Self::In) -> &Self::Out;
    fn get_mut(input: &mut Self::In) -> &mut Self::Out;
}

pub struct ComposedLens<A, B>(PhantomData<(A, B)>);

impl<A, B> Lens for ComposedLens<A, B>
where
    A: Lens,
    B: Lens<In = A::Out>,
{
    type In = A::In;
    type Out = B::Out;

    fn get(input: &Self::In) -> &Self::Out {
        B::get(A::get(input))
    }

    fn get_mut(input: &mut Self::In) -> &mut Self::Out {
        B::get_mut(A::get_mut(input))
    }
}

pub struct NoopLens<T>(PhantomData<T>);

impl<T: 'static> Lens for NoopLens<T> {
    type In = T;
    type Out = T;

    fn get(input: &Self::In) -> &Self::Out {
        input
    }

    fn get_mut(input: &mut Self::In) -> &mut Self::Out {
        input
    }
}

#[macro_export]
macro_rules! declare_lens {
    ($name:ident, $in:ty, $out:ty, $($path:tt).*) => {
        struct $name;

        impl $crate::lens::Lens for $name {
            type In = $in;
            type Out = $out;

            fn get(input: &Self::In) -> &Self::Out {
                &input$(.$path)*
            }

            fn get_mut(input: &mut Self::In) -> &mut Self::Out {
                &mut input$(.$path)*
            }
        }
    };
}

#[macro_export]
macro_rules! composed_lens {
    ($last:ty) => {
        $last
    };
    ($first:ty, $second:ty) => {
        $crate::lens::ComposedLens<$first, $second>
    };
    ($first:ty, $second:ty, $($rest:ty),*) => {
        $crate::lens::ComposedLens<composed_lens![$first, $second], composed_lens![$($rest),*]>
    };
}
