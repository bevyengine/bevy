//! The example demonstrates what type data is, how to create it, and how to use it.

use bevy::{
    prelude::*,
    reflect::{CreateTypeData, TypeRegistry},
};

// It's recommended to read this example from top to bottom.
// Comments are provided to explain the code and its purpose as you go along.
fn main() {
    trait Damageable {
        type Health: Sized + core::ops::Mul<i32, Output = Self::Health>;
        fn damage(&mut self, damage: Self::Health);
    }

    #[derive(Reflect, PartialEq, Debug)]
    struct Zombie {
        health: i32,
    }

    impl Damageable for Zombie {
        type Health = i32;
        fn damage(&mut self, damage: Self::Health) {
            self.health -= damage;
        }
    }

    // Let's say we have a reflected value.
    // Here we know it's a `Zombie`, but for demonstration purposes let's pretend we don't.
    // Pretend it's just some `Box<dyn Reflect>` value.
    let mut value: Box<dyn Reflect> = Box::new(Zombie { health: 100 });

    // We think `value` might contain a type that implements `Damageable`
    // and now we want to call `Damageable::damage` on it.
    // How can we do this without knowing in advance the concrete type is `Zombie`?

    // This is where type data comes in.
    // Type data is a way of associating type-specific data with a type for use in dynamic contexts.
    // This type data can then be used at runtime to perform type-specific operations.

    // Let's create a type data struct for `Damageable` that we can associate with `Zombie`!

    // Firstly, type data must be cloneable.
    #[derive(Clone)]
    // Next, they are usually named with the `Reflect` prefix (we'll see why in a bit).
    struct ReflectDamageable {
        // Type data can contain whatever you want, but it's common to include function pointers
        // to the type-specific operations you want to perform (such as trait methods).
        // Just remember that we're working with `Reflect` data,
        // so we can't use `Self`, generics, or associated types.
        // In those cases, we'll have to use `dyn Reflect` trait objects.
        damage: fn(&mut dyn Reflect, damage: Box<dyn Reflect>, multiplier: i32),
        multiplier: i32,
    }

    // Now, we can create a blanket implementation of the `CreateTypeData` trait to construct our type data
    // for any type that implements `Reflect` and `Damageable`.
    impl<T: Reflect + Damageable<Health: Reflect>> CreateTypeData<T> for ReflectDamageable {
        fn create_type_data(_input: ()) -> Self {
            Self {
                damage: |reflect, damage, multiplier| {
                    // This requires that `reflect` is `T` and not a dynamic representation like `DynamicStruct`.
                    // We could have the function pointer return a `Result`, but we'll just `unwrap` for simplicity.
                    let damageable = reflect.downcast_mut::<T>().unwrap();
                    let damage = damage.take::<T::Health>().unwrap() * multiplier;
                    damageable.damage(damage);
                },
                multiplier: 1,
            }
        }
    }

    // It's also common to provide convenience methods for calling the type-specific operations.
    impl ReflectDamageable {
        pub fn damage(&self, reflect: &mut dyn Reflect, damage: Box<dyn Reflect>) {
            (self.damage)(reflect, damage, self.multiplier);
        }
    }

    // With all this done, we're ready to make use of `ReflectDamageable`!
    // It starts with registering our type along with its type data:
    let mut registry = TypeRegistry::default();
    registry.register::<Zombie>();
    registry.register_type_data::<Zombie, ReflectDamageable>();

    // Then at any point we can retrieve the type data from the registry:
    let type_id = value.reflect_type_info().type_id();
    let reflect_damageable = registry
        .get_type_data::<ReflectDamageable>(type_id)
        .unwrap();

    // And call our method:
    reflect_damageable.damage(value.as_reflect_mut(), Box::new(25i32));
    assert_eq!(value.take::<Zombie>().unwrap(), Zombie { health: 75 });

    // This is a simple example, but type data can be used for much more complex operations.
    // Bevy also provides some useful shorthand for working with type data.

    // For example, we can have the type data be automatically registered when we register the type
    // by using the `#[reflect(MyTrait)]` attribute when defining our type.
    #[derive(Reflect)]
    // Notice that we don't need to type out `ReflectDamageable`.
    // This is why we named it with the `Reflect` prefix:
    // the derive macro will automatically look for a type named `ReflectDamageable` in the current scope.
    #[reflect(Damageable)]
    // We can also specify the path to type data if it isn't currently in-scope:
    // #[reflect(path::to::MyTypeData)]
    struct Skeleton {
        health: i32,
    }

    impl Damageable for Skeleton {
        type Health = i32;
        fn damage(&mut self, damage: Self::Health) {
            self.health -= damage;
        }
    }

    // This will now register `Skeleton` along with its `ReflectDamageable` type data.
    registry.register::<Skeleton>();

    // Additionally, type data can accept arbitrary input.
    // You might have already noticed that the default input is simply `()`,
    // but we can configure this with a new impl with the relevant data.
    //
    // Let's add the ability to define a damage multiplier by accepting an input of type `i32`.
    impl<T: Reflect + Damageable<Health: Reflect>> CreateTypeData<T, i32> for ReflectDamageable {
        fn create_type_data(input: i32) -> Self {
            Self {
                damage: move |reflect, damage, multiplier| {
                    // This requires that `reflect` is `T` and not a dynamic representation like `DynamicStruct`.
                    // We could have the function pointer return a `Result`, but we'll just `unwrap` for simplicity.
                    let damageable = reflect.downcast_mut::<T>().unwrap();
                    let damage = damage.take::<T::Health>().unwrap() * multiplier;
                    damageable.damage(damage);
                },
                multiplier: input,
            }
        }
    }

    #[derive(Reflect)]
    // Now we can pass our `i32` input into our type data:
    #[reflect(Damageable(2))]
    // Note that this accepts any expression. The derive macro will copy it verbatim into the impl.
    // This means you could also write:
    // - #[reflect(Damageable(1 + 1))]
    // - #[reflect(Damageable(4 / 2))]
    // - #[reflect(Damageable({let v = vec![0, 1, 2]; v.into_iter().fold(1, |acc, x| acc * x)}))]
    // - etc.
    struct Beast {
        health: i32,
    }

    impl Damageable for Beast {
        type Health = i32;
        fn damage(&mut self, damage: Self::Health) {
            self.health -= damage;
        }
    }

    // Now when we register `Beast` it will automatically register with a `ReflectDamageable` with a multiplier of `2`.
    registry.register::<Beast>();

    let data = registry
        .get_type_data::<ReflectDamageable>(core::any::TypeId::of::<Beast>())
        .unwrap();
    assert_eq!(data.multiplier, 2);

    // We can also choose to define the input when manually registering type data.
    registry.register_type_data_with::<Beast, ReflectDamageable, _>(5);

    // And for object-safe traits (see https://doc.rust-lang.org/reference/items/traits.html#object-safety),
    // Bevy provides a convenience macro for generating type data that converts `dyn Reflect` into `dyn MyTrait`.
    #[reflect_trait]
    trait Health {
        fn health(&self) -> i32;
    }

    impl Health for Skeleton {
        fn health(&self) -> i32 {
            self.health
        }
    }

    // Using the `#[reflect_trait]` macro we're able to automatically generate a `ReflectHealth` type data struct,
    // which can then be registered like any other type data:
    registry.register_type_data::<Skeleton, ReflectHealth>();

    // Now we can use `ReflectHealth` to convert `dyn Reflect` into `dyn Health`:
    let value: Box<dyn Reflect> = Box::new(Skeleton { health: 50 });

    let type_id = value.reflect_type_info().type_id();
    let reflect_health = registry.get_type_data::<ReflectHealth>(type_id).unwrap();

    // Type data generated by `#[reflect_trait]` comes with a `get`, `get_mut`, and `get_boxed` method,
    // which convert `&dyn Reflect` into `&dyn MyTrait`, `&mut dyn Reflect` into `&mut dyn MyTrait`,
    // and `Box<dyn Reflect>` into `Box<dyn MyTrait>`, respectively.
    let value: &dyn Health = reflect_health.get(value.as_reflect()).unwrap();
    assert_eq!(value.health(), 50);

    // Lastly, here's a list of some useful type data provided by Bevy that you might want to register for your types:
    // - `ReflectDefault` for types that implement `Default`
    // - `ReflectFromWorld` for types that implement `FromWorld`
    // - `ReflectComponent` for types that implement `Component`
    // - `ReflectBundle` for types that implement `Bundle`
    // - `ReflectResource` for types that implement `Resource`
    // - `ReflectAsset` for types that implement `Asset`
    // - `ReflectEvent` for types that implement `Event`
    // - `ReflectMessage` for types that implement `Message`
    // - `ReflectSerialize` for types that implement `Serialize`
    // - `ReflectDeserialize` for types that implement `Deserialize`
    // - And more!
    //
    // There are also some that are automatically registered by the `Reflect` derive macro:
    // - `ReflectFromPtr`
    // - `ReflectFromReflect` (if not `#[reflect(from_reflect = false)]`)
}
