use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use futures::task::SpawnExt;
use bevy_asset::{Asset, AssetEvent, AssetEvents, AssetId, Assets, Handle};
use bevy_ecs::component::Tick;
use bevy_ecs::event::Events;
use bevy_ecs::prelude::{Res, ResMut, World};
use bevy_ecs::system::{In, Resource, SystemMeta, SystemParam};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
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
    fn new_data(&self) -> usize {
        let average = self.receive_rate();

        average * (Instant::now() - self.last_read).as_micros() as usize
    }

    fn sent_rate(&self) -> usize {
        self.write_rates.calc_average()
    }

    fn receive_rate(&self) -> usize {
        self.read_rates.calc_average()
    }

    fn sent_data(&self) -> usize {
        let average = self.sent_rate();

        average * (Instant::now() - self.last_write).as_micros() as usize
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
    items: [usize; L],
    head: usize
}

impl<const L: usize> RollingAverage<L> {
    fn new() -> Self {
        Self {
            items: [0; L],
            head: 0
        }
    }

    fn calc_average(&self) -> usize {
        let s: u64 = self.items.iter().map(|u| {*u as u64}).sum();
        (s / L as u64) as usize
    }

    fn push(&mut self, new_value: usize) {
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

                while let Ok(n) = entry.buffer.read_from_io(10000.max(entry.new_data())).await {
                    read += n;
                }
                
                let now = Instant::now();
                
                let read_duration = now - entry.last_read;

                entry.read_rates.push(read / (read_duration.as_micros() as usize).max(1));
                
                entry.last_read = now;

                while let Ok(n) = entry.buffer.write_to_io(10000.max(entry.new_data())).await {
                    write += n;
                }
                
                let now = Instant::now();
                
                let write_duration = now - entry.last_write;

                entry.write_rates.push(write / (write_duration.as_micros() as usize).max(1));
                
                entry.last_write = now;

                entry.buffer.additional_updates().await;
            });
        }
    });
}

pub fn handle_socket_events_system<B: Send + Sync + TypePath>
(mut events: ResMut<Events<AssetEvent<BufferEntry<B>>>>, mut sockets: ResMut<Sockets<B>>) {
    for event in events.update_drain() {
        match event {
            AssetEvent::Unused { id } => {
                sockets.inner.remove(id);
            }
            _ => {}
        }
    }
}

#[derive(Resource)]
pub struct Sockets<B>
where B: Send + Sync + TypePath {
    inner: Assets<BufferEntry<B>>
}

#[derive(Debug)]
pub struct Key<B> 
where B: Send + Sync + TypePath {
    inner: Handle<BufferEntry<B>>
}

impl<B> Default for Sockets<B>
where B: Send + Sync + TypePath {
    fn default() -> Self {
        Self {
            inner: Assets::default(),
        }
    }
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
