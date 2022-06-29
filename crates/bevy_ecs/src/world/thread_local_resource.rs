use thread_local_object::ThreadLocal as ThreadLocalObject;


// This is a thin wrapper around thread local to make some methods take &mut self.
// This makes accessing a &mut T require a NonSendMut from a system
// The underlying implementation panics if there are violations to rusts mutability rules.
// Used by non-send resources to make sending World safe.
pub struct ThreadLocalResource<T: 'static>(ThreadLocalObject<T>);

impl<T: 'static> ThreadLocalResource<T> {
    pub fn new() -> Self {
        ThreadLocalResource(ThreadLocalObject::new())
    }

    pub fn set(&self, value: T) -> Option<T> {
        self.0.set(value)
    }

    pub fn remove(&mut self) -> Option<T> {
        self.0.remove()
    }

    pub fn get<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.get(|t|
            // TODO: add typename to error message. possibly add reference to NonSend System param
            f(t.unwrap_or_else(|| 
                panic!(
                    "Requested non-send resource {} does not exist on this thread.
                    You may be on the wrong thread or need to call .set on the resource.",
                    std::any::type_name::<R>()
                )
            ))
        )
    }

    pub fn get_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.0.get_mut(|t| 
            // TODO: add typename to error message. possibly add reference to NonSend System param
            f(t.unwrap_or_else(||
                panic!(
                "Requested non-send resource {} does not exist on this thread.
                    You may be on the wrong thread or need to call .set on the resource.",
                    std::any::type_name::<R>()
                )
            ))
        )
    }
}

impl<T: 'static + std::fmt::Debug> std::fmt::Debug for ThreadLocalResource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.get(|field| {
            f.debug_tuple("ThreadLocalResource")
            .field(&field)
            .finish()
        })
    }
}

impl<T: 'static + Default> Default for ThreadLocalResource<T> {
    fn default() -> Self {
        ThreadLocalResource::new()
    }
}

// try to drop the resource on the current thread
// Note: this does not necessarily drop every resouce in ThreadLocalResource
// only the one on the thread that drops the TheadLocalResource
impl<T: 'static> Drop for ThreadLocalResource<T> {
    fn drop(&mut self) {
        self.remove();
    }
}
