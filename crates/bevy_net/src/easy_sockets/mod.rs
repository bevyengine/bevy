use std::future::Future;
use bevy_ecs::system::Resource;
use bevy_internal::tasks::futures_lite::{AsyncRead, AsyncWrite};
use bevy_internal::tasks::{IoTaskPool, TaskPool};
use bevy_internal::utils::hashbrown::hash_map::{Iter, IterMut};
use bevy_internal::utils::HashMap;
mod tcp_registory;

pub struct AsyncHandler<V> {
    map: HashMap<u64, V>,
    next_key: u64,
}

impl<V> AsyncHandler<V> {
    
    pub fn new() -> Self {
        Self { map: HashMap::default(), next_key: 0 }
    }
    
    pub fn insert(&mut self, value: V) -> u64 {
        let key = self.next_key;
        self.next_key += 1;
        
        assert!(self.map.insert(key, value).is_none());
        
        key
    }
    
    pub fn get(&self, key: &u64) -> Option<&V> {
        self.map.get(key)
    }
    
    pub fn get_mut(&mut self, key: &u64) -> Option<&mut V> {
        self.map.get_mut(key)
    }
    
    pub fn iter_mut(&mut self) -> IterMut<'_, u64, V> {
        self.map.iter_mut()
    }
    
    pub fn iter(&self) -> Iter<'_, u64, V> {
        self.map.iter()
    }
    
    pub fn for_each_async_mut<'a, F, T, Fut>(&'a mut self, function: F, io: &TaskPool) -> Vec<T>
        where F: Fn(&'a u64, &'a mut V) -> Fut, 
              Fut: Future<Output = T>, T: Send + 'static {
        io.scope(|s| {
            for (key, value) in self.map.iter_mut() {
                s.spawn(function(key, value))
            }
        })
    }
}

#[test]
fn test_for_each_async_mut() {
    let pool = TaskPool::new();
    
    let mut map = AsyncHandler::new();
    
    let k1 = map.insert(50);
    let k2 = map.insert(100);
    let k3 = map.insert(200);
    
    map.for_each_async_mut(|key, value| async {
        *value = *value / 2;
    }, &pool);
    
    assert!(*map.get(&k1).unwrap() == 25 && *map.get(&k2).unwrap() == 50 && *map.get(&k3).unwrap() == 100)
}

