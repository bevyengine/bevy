pub trait IntoSystemConfigs: Sized {
    fn after<T>(self, _other: T) -> Self {
        self
    }
}

impl<T> IntoSystemConfigs for T {}
