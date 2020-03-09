// Things happen here, and they work.
//                       ,---.
//                       /    |
//                      /     |
//                     /      |
//                    /       |
//               ___,'        |
//             <  -'          :
//              `-.__..--'``-,_\_
//                 |o/ ` :,.)_`>
//                 :/ `     ||/)
//                 (_.).__,-` |\
//                 /( `.``   `| :
//                 \'`-.)  `  ; ;
//                 | `       /-<
//                 |     `  /   `.
// ,-_-..____     /|  `    :__..-'\
// ,'-.__\\  ``-./ :`      ;       \
//`\ `\  `\\  \ :  (   `  /  ,   `. \
//  \` \   \\   |  | `   :  :     .\ \
//   \ `\_  ))  :  ;     |  |      ): :
//  (`-.-'\ ||  |\ \   ` ;  ;       | |
//   \-_   `;;._   ( `  /  /_       | |
//    `-.-.// ,'`-._\__/_,'         ; |
//       \:: :     /     `     ,   /  |
//        || |    (        ,' /   /   |
//        ||                ,'   /    |

/// Prepend a new type into a cons list
pub trait ConsPrepend<T> {
    /// Result of prepend
    type Output;
    /// Prepend to runtime cons value
    fn prepend(self, t: T) -> Self::Output;
}

impl<T> ConsPrepend<T> for () {
    type Output = (T, Self);
    fn prepend(self, t: T) -> Self::Output { (t, self) }
}

impl<T, A, B> ConsPrepend<T> for (A, B) {
    type Output = (T, Self);
    fn prepend(self, t: T) -> Self::Output { (t, self) }
}

/// Prepend a new type into a cons list
pub trait ConsAppend<T> {
    /// Result of append
    type Output;
    /// Prepend to runtime cons value
    fn append(self, t: T) -> Self::Output;
}

impl<T> ConsAppend<T> for () {
    type Output = (T, Self);
    fn append(self, t: T) -> Self::Output { (t, ()) }
}

impl<T, A, B: ConsAppend<T>> ConsAppend<T> for (A, B) {
    type Output = (A, <B as ConsAppend<T>>::Output);
    fn append(self, t: T) -> Self::Output {
        let (a, b) = self;
        (a, b.append(t))
    }
}

/// transform cons list into a flat tuple
pub trait ConsFlatten {
    /// Flattened tuple
    type Output;
    /// Flatten runtime cons value
    fn flatten(self) -> Self::Output;
}

impl ConsFlatten for () {
    type Output = ();
    fn flatten(self) -> Self::Output { self }
}

macro_rules! cons {
    () => (
        ()
    );
    ($head:tt) => (
        ($head, ())
    );
    ($head:tt, $($tail:tt),*) => (
        ($head, cons!($($tail),*))
    );
}

macro_rules! impl_flatten {
    ($($items:ident),*) => {
    #[allow(unused_parens)] // This is added because the nightly compiler complains
        impl<$($items),*> ConsFlatten for cons!($($items),*)
        {
            type Output = ($($items),*);
            fn flatten(self) -> Self::Output {
                #[allow(non_snake_case)]
                let cons!($($items),*) = self;
                ($($items),*)
            }
        }

        impl_flatten!(@ $($items),*);
    };
    (@ $head:ident, $($tail:ident),*) => {
        impl_flatten!($($tail),*);
    };
    (@ $head:ident) => {};
}

impl_flatten!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

fn test_api() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cons_macro() {
        #![allow(clippy::unit_cmp)]
        assert_eq!(cons!(), ());
        assert_eq!(cons!(1), (1, ()));
        assert_eq!(cons!(1, 2, 3, 4), (1, (2, (3, (4, ())))));
    }

    #[test]
    fn cons_prepend() {
        assert_eq!(().prepend(123), (123, ()));
        assert_eq!(
            cons!(1, 2, 3, 4, 5).prepend(123).prepend(15),
            cons!(15, 123, 1, 2, 3, 4, 5)
        );
    }

    #[test]
    fn cons_append() {
        assert_eq!(().append(123), (123, ()));
        assert_eq!(
            cons!(1, 2, 3, 4, 5).append(123).append(15),
            cons!(1, 2, 3, 4, 5, 123, 15)
        );
    }

    #[test]
    fn cons_flatten() {
        #![allow(clippy::unit_cmp)]
        assert_eq!(().flatten(), ());
        assert_eq!((1, ()).flatten(), 1);
        assert_eq!(cons!(1, 2, 3, 4, 5).flatten(), (1, 2, 3, 4, 5));
    }
}
