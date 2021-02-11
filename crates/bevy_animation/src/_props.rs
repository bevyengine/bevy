// TODO: Implement safe property path like `Transforms::props().translation().x()` using only const, (Property validation is working at least)

/// A type safe way of referring to properties that can be animated.
///
/// If a type implements `AnimatedProperties` each property can be retrieved
/// by `Transform::props().translation().x()`
pub struct Prop<T> {
    pub name: Cow<'static, str>,
    _marker: PhantomData<T>,
}

impl<T> Prop<T> {
    pub const fn borrowed(name: &'static str) -> Self {
        Prop {
            name: Cow::Borrowed(name),
            _marker: PhantomData,
        }
    }

    pub const fn owned(name: String) -> Self {
        Prop {
            name: Cow::Owned(name),
            _marker: PhantomData,
        }
    }
}

// ? NOTE: That upsets me, it should be auto generated path
impl Prop<Vec3> {
    pub fn x(&self) -> Prop<f32> {
        let mut name = self.name.as_ref().to_owned();
        name.push_str(".x");
        Prop {
            name: Cow::Owned(name),
            _marker: PhantomData,
        }
    }

    pub fn y(&self) -> Prop<f32> {
        let mut name = self.name.as_ref().to_owned();
        name.push_str(".y");
        Prop {
            name: Cow::Owned(name),
            _marker: PhantomData,
        }
    }

    pub fn z(&self) -> Prop<f32> {
        let mut name = self.name.as_ref().to_owned();
        name.push_str(".z");
        Prop {
            name: Cow::Owned(name),
            _marker: PhantomData,
        }
    }
}
