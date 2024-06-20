use std::borrow::Borrow;
use std::collections::VecDeque;
use std::io::{Error, ErrorKind, IoSlice, IoSliceMut, Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::ops::Deref;
use std::process::Output;
use std::time::Duration;
use bevy_ecs::prelude::ResMut;
use bevy_ecs::system::Resource;
use bevy_internal::tasks::IoTaskPool;
use bevy_internal::utils::hashbrown::Equivalent;
use bevy_internal::utils::HashMap;


pub struct TcpBuffer {
    port: TcpStream,
    read_buf: VecDeque<u8>,
    bytes_read_from_os: usize,
    write_buf: VecDeque<u8>,
    bytes_written: usize,
 
    ttl: Option<u32>,
    derfered_ttl: Option<u32>,

    read_timeout: Option<Option<Duration>>,
    read_timeout_defered: Option<Option<Duration>>,

    write_timeout: Option<Option<Duration>>,
    write_timeout_defered: Option<Option<Duration>>,
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
            derfered_ttl: None,

            read_timeout: stream.read_timeout().ok(),
            read_timeout_defered: None,

            write_timeout: stream.write_timeout().ok(),
            write_timeout_defered: None,

            port: stream,
            read_buf: VecDeque::with_capacity(4096),
            bytes_read_from_os: 0,
            write_buf: VecDeque::with_capacity(4096),
            bytes_written: 0,
        }
    }
    
    //these functions are called internally once every frame
    async fn update_read_buffer(&mut self) -> Result<usize, Error> {

        let mut bytes = Vec::with_capacity(self.bytes_read_from_os);

        let n = self.port.read_to_end(&mut bytes)?;

        
        self.bytes_read_from_os = n;

        for byte in bytes {
            self.read_buf.push_back(byte);
        }
                
        Ok(n)


    }
    
    async fn transfer_write_buffer(&mut self) -> Result<usize, Error> {
        
        let (s1, s2) = self.write_buf.as_slices();

        let n = self.port.write_vectored(&[IoSlice::new(s1), IoSlice::new(s2)])?;
        
        self.write_buf.drain(..n);

        Ok(n)
    }

    async fn exacute_deferred_ttl(&mut self) -> Result<(), Error> {
        if let Some(ttl) = self.derfered_ttl {
            //qualified call to "into()"
            <Result<(), std::io::Error> as Into<Result<(), Error>>>::into(self.port.set_ttl(ttl))?;

            self.ttl = Some(ttl);
        }

        Ok(())
    }

    async fn exacute_deferred_write_timeout(&mut self) -> Result<(), Error> {
        if let Some(timeout) = self.write_timeout_defered {
            self.port.set_write_timeout(timeout)?;
            self.write_timeout = Some(timeout);
        }
        Ok(())
    }

    async fn exacute_deferred_read_timeout(&mut self) -> Result<(), Error> {
        if let Some(timeout) = self.read_timeout_defered {
            self.port.set_read_timeout(timeout)?;
            self.read_timeout = Some(timeout);
        }
        Ok(())
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
        self.derfered_ttl = Some(ttl);
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
            return Some(buffer.port)
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

impl TcpStreams {
    fn handle_io_errors(&mut self, errors: Vec<Option<(TcpStreamKey, Error)>>) {
        for optional_error in errors.iter() {
            if let Some((key, error)) = optional_error {
                match error.kind() {
                    ErrorKind::PermissionDenied => 
                    panic!("This process does not have permission to perform operations. {}", error),

                    ErrorKind::ConnectionRefused | 
                    ErrorKind::ConnectionReset | 
                    ErrorKind::ConnectionAborted | 
                    ErrorKind::NotConnected => {self.deregister(key).unwrap();}

                    ErrorKind::TimedOut | ErrorKind::WouldBlock => {
                        //set socket to blocking mode incase that is the cause of the issue
                        self.streams.get_mut(key).unwrap().port.set_nonblocking(false);
                        //todo figure out the best way to log os blocking errors
                    },
                    unexpected => panic!("An unexpected error occoured! {}", error),
                }
            }
        }
    }

    fn update_reads(&mut self) {
        let io = IoTaskPool::get();

        let errors = io.scope(|s| {
            for entrey in self.streams.iter_mut() {
                s.spawn(async move {
                    match entrey.1.update_read_buffer().await {
                        Err(e) => Some((*entrey.0, e)),
                        _ => None,
                    }
                })
            }
        });

        self.handle_io_errors(errors);
    }

    fn flush_writes(&mut self) {
        let errors = IoTaskPool::get().scope(|s| {
            for (key, buffer) in self.streams.iter_mut() {
                s.spawn(
                    async {
                        match buffer.transfer_write_buffer().await {
                            Err(e) => Some((*key, e)),
                            _ => None,
                        }
                    }
                )
            }
        });

        self.handle_io_errors(errors);
    }

    fn make_defered_changes(&mut self) {
        let io = IoTaskPool::get();

        let ttl_results = io.scope(|s| {
            for (key, buffer) in self.streams.iter_mut() {
                s.spawn(async {
                    if let Err(error) = buffer.exacute_deferred_ttl().await {
                        return Some((*key, error))
                    }
                    None
                });
            }
        });

        let write_timeout_results = io.scope(|s| {
            for (key, buffer) in self.streams.iter_mut() {
                s.spawn(async {
                    if let Err(error) = buffer.exacute_deferred_write_timeout().await {
                        return Some((*key, error))
                    }
                    None
                });
            }
        });

        let read_timeout_results =  io.scope(|s| {
            for (key, buffer) in self.streams.iter_mut() {
                s.spawn(async {
                    if let Err(error) = buffer.exacute_deferred_write_timeout().await {
                        return Some((*key, error))
                    }
                    None
                });
            }
        });

        self.handle_io_errors(ttl_results);
        self.handle_io_errors(write_timeout_results);
        self.handle_io_errors(read_timeout_results);
    }

    fn finish_update(&mut self) {
        for (key, buffer) in self.streams.iter_mut() {
            buffer.prepare_for_next_tick();
        }
    }
}


