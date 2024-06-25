use core::fmt;
use managed::{ManagedSlice, SlotVec};

use super::socket_meta::Meta;
use crate::socket::{AnySocket, Socket};

/// Opaque struct with space for storing one socket.
///
/// This is public so you can use it to allocate space for storing
/// sockets when creating an Interface.
pub type SocketStorage<'a> = Item<'a>;

/// An item of a socket set.
#[derive(Debug)]
pub(crate) struct Item<'a> {
    pub(crate) meta: Meta,
    pub(crate) socket: Socket<'a>,
}

/// A handle, identifying a socket in an Interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketHandle(usize);

impl fmt::Display for SocketHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// An extensible set of sockets.
///
/// The lifetime `'a` is used when storing a `Socket<'a>`.  If you're using
/// owned buffers for your sockets (passed in as `Vec`s) you can use
/// `SocketSet<'static>`.
#[derive(Debug)]
pub struct SocketSet<'a> {
    sockets: SlotVec<'a, SocketStorage<'a>>,
}

impl<'a> SocketSet<'a> {
    /// Create a socket set using the provided storage.
    pub fn new<SocketsT>(sockets: SocketsT) -> SocketSet<'a>
    where
        SocketsT: Into<ManagedSlice<'a, Option<SocketStorage<'a>>>>,
    {
        let sockets = SlotVec::new(sockets.into());
        SocketSet { sockets }
    }

    /// Add a socket to the set, and return its handle.
    ///
    /// # Panics
    /// This function panics if the storage is fixed-size (not a `Vec`) and is full.
    pub fn add<T: AnySocket<'a>>(&mut self, socket: T) -> SocketHandle {
        let index = self
            .sockets
            .push_with(|index| {
                net_trace!("[{}]: adding", index);
                let handle = SocketHandle(index);
                let mut meta = Meta::default();
                meta.handle = handle;
                let socket = socket.upcast();
                Item { meta, socket }
            })
            .expect("adding a socket to a full SocketSet");
        self.sockets[index].meta.handle
    }

    /// Get a socket from the set by its handle, as mutable.
    ///
    /// # Panics
    /// This function may panic if the handle does not belong to this socket set
    /// or the socket has the wrong type.
    pub fn get<T: AnySocket<'a>>(&self, handle: SocketHandle) -> &T {
        let item = self
            .sockets
            .get(handle.0)
            .expect("handle does not refer to a valid socket");
        T::downcast(&item.socket).expect("handle refers to a socket of a wrong type")
    }

    /// Get a mutable socket from the set by its handle, as mutable.
    ///
    /// # Panics
    /// This function may panic if the handle does not belong to this socket set
    /// or the socket has the wrong type.
    pub fn get_mut<T: AnySocket<'a>>(&mut self, handle: SocketHandle) -> &mut T {
        let item = self
            .sockets
            .get_mut(handle.0)
            .expect("handle does not refer to a valid socket");
        T::downcast_mut(&mut item.socket).expect("handle refers to a socket of a wrong type")
    }

    /// Remove a socket from the set, without changing its state.
    ///
    /// # Panics
    /// This function may panic if the handle does not belong to this socket set.
    pub fn remove(&mut self, handle: SocketHandle) -> Socket<'a> {
        net_trace!("[{}]: removing", handle.0);
        self.sockets
            .remove(handle.0)
            .map(|item| item.socket)
            .expect("handle does not refer to a valid socket")
    }

    /// Get an iterator to the inner sockets.
    pub fn iter(&self) -> impl Iterator<Item = (SocketHandle, &Socket<'a>)> {
        self.sockets.iter().map(|i| (i.meta.handle, &i.socket))
    }

    /// Get a mutable iterator to the inner sockets.
    pub fn iter_mut(&'a mut self) -> impl Iterator<Item = (SocketHandle, &mut Socket<'a>)> {
        self.sockets
            .iter_mut()
            .map(|i| (i.meta.handle, &mut i.socket))
    }

    /// Checks the handle refers to a valid socket.
    ///
    /// Returns true if the handle refers to a valid socket,
    /// or false if matches any of the following:
    /// - the handle does not belong to this socket set,
    /// - the handle refers to a socket has the wrong type.
    pub fn check<T: AnySocket<'a>>(&self, handle: SocketHandle) -> bool {
        self.sockets
            .get(handle.0)
            .and_then(|item| T::downcast(&item.socket))
            .is_some()
    }
}
