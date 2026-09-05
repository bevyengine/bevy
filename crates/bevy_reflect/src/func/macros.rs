/// Helper macro to implement the necessary traits for function reflection.
///
/// This macro calls the following macros:
/// - [`impl_get_ownership`](crate::func::args::impl_get_ownership)
/// - [`impl_from_arg`](crate::func::args::impl_from_arg)
/// - [`impl_into_return`](crate::func::impl_into_return)
///
/// # Syntax
///
/// For non-generic types, the macro simply expects the type:
///
/// ```ignore
/// impl_function_traits!(foo::bar::Baz);
/// ```
///
/// For generic types, however, the generic type parameters must also be given in angle brackets (`<` and `>`):
///
/// ```ignore
/// impl_function_traits!(foo::bar::Baz<T, U>; <T: Clone, U>);
/// ```
///
/// For generic const parameters, they must be given in square brackets (`[` and `]`):
///
/// ```ignore
/// impl_function_traits!(foo::bar::Baz<T, N>; <T> [const N: usize]);
/// ```
macro_rules! impl_function_traits {
    (
        $ty: ty
        $(;
            < $($T: ident $(: $T1: tt $(+ $T2: tt)*)?),* >
        )?
        $(
            [ $(const $N: ident : $size: ident),* ]
        )?
        $(
            where $($U: ty $(: $U1: tt $(+ $U2: tt)*)?),*
        )?
    ) => {
        $crate::func::args::impl_get_ownership!(
            $ty
            $(;
                < $($T $(: $T1 $(+ $T2)*)?),* >
            )?
            $(
                [ $(const $N : $size),* ]
            )?
            $(
                where $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
        $crate::func::args::impl_from_arg!(
            $ty
            $(;
                < $($T $(: $T1 $(+ $T2)*)?),* >
            )?
            $(
                [ $(const $N : $size),* ]
            )?
            $(
                where $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
        $crate::func::impl_into_return!(
            $ty
            $(;
                < $($T $(: $T1 $(+ $T2)*)?),* >
            )?
            $(
                [ $(const $N : $size),* ]
            )?
            $(
                where $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
    };
}

pub(crate) use impl_function_traits;

/// Helper macro that returns the number of tokens it receives.
///
/// See [here] for details.
///
/// [here]: https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html#bit-twiddling
macro_rules! count_tokens {
    () => { 0 };
    ($odd:tt $($a:tt $b:tt)*) => { ($crate::func::macros::count_tokens!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { $crate::func::macros::count_tokens!($($a)*) << 1 };
}

pub(crate) use count_tokens;

/// A helper macro for generating instances of [`DynamicFunction`] and [`DynamicFunctionMut`].
///
/// There are some functions that cannot be automatically converted to a dynamic function
/// via [`IntoFunction`] or [`IntoFunctionMut`].
/// This normally includes functions with more than 15 arguments and functions that
/// return a reference with a lifetime not tied to the first argument.
///
/// For example, the following fails to compile:
///
/// ```compile_fail
/// # use bevy_reflect::func::IntoFunction;
/// fn insert_at(index: usize, value: i32, list: &mut Vec<i32>) -> &i32 {
///     list.insert(index, value);
///     &list[index]
/// }
///
/// // This will fail to compile since `IntoFunction` expects return values
/// // to have a lifetime tied to the first argument, but this function
/// // returns a reference tied to the third argument.
/// let func = insert_at.into_function();
/// ```
///
/// In these cases, we normally need to generate the [`DynamicFunction`] manually:
///
/// ```
/// # use bevy_reflect::func::{DynamicFunction, IntoFunction, IntoReturn, SignatureInfo};
/// # fn insert_at(index: usize, value: i32, list: &mut Vec<i32>) -> &i32 {
/// #     list.insert(index, value);
/// #     &list[index]
/// # }
/// let func = DynamicFunction::new(
///     |mut args| {
///         let index = args.take::<usize>()?;
///         let value = args.take::<i32>()?;
///         let list = args.take::<&mut Vec<i32>>()?;
///
///         let result = insert_at(index, value, list);
///
///         Ok(result.into_return())
///     },
///     SignatureInfo::named("insert_at")
///         .with_arg::<usize>("index")
///         .with_arg::<i32>("value")
///         .with_arg::<&mut Vec<i32>>("list")
///         .with_return::<&i32>(),
/// );
/// ```
///
/// However, this is both verbose and error-prone.
/// What happens if we forget to add an argument to the [`SignatureInfo`]?
/// What if we forget to set the return type?
///
/// This macro can be used to generate the above code safely, automatically,
/// and with less boilerplate:
///
/// ```
/// # use bevy_reflect::func::ArgList;
/// # use bevy_reflect::reflect_fn;
/// # fn insert_at(index: usize, value: i32, list: &mut Vec<i32>) -> &i32 {
/// #     list.insert(index, value);
/// #     &list[index]
/// # }
/// let func = reflect_fn!(
///     fn insert_at(index: usize, value: i32, list: &mut Vec<i32>) -> &i32 {
///         insert_at(index, value, list)
///     }
/// );
/// # // Sanity tests:
/// # let info = func.info();
/// # assert_eq!(info.name().unwrap(), "insert_at");
/// # assert_eq!(info.base().arg_count(), 3);
/// # assert_eq!(info.base().args()[0].name(), Some("index"));
/// # assert!(info.base().args()[0].is::<usize>());
/// # assert_eq!(info.base().args()[1].name(), Some("value"));
/// # assert!(info.base().args()[1].is::<i32>());
/// # assert_eq!(info.base().args()[2].name(), Some("list"));
/// # assert!(info.base().args()[2].is::<&mut Vec<i32>>());
/// # assert!(info.base().return_info().is::<&i32>());
/// #
/// # let mut list = vec![1, 2, 3];
/// # let args = ArgList::new().push_owned(0_usize).push_owned(5_i32).push_mut(&mut list);
/// # let result = func.call(args).unwrap().unwrap_ref();
/// # assert_eq!(result.try_downcast_ref::<i32>(), Some(&5));
/// # assert_eq!(list, vec![5, 1, 2, 3]);
/// ```
///
/// # Syntax
///
/// The macro expects the following syntax:
///
/// ```text
/// MUT MOVE fn NAME ( ARGS ) RETURN BLOCK
/// ```
///
/// - `MUT`: `mut` | _none_
///   - If present, the generated function will instead be a [`DynamicFunctionMut`].
/// - `MOVE`: `move` | _none_
///   - If present, adds the `move` keyword to the internal closure.
/// - `NAME`: Block | Identifier | `[` Expression Path `]` | Literal | _none_
///    - If present, defines the name of the function for the [`SignatureInfo`].
///    - Blocks should evaluate to a string, Identifiers will be [stringified],
///      Expression Paths will be evaluated with [`core::any::type_name_of_val`],
///      and Literals will be used as-is.
/// - `ARGS`: ( `mut`? Identifier `:` Type `,`? )*
///   - The list of 0 or more arguments the function accepts.
/// - `RETURN`: `->` Type | _none_
///   - If present, defines the return type of the function.
///     Otherwise, the return type is assumed to be `()`.
/// - `BLOCK`: Block | _none_
///   - Optional if `NAME` is an `Expression Path` of a function in scope.
///     In such cases, a Block will be generated that calls `NAME` with `ARGS`.
///   - Otherwise, if present, defines the block of code that the function will execute.
///
/// # Examples
///
/// Using a function already in scope:
///
/// ```
/// # use bevy_reflect::func::ArgList;
/// # use bevy_reflect::reflect_fn;
/// mod math {
///     use std::ops::Add;
///
///     pub fn add<T: Add<Output=T>>(a: T, b: T) -> T {
///         a + b
///     }
/// }
///
/// let func = reflect_fn!(fn [math::add::<i32>](a: i32, b: i32) -> i32);
///
/// let info = func.info();
/// assert!(info.name().unwrap().ends_with("math::add<i32>"));
///
/// let args = ArgList::new().push_owned(1).push_owned(2);
/// let result = func.call(args).unwrap().unwrap_owned();
/// assert_eq!(result.try_downcast_ref(), Some(&3));
/// ```
///
/// Defining anonymous functions:
///
/// ```
/// # use bevy_reflect::reflect_fn;
/// let func = reflect_fn!(
///     fn(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// );
///
/// let info = func.info();
/// assert_eq!(info.name(), None);
/// ```
///
/// Defining functions with computed names:
///
/// ```
/// # use bevy_reflect::reflect_fn;
/// let func = reflect_fn!(
///     fn {concat!("a", "d", "d")} (a: i32, b: i32) -> i32 {
///         a + b
///     }
/// );
///
/// let info = func.info();
/// assert_eq!(info.name().unwrap(), "add");
/// ```
///
/// Defining functions with literal names:
///
/// ```
/// # use bevy_reflect::reflect_fn;
/// let func = reflect_fn!(
///     fn "add two numbers" (a: i32, b: i32) -> i32 {
///         a + b
///     }
/// );
///
/// let info = func.info();
/// assert_eq!(info.name().unwrap(), "add two numbers");
/// ```
///
/// Generating a [`DynamicFunctionMut`]:
///
/// ```
/// # use bevy_reflect::func::ArgList;
/// # use bevy_reflect::reflect_fn;
/// let mut list = Vec::<i32>::new();
/// let func = reflect_fn!(
///     mut fn push(value: i32) {
///         list.push(value);
///     }
/// );
///
/// let args = ArgList::new().push_owned(123);
/// func.call_once(args).unwrap();
/// assert_eq!(list, vec![123]);
/// ```
///
/// Capturing variables with `move`:
///
/// ```
/// # use bevy_reflect::reflect_fn;
/// # use std::sync::Arc;
/// let name = Arc::new(String::from("World"));
///
/// let name_clone = Arc::clone(&name);
/// let func = reflect_fn!(
///     move fn print() {
///         println!("Hello, {}", name_clone);
///     }
/// );
///
/// assert_eq!(Arc::strong_count(&name), 2);
/// drop(func);
/// assert_eq!(Arc::strong_count(&name), 1);
/// ```
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
/// [`IntoFunction`]: crate::func::IntoFunction
/// [`IntoFunctionMut`]: crate::func::IntoFunctionMut
/// [`SignatureInfo`]: crate::func::SignatureInfo
/// [stringified]: core::stringify
#[macro_export]
macro_rules! reflect_fn {
    // === Main === //
    (@main [$($mut_:tt)?] [$($move_:tt)?] fn $($name:block)? ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@func $($mut_)?)(
            #[allow(unused_variables, unused_mut)]
            $($move_)? |mut args| {
                $(let $arg_name = args.take::<$arg_ty>()?;)*
                let result = $crate::reflect_fn!(@eval $block $(as $ret_ty)?);
                ::core::result::Result::Ok($crate::func::IntoReturn::into_return(result))
            },
            $crate::reflect_fn!(@info $($name)?)
                $(.with_arg::<$arg_ty>(::core::stringify!($arg_name)))*
                $(.with_return::<$ret_ty>())?
        )
    };

    // === Helpers === //
    (@func mut) => {
        $crate::func::DynamicFunctionMut::new
    };
    (@func) => {
        $crate::func::DynamicFunction::new
    };
    (@info $name:block) => {
        $crate::func::SignatureInfo::named($name)
    };
    (@info) => {
        $crate::func::SignatureInfo::anonymous()
    };
    (@eval $block:block as $ty:ty) => {
        // We don't actually use `$ty` here since it can lead to `Missing lifetime specifier` errors.
        $block
    };
    (@eval $block:block) => {{
        // Ensures that `$block` actually evaluates to `()` and isn't relying on type inference,
        // which would end up not being reflected by `SignatureInfo` properly.
        let temp: () = $block;
        temp
    }};

    // === Anonymous === //
    (fn ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [] fn ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut fn ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [] fn ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (move fn ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [move] fn ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut move fn ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [move] fn ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };

    // === Block Named === //
    (fn $name:block ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [] fn $name ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut fn $name:block ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [] fn $name ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (move fn $name:block ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [move] fn $name ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut move fn $name:block ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [move] fn $name ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };

    // === Ident Named === //
    (fn $name:ident ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [] fn {::core::stringify!($name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut fn $name:ident ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [] fn {::core::stringify!($name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (move fn $name:ident ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [move] fn {::core::stringify!($name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut move fn $name:ident ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [move] fn {::core::stringify!($name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };

    // === Literal Named === //
    (fn $name:literal ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [] fn {$name} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut fn $name:literal ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [] fn {$name} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (move fn $name:literal ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [] [move] fn {$name} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };
    (mut move fn $name:literal ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $block:block) => {
        $crate::reflect_fn!(@main [mut] [move] fn {$name} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? $block)
    };

    // === In Scope === //
    (fn [$name:expr] ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $($block:block)?) => {
        $crate::reflect_fn!(@main [] [] fn {::core::any::type_name_of_val(&$name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? { $name($($arg_name),*) })
    };
    (mut fn [$name:expr] ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $($block:block)?) => {
        $crate::reflect_fn!(@main [mut] [] fn {::core::any::type_name_of_val(&$name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? { $name($($arg_name),*) })
    };
    (move fn [$name:expr] ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $($block:block)?) => {
        $crate::reflect_fn!(@main [] [move] fn {::core::any::type_name_of_val(&$name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? { $name($($arg_name),*) })
    };
    (mut move fn [$name:expr] ($($(mut)? $arg_name:ident : $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $($block:block)?) => {
        $crate::reflect_fn!(@main [mut] [move] fn {::core::any::type_name_of_val(&$name)} ($($arg_name : $arg_ty),*) $(-> $ret_ty)? { $name($($arg_name),*) })
    };
}
