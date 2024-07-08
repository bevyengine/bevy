use crate::func::args::{Arg, ArgValue, FromArg};
use crate::func::ArgError;
use crate::{Reflect, TypePath};
use std::collections::VecDeque;

/// A list of arguments that can be passed to a [`DynamicFunction`], [`DynamicClosure`],
/// or [`DynamicClosureMut`].
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{ArgValue, ArgList};
/// let foo = 123;
/// let bar = 456;
/// let mut baz = 789;
/// let args = ArgList::new()
///   // Push an owned argument
///   .push_owned(foo)
///   // Push an owned and boxed argument
///   .push_boxed(Box::new(foo))
///   // Push a reference argument
///   .push_ref(&bar)
///   // Push a mutable reference argument
///   .push_mut(&mut baz)
///   // Push a manually constructed argument
///   .push_arg(ArgValue::Ref(&3.14));
/// ```
///
/// [arguments]: Arg
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicClosure`]: crate::func::DynamicClosure
/// [`DynamicClosureMut`]: crate::func::DynamicClosureMut
#[derive(Default, Debug)]
pub struct ArgList<'a>(VecDeque<Arg<'a>>);

impl<'a> ArgList<'a> {
    /// Create a new empty list of arguments.
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    /// Push an [`ArgValue`] onto the list.
    pub fn push_arg(mut self, arg: ArgValue<'a>) -> Self {
        let index = self.0.len();
        self.0.push_back(Arg::new(index, arg));
        self
    }

    /// Push an [`ArgValue::Ref`] onto the list with the given reference.
    pub fn push_ref(self, arg: &'a dyn Reflect) -> Self {
        self.push_arg(ArgValue::Ref(arg))
    }

    /// Push an [`ArgValue::Mut`] onto the list with the given mutable reference.
    pub fn push_mut(self, arg: &'a mut dyn Reflect) -> Self {
        self.push_arg(ArgValue::Mut(arg))
    }

    /// Push an [`ArgValue::Owned`] onto the list with the given owned value.
    pub fn push_owned(self, arg: impl Reflect) -> Self {
        self.push_arg(ArgValue::Owned(Box::new(arg)))
    }

    /// Push an [`ArgValue::Owned`] onto the list with the given boxed value.
    pub fn push_boxed(self, arg: Box<dyn Reflect>) -> Self {
        self.push_arg(ArgValue::Owned(arg))
    }

    /// Remove the first argument in the list and return it.
    ///
    /// It's generally preferred to use [`Self::next`] instead of this method
    /// as it provides a more ergonomic way to immediately downcast the argument.
    pub fn next_arg(&mut self) -> Result<Arg<'a>, ArgError> {
        self.0.pop_front().ok_or(ArgError::EmptyArgList)
    }

    /// Remove the first argument in the list and return `Ok(T::Item)`.
    ///
    /// If the list is empty or the [`FromArg::from_arg`] call fails, returns an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let a = 1u32;
    /// let b = 2u32;
    /// let mut c = 3u32;
    /// let mut args = ArgList::new().push_owned(a).push_ref(&b).push_mut(&mut c);
    ///
    /// let a = args.next::<u32>().unwrap();
    /// assert_eq!(a, 1);
    ///
    /// let b = args.next::<&u32>().unwrap();
    /// assert_eq!(*b, 2);
    ///
    /// let c = args.next::<&mut u32>().unwrap();
    /// assert_eq!(*c, 3);
    /// ```
    pub fn next<T: FromArg>(&mut self) -> Result<T::Item<'a>, ArgError> {
        self.next_arg()?.take::<T>()
    }

    /// Remove the first argument in the list and return `Ok(T)` if the argument is [`ArgValue::Owned`].
    ///
    /// If the list is empty or the argument is not owned, returns an error.
    ///
    /// It's generally preferred to use [`Self::next`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().push_owned(value);
    /// let value = args.next_owned::<u32>().unwrap();
    /// assert_eq!(value, 123);
    /// ```
    pub fn next_owned<T: Reflect + TypePath>(&mut self) -> Result<T, ArgError> {
        self.next_arg()?.take_owned()
    }

    /// Remove the first argument in the list and return `Ok(&T)` if the argument is [`ArgValue::Ref`].
    ///
    /// If the list is empty or the argument is not a reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::next`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().push_ref(&value);
    /// let value = args.next_ref::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn next_ref<T: Reflect + TypePath>(&mut self) -> Result<&'a T, ArgError> {
        self.next_arg()?.take_ref()
    }

    /// Remove the first argument in the list and return `Ok(&mut T)` if the argument is [`ArgValue::Mut`].
    ///
    /// If the list is empty or the argument is not a mutable reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::next`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let mut value = 123u32;
    /// let mut args = ArgList::new().push_mut(&mut value);
    /// let value = args.next_mut::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn next_mut<T: Reflect + TypePath>(&mut self) -> Result<&'a mut T, ArgError> {
        self.next_arg()?.take_mut()
    }

    /// Remove the last argument in the list and return it.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method
    /// as it provides a more ergonomic way to immediately downcast the argument.
    pub fn pop_arg(&mut self) -> Result<Arg<'a>, ArgError> {
        self.0.pop_back().ok_or(ArgError::EmptyArgList)
    }

    /// Remove the last argument in the list and return `Ok(T::Item)`.
    ///
    /// If the list is empty or the [`FromArg::from_arg`] call fails, returns an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let a = 1u32;
    /// let b = 2u32;
    /// let mut c = 3u32;
    /// let mut args = ArgList::new().push_owned(a).push_ref(&b).push_mut(&mut c);
    ///
    /// let c = args.pop::<&mut u32>().unwrap();
    /// assert_eq!(*c, 3);
    ///
    /// let b = args.pop::<&u32>().unwrap();
    /// assert_eq!(*b, 2);
    ///
    /// let a = args.pop::<u32>().unwrap();
    /// assert_eq!(a, 1);
    /// ```
    pub fn pop<T: FromArg>(&mut self) -> Result<T::Item<'a>, ArgError> {
        self.pop_arg()?.take::<T>()
    }

    /// Remove the last argument in the list and return `Ok(T)` if the argument is [`ArgValue::Owned`].
    ///
    /// If the list is empty or the argument is not owned, returns an error.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().push_owned(value);
    /// let value = args.pop_owned::<u32>().unwrap();
    /// assert_eq!(value, 123);
    /// ```
    pub fn pop_owned<T: Reflect + TypePath>(&mut self) -> Result<T, ArgError> {
        self.pop_arg()?.take_owned()
    }

    /// Remove the last argument in the list and return `Ok(&T)` if the argument is [`ArgValue::Ref`].
    ///
    /// If the list is empty or the argument is not a reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().push_ref(&value);
    /// let value = args.pop_ref::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn pop_ref<T: Reflect + TypePath>(&mut self) -> Result<&'a T, ArgError> {
        self.pop_arg()?.take_ref()
    }

    /// Remove the last argument in the list and return `Ok(&mut T)` if the argument is [`ArgValue::Mut`].
    ///
    /// If the list is empty or the argument is not a mutable reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let mut value = 123u32;
    /// let mut args = ArgList::new().push_mut(&mut value);
    /// let value = args.pop_mut::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn pop_mut<T: Reflect + TypePath>(&mut self) -> Result<&'a mut T, ArgError> {
        self.pop_arg()?.take_mut()
    }

    /// Returns the number of arguments in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the list of arguments is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
