//! This example demonstrates how functions can be called dynamically using reflection.
//!
//! Function reflection is useful for calling regular Rust functions in a dynamic context,
//! where the types of arguments, return values, and even the function itself aren't known at compile time.
//!
//! This can be used for things like adding scripting support to your application,
//! processing deserialized reflection data, or even just storing type-erased versions of your functions.

use bevy::reflect::{
    func::{
        ArgList, DynamicFunction, DynamicFunctionMut, FunctionResult, IntoFunction,
        IntoFunctionMut, Return, SignatureInfo,
    },
    PartialReflect, Reflect,
};

// Note that the `dbg!` invocations are used purely for demonstration purposes
// and are not strictly necessary for the example to work.
fn main() {
    // There are times when it may be helpful to store a function away for later.
    // In Rust, we can do this by storing either a function pointer or a function trait object.
    // For example, say we wanted to store the following function:
    fn add(left: i32, right: i32) -> i32 {
        left + right
    }

    // We could store it as either of the following:
    let fn_pointer: fn(i32, i32) -> i32 = add;
    let fn_trait_object: Box<dyn Fn(i32, i32) -> i32> = Box::new(add);

    // And we can call them like so:
    let result = fn_pointer(2, 2);
    assert_eq!(result, 4);
    let result = fn_trait_object(2, 2);
    assert_eq!(result, 4);

    // However, you'll notice that we have to know the types of the arguments and return value at compile time.
    // This means there's not really a way to store or call these functions dynamically at runtime.
    // Luckily, Bevy's reflection crate comes with a set of tools for doing just that!
    // We do this by first converting our function into the reflection-based `DynamicFunction` type
    // using the `IntoFunction` trait.
    let function: DynamicFunction<'static> = dbg!(add.into_function());

    // This time, you'll notice that `DynamicFunction` doesn't take any information about the function's arguments or return value.
    // This is because `DynamicFunction` checks the types of the arguments and return value at runtime.
    // Now we can generate a list of arguments:
    let args: ArgList = dbg!(ArgList::new().with_owned(2_i32).with_owned(2_i32));

    // And finally, we can call the function.
    // This returns a `Result` indicating whether the function was called successfully.
    // For now, we'll just unwrap it to get our `Return` value,
    // which is an enum containing the function's return value.
    let return_value: Return = dbg!(function.call(args).unwrap());

    // The `Return` value can be pattern matched or unwrapped to get the underlying reflection data.
    // For the sake of brevity, we'll just unwrap it here and downcast it to the expected type of `i32`.
    let value: Box<dyn PartialReflect> = return_value.unwrap_owned();
    assert_eq!(value.try_take::<i32>().unwrap(), 4);

    // The same can also be done for closures that capture references to their environment.
    // Closures that capture their environment immutably can be converted into a `DynamicFunction`
    // using the `IntoFunction` trait.
    let minimum = 5;
    let clamp = |value: i32| value.max(minimum);

    let function: DynamicFunction = dbg!(clamp.into_function());
    let args = dbg!(ArgList::new().with_owned(2_i32));
    let return_value = dbg!(function.call(args).unwrap());
    let value: Box<dyn PartialReflect> = return_value.unwrap_owned();
    assert_eq!(value.try_take::<i32>().unwrap(), 5);

    // We can also handle closures that capture their environment mutably
    // using the `IntoFunctionMut` trait.
    let mut count = 0;
    let increment = |amount: i32| count += amount;

    let closure: DynamicFunctionMut = dbg!(increment.into_function_mut());
    let args = dbg!(ArgList::new().with_owned(5_i32));

    // Because `DynamicFunctionMut` mutably borrows `total`,
    // it will need to be dropped before `total` can be accessed again.
    // This can be done manually with `drop(closure)` or by using the `DynamicFunctionMut::call_once` method.
    dbg!(closure.call_once(args).unwrap());
    assert_eq!(count, 5);

    // Generic functions can also be converted into a `DynamicFunction`,
    // however, they will need to be manually monomorphized first.
    fn stringify<T: ToString>(value: T) -> String {
        value.to_string()
    }

    // We have to manually specify the concrete generic type we want to use.
    let function = stringify::<i32>.into_function();

    let args = ArgList::new().with_owned(123_i32);
    let return_value = function.call(args).unwrap();
    let value: Box<dyn PartialReflect> = return_value.unwrap_owned();
    assert_eq!(value.try_take::<String>().unwrap(), "123");

    // To make things a little easier, we can also "overload" functions.
    // This makes it so that a single `DynamicFunction` can represent multiple functions,
    // and the correct one is chosen based on the types of the arguments.
    // Each function overload must have a unique argument signature.
    let function = stringify::<i32>
        .into_function()
        .with_overload(stringify::<f32>);

    // Now our `function` accepts both `i32` and `f32` arguments.
    let args = ArgList::new().with_owned(1.23_f32);
    let return_value = function.call(args).unwrap();
    let value: Box<dyn PartialReflect> = return_value.unwrap_owned();
    assert_eq!(value.try_take::<String>().unwrap(), "1.23");

    // Function overloading even allows us to have a variable number of arguments.
    let function = (|| 0)
        .into_function()
        .with_overload(|a: i32| a)
        .with_overload(|a: i32, b: i32| a + b)
        .with_overload(|a: i32, b: i32, c: i32| a + b + c);

    let args = ArgList::new()
        .with_owned(1_i32)
        .with_owned(2_i32)
        .with_owned(3_i32);
    let return_value = function.call(args).unwrap();
    let value: Box<dyn PartialReflect> = return_value.unwrap_owned();
    assert_eq!(value.try_take::<i32>().unwrap(), 6);

    // As stated earlier, `IntoFunction` works for many kinds of simple functions.
    // Functions with non-reflectable arguments or return values may not be able to be converted.
    // Generic functions are also not supported (unless manually monomorphized like `foo::<i32>.into_function()`).
    // Additionally, the lifetime of the return value is tied to the lifetime of the first argument.
    // However, this means that many methods (i.e. functions with a `self` parameter) are also supported:
    #[derive(Reflect, Default)]
    struct Data {
        value: String,
    }

    impl Data {
        fn set_value(&mut self, value: String) {
            self.value = value;
        }

        // Note that only `&'static str` implements `Reflect`.
        // To get around this limitation we can use `&String` instead.
        fn get_value(&self) -> &String {
            &self.value
        }
    }

    let mut data = Data::default();

    let set_value = dbg!(Data::set_value.into_function());
    let args = dbg!(ArgList::new().with_mut(&mut data)).with_owned(String::from("Hello, world!"));
    dbg!(set_value.call(args).unwrap());
    assert_eq!(data.value, "Hello, world!");

    let get_value = dbg!(Data::get_value.into_function());
    let args = dbg!(ArgList::new().with_ref(&data));
    let return_value = dbg!(get_value.call(args).unwrap());
    let value: &dyn PartialReflect = return_value.unwrap_ref();
    assert_eq!(value.try_downcast_ref::<String>().unwrap(), "Hello, world!");

    // For more complex use cases, you can always create a custom `DynamicFunction` manually.
    // This is useful for functions that can't be converted via the `IntoFunction` trait.
    // For example, this function doesn't implement `IntoFunction` due to the fact that
    // the lifetime of the return value is not tied to the lifetime of the first argument.
    fn get_or_insert(value: i32, container: &mut Option<i32>) -> &i32 {
        if container.is_none() {
            *container = Some(value);
        }

        container.as_ref().unwrap()
    }

    let get_or_insert_function = dbg!(DynamicFunction::new(
        |mut args: ArgList| -> FunctionResult {
            // The `ArgList` contains the arguments in the order they were pushed.
            // The `DynamicFunction` will validate that the list contains
            // exactly the number of arguments we expect.
            // We can retrieve them out in order (note that this modifies the `ArgList`):
            let value = args.take::<i32>()?;
            let container = args.take::<&mut Option<i32>>()?;

            // We could have also done the following to make use of type inference:
            // let value = args.take_owned()?;
            // let container = args.take_mut()?;

            Ok(Return::Ref(get_or_insert(value, container)))
        },
        // Functions can be either anonymous or named.
        // It's good practice, though, to try and name your functions whenever possible.
        // This makes it easier to debug and is also required for function registration.
        // We can either give it a custom name or use the function's type name as
        // derived from `std::any::type_name_of_val`.
        SignatureInfo::named(std::any::type_name_of_val(&get_or_insert))
            // We can always change the name if needed.
            // It's a good idea to also ensure that the name is unique,
            // such as by using its type name or by prefixing it with your crate name.
            .with_name("my_crate::get_or_insert")
            // Since our function takes arguments, we should provide that argument information.
            // This is used to validate arguments when calling the function.
            // And it aids consumers of the function with their own validation and debugging.
            // Arguments should be provided in the order they are defined in the function.
            .with_arg::<i32>("value")
            .with_arg::<&mut Option<i32>>("container")
            // We can provide return information as well.
            .with_return::<&i32>(),
    ));

    let mut container: Option<i32> = None;

    let args = dbg!(ArgList::new().with_owned(5_i32).with_mut(&mut container));
    let value = dbg!(get_or_insert_function.call(args).unwrap()).unwrap_ref();
    assert_eq!(value.try_downcast_ref::<i32>(), Some(&5));

    let args = dbg!(ArgList::new().with_owned(500_i32).with_mut(&mut container));
    let value = dbg!(get_or_insert_function.call(args).unwrap()).unwrap_ref();
    assert_eq!(value.try_downcast_ref::<i32>(), Some(&5));
}
