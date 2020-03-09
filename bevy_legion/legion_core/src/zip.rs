// This is copied in from itertools because they only implement zip for up to 8 elements.
// This implements our own zip up to 26 elements.

// Copyright (c) 2015
// https://github.com/rust-itertools/itertools
//
//Permission is hereby granted, free of charge, to any
//person obtaining a copy of this software and associated
//documentation files (the "Software"), to deal in the
//Software without restriction, including without
//limitation the rights to use, copy, modify, merge,
//publish, distribute, sublicense, and/or sell copies of
//the Software, and to permit persons to whom the Software
//is furnished to do so, subject to the following
//conditions:
//
//The above copyright notice and this permission notice
//shall be included in all copies or substantial portions
//of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
//ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
//TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
//PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
//SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
//OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
//IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//DEALINGS IN THE SOFTWARE.

#[inline]
pub fn min(a: (usize, Option<usize>), b: (usize, Option<usize>)) -> (usize, Option<usize>) {
    let (a_lower, a_upper) = a;
    let (b_lower, b_upper) = b;
    let lower = std::cmp::min(a_lower, b_lower);
    let upper = match (a_upper, b_upper) {
        (Some(u1), Some(u2)) => Some(std::cmp::min(u1, u2)),
        _ => a_upper.or(b_upper),
    };
    (lower, upper)
}

#[derive(Clone, Debug)]
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Zip<T> {
    t: T,
}

pub fn multizip<T, U>(t: U) -> Zip<T>
where
    Zip<T>: From<U>,
    Zip<T>: Iterator,
{
    Zip::from(t)
}

macro_rules! impl_zip_iter {
    ($($B:ident),*) => (
        #[allow(non_snake_case)]
        impl<$($B: IntoIterator),*> From<($($B,)*)> for Zip<($($B::IntoIter,)*)> {
            fn from(t: ($($B,)*)) -> Self {
                let ($($B,)*) = t;
                Zip { t: ($($B.into_iter(),)*) }
            }
        }

        #[allow(non_snake_case)]
        #[allow(unused_assignments)]
        impl<$($B),*> Iterator for Zip<($($B,)*)>
            where
            $(
                $B: Iterator,
            )*
        {
            type Item = ($($B::Item,)*);

            fn next(&mut self) -> Option<Self::Item>
            {
                let ($(ref mut $B,)*) = self.t;

                // NOTE: Just like iter::Zip, we check the iterators
                // for None in order. We may finish unevenly (some
                // iterators gave n + 1 elements, some only n).
                $(
                    let $B = match $B.next() {
                        None => return None,
                        Some(elt) => elt
                    };
                )*
                Some(($($B,)*))
            }

            #[allow(clippy::let_and_return)]
            fn size_hint(&self) -> (usize, Option<usize>)
            {
                let sh = (::std::usize::MAX, None);
                let ($(ref $B,)*) = self.t;
                $(
                    let sh = min($B.size_hint(), sh);
                )*
                sh
            }
        }

        #[allow(non_snake_case)]
        impl<$($B),*> ExactSizeIterator for Zip<($($B,)*)> where
            $(
                $B: ExactSizeIterator,
            )*
        { }
    );
}

impl_zip_iter!(A);
impl_zip_iter!(A, B);
impl_zip_iter!(A, B, C);
impl_zip_iter!(A, B, C, D);
impl_zip_iter!(A, B, C, D, E);
impl_zip_iter!(A, B, C, D, E, F);
impl_zip_iter!(A, B, C, D, E, F, G);
impl_zip_iter!(A, B, C, D, E, F, G, H);
impl_zip_iter!(A, B, C, D, E, F, G, H, I);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
impl_zip_iter!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
