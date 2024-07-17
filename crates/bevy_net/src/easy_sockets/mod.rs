use std::array::from_fn;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::future::Future;
use bevy_ecs::system::Resource;
use bevy_tasks::futures_lite::{AsyncRead, AsyncWrite};
use std::error::Error as StdError;

mod socket_manager;
mod plugin;
mod net_buffer_types;

pub use socket_manager::Sockets;
pub use net_buffer_types::*;
pub use plugin::*;

trait Buffer: Sized {
    
    /// Read upto `target` bytes (or the minimum additional bytes for data integrity)
    /// extra from the io source and return how many were read, return an error
    /// if all currently available bytes have been read, or the another issue occurred.
    fn read_from_io(&mut self, target: usize) -> impl Future<Output = Result<usize, ()>> + Send;

    /// Write upto `target` bytes (or the minimum additional bytes for data integrity)
    /// to the io source and return how many were written, return an error
    /// if there isn't enough available space to write to, or the another issue occurred.
    fn write_to_io(&mut self, target: usize) -> impl Future<Output = Result<usize, ()>> + Send;
    
    /// Called exactly once per frame only after all the reads and writes for 
    /// this socket have been performed for this frame. This functions as a utility 
    /// for any additional state updates.
    fn additional_updates(&mut self) -> impl Future<Output = ()> + Send;
}


