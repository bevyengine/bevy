use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use futures::task::SpawnExt;
use bevy_asset::{Asset, AssetId, Assets};
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
    read_amount: usize,
    write_amount: usize
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

#[derive(Resource)]
struct GlobalManagerData {
    written: AtomicU64,
    read: AtomicU64,
}

pub fn update_ports_system<B>(
    time: Res<Time<Real>>,
    mut buffers: ResMut<Assets<BufferEntry<B>>>
) 
where B: Send + Sync + TypePath + Buffer {
    IoTaskPool::get().scope(|s| {
        for (id, entry) in buffers.iter_mut() {
            s.spawn(async move {
                let time_limit = Duration::from_micros(10);
                
                let mut written = 0;
                let mut read = 0;

                let read_start = Instant::now();
                
                while let Ok(n) = entry.buffer.write_to_io(100000).await {
                    written += n;
                }

                let read_time = Instant::now() - read_start;

                let write_start = Instant::now();

                while let Ok(n) = entry.buffer.read_from_io(100000).await {
                    read += n;
                }
                
                let write_time = Instant::now() - write_start;

                entry.buffer.additional_updates().await;
                
                entry.read_amount = read;
                entry.write_amount = written;
            });
        }
    });

    drop(buffers);
}

