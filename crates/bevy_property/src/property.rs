use crate::Properties;
use std::any::Any;
use erased_serde::Serialize;


pub enum Serializable<'a> {
    Owned(Box<dyn Serialize + 'a>),
    Borrowed(&'a dyn Serialize),
}

impl<'a> Serializable<'a> {
    pub fn borrow(&self) -> &dyn Serialize {
        match self {
            Serializable::Borrowed(serialize) => serialize,
            Serializable::Owned(serialize) => serialize,
        }
    }
}

pub trait Property: Send + Sync + Any + 'static {
    fn type_name(&self) -> &str;
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn clone_prop(&self) -> Box<dyn Property>;
    fn set(&mut self, value: &dyn Property);
    fn apply(&mut self, value: &dyn Property);
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
    fn is_sequence(&self) -> bool {
        false
    }

    fn serializable(&self) -> Serializable;
}

pub trait PropertyVal {
    fn val<T: 'static>(&self) -> Option<&T>;
    fn set_val<T: 'static>(&mut self, value: T);
}

impl PropertyVal for dyn Property {
    #[inline]
    fn val<T: 'static>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }

    #[inline]
    fn set_val<T: 'static>(&mut self, value: T) {
        if let Some(prop) = self.any_mut().downcast_mut::<T>() {
            *prop = value;
        } else {
            panic!("prop value is not {}", std::any::type_name::<T>());
        }
    }
}

// used by impl_property
#[allow(unused_macros)]
macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

#[macro_export]
macro_rules! impl_property {
    ($ty:ident) => {
        impl Property for $ty {
            #[inline]
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            #[inline]
            fn any(&self) -> &dyn std::any::Any {
                self
            }

            #[inline]
            fn any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            #[inline]
            fn clone_prop(&self) -> Box<dyn Property> {
                Box::new(self.clone())
            }

            #[inline]
            fn apply(&mut self, value: &dyn Property) {
                self.set(value);
            }

            fn set(&mut self, value: &dyn Property) {
                let value = value.any();
                if let Some(prop) = value.downcast_ref::<Self>() {
                    *self = prop.clone();
                } else {
                    panic!("prop value is not {}", std::any::type_name::<Self>());
                }
            }

            fn serializable(&self) -> Serializable {
                Serializable::Borrowed(self)
            }
       }
    };
    (SEQUENCE, @$trait_:ident [$($args:ident,)*] where [$($preds:tt)+]) => {
        impl_property! {
            @as_item
            impl<$($args),*> Property for $trait_<$($args),*> where $($args: ::std::any::Any + 'static,)*
            $($preds)* {
                #[inline]
                fn type_name(&self) -> &str {
                    std::any::type_name::<Self>()
                }

                #[inline]
                fn any(&self) -> &dyn std::any::Any {
                    self
                }

                #[inline]
                fn any_mut(&mut self) -> &mut dyn std::any::Any {
                    self
                }

                #[inline]
                fn clone_prop(&self) -> Box<dyn Property> {
                    Box::new(self.clone())
                }

                #[inline]
                fn apply(&mut self, value: &dyn Property) {
                    self.set(value);
                }

                fn set(&mut self, value: &dyn Property) {
                    let value = value.any();
                    if let Some(prop) = value.downcast_ref::<Self>() {
                        *self = prop.clone();
                    } else {
                        panic!("prop value is not {}", std::any::type_name::<Self>());
                    }
                }

                fn is_sequence(&self) -> bool {
                    true
                }

                fn serializable(&self) -> Serializable {
                    Serializable::Borrowed(self)
                }
           }
        }
    };
    (@$trait_:ident [$($args:ident,)*] where [$($preds:tt)+]) => {
        impl_property! {
            @as_item
            impl<$($args),*> Property for $trait_<$($args),*> where $($args: ::std::any::Any + 'static,)*
            $($preds)* {
                #[inline]
                fn type_name(&self) -> &str {
                    std::any::type_name::<Self>()
                }

                #[inline]
                fn any(&self) -> &dyn std::any::Any {
                    self
                }

                #[inline]
                fn any_mut(&mut self) -> &mut dyn std::any::Any {
                    self
                }

                #[inline]
                fn clone_prop(&self) -> Box<dyn Property> {
                    Box::new(self.clone())
                }

                #[inline]
                fn apply(&mut self, value: &dyn Property) {
                    self.set(value);
                }

                fn set(&mut self, value: &dyn Property) {
                    let value = value.any();
                    if let Some(prop) = value.downcast_ref::<Self>() {
                        *self = prop.clone();
                    } else {
                        panic!("prop value is not {}", std::any::type_name::<Self>());
                    }
                }

                fn serializable(&self) -> Serializable {
                    Serializable::Borrowed(self)
                }
           }
        }
    };
    (@as_item $i:item) => { $i };

    (
        SEQUENCE, $trait_:ident < $($args:ident),* $(,)* >
        where $($preds:tt)+
    ) => {
        impl_property! {SEQUENCE, @$trait_ [$($args,)*] where [$($preds)*] }
    };
    (
        $trait_:ident < $($args:ident),* $(,)* >
        where $($preds:tt)+
    ) => {
        impl_property! { @$trait_ [$($args,)*] where [$($preds)*] }
    };
}