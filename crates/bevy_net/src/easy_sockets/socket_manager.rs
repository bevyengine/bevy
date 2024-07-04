use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use futures::task::SpawnExt;
use bevy_asset::{Asset, AssetId, Assets, Handle};
use bevy_ecs::prelude::{Res, ResMut};
use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use bevy_tasks::IoTaskPool;
use bevy_time::{Real, Time};
use crate::easy_sockets::Buffer;

#[derive(Asset, TypePath)]
struct BufferEntry<B>
where B: Send + Sync + TypePath {
    buffer: B,
    //bytes per microsecond
    write_rates: RollingAverage<15>,
    read_rates: RollingAverage<15>,
    last_write: Instant,
    last_read: Instant
}

impl<B> BufferEntry<B>
where B: Send + Sync + TypePath {
    /// Estimate the amount of fresh data
    /// arrived since last frame.
    fn new_data(&self) -> f64 {
        let average = self.receive_rate();

        average * (Instant::now() - self.last_read).as_micros() as f64
    }

    fn sent_rate(&self) -> f64 {
        self.write_rates.calc_average()
    }

    fn receive_rate(&self) -> f64 {
        self.read_rates.calc_average()
    }

    fn sent_data(&self) -> f64 {
        let average = self.sent_rate();

        average * (Instant::now() - self.last_write).as_micros() as f64
    }
}

impl<B> Deref for BufferEntry<B> 
where B: Send + Sync + TypePath {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<B> DerefMut for BufferEntry<B>
where B: Send + Sync + TypePath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

struct RollingAverage<const L: usize> {
    items: [f64; L],
    head: usize
}

impl<const L: usize> RollingAverage<L> {
    fn new() -> Self {
        Self {
            items: [0.0; L],
            head: 0
        }
    }

    fn calc_average(&self) -> f64 {
        let s: f64 = self.items.iter().sum();
        s / L as f64
    }

    fn push(&mut self, new_value: f64) {
        self.items[self.head] = new_value;
        self.head = (self.head + 1) % L
    }
}

pub fn update_ports_system<B>(
    mut buffers: ResMut<Assets<BufferEntry<B>>>
) 
where B: Send + Sync + TypePath + Buffer {
    IoTaskPool::get().scope(|s| {
        for (id, entry) in buffers.iter_mut() {
            s.spawn(async move {
                let mut write = 0;
                let mut read = 0;

                let read_start = Instant::now();

                while let Ok(n) = entry.buffer.read_from_io(100000).await {
                    read += n;
                }

                entry.last_read = Instant::now();

                let read_duration = entry.last_read - read_start;

                entry.read_rates.push(read as f64 / read_duration.as_micros() as f64);

                let write_start = Instant::now();

                while let Ok(n) = entry.buffer.write_to_io(100000).await {
                    write += n;
                }

                entry.last_write = Instant::now();

                let write_duration = entry.last_write - write_start;

                entry.write_rates.push(write as f64 / write_duration.as_micros() as f64);

                entry.buffer.additional_updates().await;
            });
        }
    });
}

#[derive(Resource)]
pub struct Sockets<B>
where B: Send + Sync + TypePath {
    inner: ResMut<'static, Assets<BufferEntry<B>>>
}

#[derive(Debug)]
pub struct Key<B> 
where B: Send + Sync + TypePath {
    inner: Handle<BufferEntry<B>>
}

impl<B> Key<B>
where B: Send + Sync + TypePath {
    pub fn weak(&self) -> WeakKey<B> {
        WeakKey { inner: self.inner.clone_weak() }
    }
}

impl<B> GetSocket<B> for Key<B>
where B: Send + Sync + TypePath {
    type Output<'a> = &'a B;

    fn get_socket<'a>(&self, assets: &'a Assets<BufferEntry<B>>) -> Self::Output<'a> {
        assets.get(&self.inner).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct WeakKey<B>
where B: Send + Sync + TypePath {
    inner: Handle<BufferEntry<B>>
}

impl<B> GetSocket<B> for WeakKey<B>
where B: Send + Sync + TypePath {
    type Output<'a> = Option<&'a B>;

    fn get_socket<'a>(&self, assets: &'a Assets<BufferEntry<B>>) -> Self::Output<'a> {
        match assets.get(&self.inner) {
            None => {
                None
            }
            Some(entry) => {
                Some(entry.deref())
            }
        }
    }
}

trait GetSocket<B> 
where B: Send + Sync + TypePath {
    type Output<'a>;
    fn get_socket<'a>(&self, assets: &'a Assets<BufferEntry<B>>) -> Self::Output<'a>;
}

trait GetSocketMut<B>
where B: Send + Sync + TypePath {
    type Output<'a>;
    fn get_socket<'a>(&self, assets: &'a mut Assets<BufferEntry<B>>) -> Self::Output<'a>;
}

impl<B> GetSocketMut<B> for Key<B>
where B: Send + Sync + TypePath {
    type Output<'a> = &'a mut B;

    fn get_socket<'a>(&self, assets: &'a mut Assets<BufferEntry<B>>) -> Self::Output<'a> {
        assets.get_mut(&self.inner).unwrap()
    }
}

impl<B> GetSocketMut<B> for WeakKey<B>
where B: Send + Sync + TypePath {
    type Output<'a> = Option<&'a mut B>;

    fn get_socket<'a>(&self, assets: &'a mut Assets<BufferEntry<B>>) -> Self::Output<'a> {
        if let Some(ref_) = assets.get_mut(&self.inner) {
            return Some(ref_)
        }
        None
    }
}

impl<B> Sockets<B>
where B: Send + Sync + TypePath {
    pub fn get<'a, H: GetSocket<B>>(&'a self, handle: H) -> H::Output<'a> {
        handle.get_socket(&self.inner)
    }
    
    pub fn get_mut<'a, H: GetSocketMut<B>>(&'a mut self, handle: H) -> H::Output<'a> {
        handle.get_socket(&mut self.inner)
    }
    
    pub fn deregister(&mut self, key: Key<B>) -> B {
        self.inner.remove(&key.inner).unwrap().buffer
    }
    
    pub fn register(&mut self, buffer: B) -> Key<B> {
        let now = Instant::now();
        
        Key { inner: self.inner.add(BufferEntry {
            buffer: buffer,
            write_rates: RollingAverage::new(),
            read_rates: RollingAverage::new(),
            last_write: now,
            last_read: now,
        }) }
    }
}
