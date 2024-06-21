use std::collections::VecDeque;
use std::io::{Error, ErrorKind, IoSlice, Read, Write};
use std::time::Duration;
use async_net::TcpStream;
use bevy_ecs::prelude::Resource;
use bevy_internal::tasks::IoTaskPool;
use bevy_internal::utils::HashMap;

pub struct TcpBuffer {
    socket: TcpStream,
    read_buf: VecDeque<u8>,
    bytes_read_from_os: usize,
    write_buf: VecDeque<u8>,
    bytes_written: usize,

    ttl: Option<u32>,
    deferred_ttl: Option<u32>
}

pub struct ReadIter<'a> {
    io: &'a mut TcpBuffer,
}

pub struct PeekIter<'a> {
    io: &'a TcpBuffer,
    index: usize
}

impl<'a> Iterator for PeekIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.io.read_buf.get(self.index);

        if let Some(byte) = item {
            self.index += 1;

            return Some(*byte)
        }

        None
    }
}

impl<'a> Iterator for ReadIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.io.read_buf.pop_front()
    }
}

impl TcpBuffer {

    ///For correct funtion stream should be set to blocking mode
    fn new(mut stream: TcpStream) -> Self {
        Self {
            ttl: stream.ttl().ok(),
            deferred_ttl: None,
            socket: stream,
            read_buf: VecDeque::with_capacity(4096),
            bytes_read_from_os: 0,
            write_buf: VecDeque::with_capacity(4096),
            bytes_written: 0,
        }
    }

    //these functions are called internally once every frame
    async fn update_read_buffer(&mut self) -> Result<usize, Error> { 
        todo!() 
    }

    async fn transfer_write_buffer(&mut self) -> Result<usize, Error> {
        todo!()
    }

    async fn execute_deferred_ttl(&mut self) -> Result<(), Error> {
        todo!()
    }
    fn prepare_for_next_tick(&mut self) {
        //the multiply by 5 then divide by 4 here
        //is equivilant to multiplying by 1.25, to give some extra wiggle room
        self.read_buf.shrink_to(((self.bytes_read_from_os * 5 / 4) + self.read_buf.len()).max(4096));
        self.write_buf.shrink_to((self.bytes_written * 5 / 4) + self.write_buf.len().max(4096));

        self.bytes_written = 0;
        self.bytes_read_from_os = 0;
    }
}

//pub impls
impl TcpBuffer {
    pub fn write_bytes<'a, I>(&mut self, iter: I)
        where I: Iterator<Item = &'a u8> {
        self.write_buf.reserve_exact(iter.size_hint().0);

        for byte in iter {
            self.write_buf.push_back(*byte);
            self.bytes_written += 1;
        }
    }

    pub fn read_iter(&mut self) -> ReadIter {
        ReadIter { io: self }
    }

    pub fn peak_iter(&self) -> PeekIter {
        PeekIter { io: self, index: 0 }
    }

    pub fn set_ttl(&mut self, ttl: u32) {
        self.deferred_ttl = Some(ttl);
    }

    pub fn ttl(&self) -> Option<u32> {
        self.ttl
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct TcpStreamKey(u64);

#[derive(Resource, Default)]
pub struct TcpStreams {
    streams: HashMap<TcpStreamKey, TcpBuffer>,
    next_key: u64
}

impl TcpStreams {
    pub fn register(&mut self, stream: TcpStream) -> TcpStreamKey {
        let key = TcpStreamKey(self.next_key);
        self.next_key += 1;

        assert!(self.streams.insert(key, TcpBuffer::new(stream)).is_none());

        key
    }

    pub fn deregister(&mut self, key: &TcpStreamKey) -> Option<TcpStream> {
        if let Some(buffer) = self.streams.remove(key) {
            return Some(buffer.socket)
        }

        None
    }

    pub fn get_mut(&mut self, key: &TcpStreamKey) -> Option<&mut TcpBuffer> {
        self.streams.get_mut(key)
    }

    pub fn get(&self, key: &TcpStreamKey) -> Option<&TcpBuffer> {
        self.streams.get(key)
    }
}