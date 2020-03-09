// This is required to be copied in because PartialEq is only implemented up to 14 elements on a tuple.
// This implements our own Eq for up to 26 parameters (A-Z)

use std::cmp::*;

pub trait TupleEq<T: ?Sized = Self> {
    fn legion_eq(&self, other: &T) -> bool;
    fn legion_ne(&self, other: &T) -> bool;
}

// macro for implementing n-ary tuple functions and operations
macro_rules! tuple_impls {
    ($(
        $Tuple:ident {
            $(($idx:tt) -> $T:ident)+
        }
    )+) => {
        $(
            impl<$($T:PartialEq),+> TupleEq for ($($T,)+) where last_type!($($T,)+): ?Sized {
                #[inline]
                fn legion_eq(&self, other: &($($T,)+)) -> bool {
                    $(self.$idx == other.$idx)&&+
                }
                #[inline]
                fn legion_ne(&self, other: &($($T,)+)) -> bool {
                    $(self.$idx != other.$idx)||+
                }
            }
        )+
    }
}

macro_rules! last_type {
    ($a:ident,) => { $a };
    ($a:ident, $($rest_a:ident,)+) => { last_type!($($rest_a,)+) };
}

tuple_impls! {
    Tuple1 {
        (0) -> A
    }
    Tuple2 {
        (0) -> A
        (1) -> B
    }
    Tuple3 {
        (0) -> A
        (1) -> B
        (2) -> C
    }
    Tuple4 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
    }
    Tuple5 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
    }
    Tuple6 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
    }
    Tuple7 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
    }
    Tuple8 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
    }
    Tuple9 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
    }
    Tuple10 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
    }
    Tuple11 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
    }
    Tuple12 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
    }
    Tuple13 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
    }
    Tuple14 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
    }
    Tuple15 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
    }
    Tuple16 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
    }
    Tuple17 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
    }
    Tuple18 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
    }
    Tuple19 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
    }
    Tuple20 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
    }
    Tuple21 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
        (20) -> U
    }
    Tuple22 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
        (20) -> U
        (21) -> V
    }
    Tuple23 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
        (20) -> U
        (21) -> V
        (22) -> W
    }
    Tuple24 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
        (20) -> U
        (21) -> V
        (22) -> W
        (23) -> X
    }
    Tuple25 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
        (20) -> U
        (21) -> V
        (22) -> W
        (23) -> X
        (24) -> Y
    }
    Tuple26 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
        (16) -> Q
        (17) -> R
        (18) -> S
        (19) -> T
        (20) -> U
        (21) -> V
        (22) -> W
        (23) -> X
        (24) -> Y
        (25) -> Z
    }
}
