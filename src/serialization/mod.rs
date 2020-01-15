mod world;

pub use world::*;

pub fn type_name_of_val<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}
