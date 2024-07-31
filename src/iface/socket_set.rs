use super::socket_meta::Meta;
use crate::socket::{AnySocket, Socket, SocketKind};

// pub use self::impl_rwlock_api::*;
pub use spin::lock_api::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard,
    RwLockUpgradableReadGuard, RwLockWriteGuard,
};

pub use self::impl_socket_set::SocketSet;

pub enum RwLockGuard<'a, T> {
    Read(RwLockReadGuard<'a, T>),
    Write(RwLockWriteGuard<'a, T>),
}

impl<'a, T> RwLockGuard<'a, T> {
    pub fn read(self) -> Option<RwLockReadGuard<'a, T>> {
        match self {
            RwLockGuard::Read(ret) => Some(ret),
            RwLockGuard::Write(_) => None,
        }
    }

    pub fn write(self) -> Option<RwLockWriteGuard<'a, T>> {
        match self {
            RwLockGuard::Read(_) => None,
            RwLockGuard::Write(ret) => Some(ret),
        }
    }

    pub fn downgrade(guard: RwLockUpgradableReadGuard<'a, T>) -> RwLockGuard<'a, T> {
        RwLockGuard::Read(RwLockUpgradableReadGuard::downgrade(guard))
    }

    pub fn upgrade(guard: RwLockUpgradableReadGuard<'a, T>) -> RwLockGuard<'a, T> {
        RwLockGuard::Write(RwLockUpgradableReadGuard::upgrade(guard))
    }
}

pub enum MappedRwLockGuard<'a, T: ?Sized> {
    Read(MappedRwLockReadGuard<'a, T>),
    Write(MappedRwLockWriteGuard<'a, T>),
}

impl<'a, T> MappedRwLockGuard<'a, T> {
    pub fn read(self) -> Option<MappedRwLockReadGuard<'a, T>> {
        match self {
            MappedRwLockGuard::Read(ret) => Some(ret),
            MappedRwLockGuard::Write(_) => None,
        }
    }

    pub fn write(self) -> Option<MappedRwLockWriteGuard<'a, T>> {
        match self {
            MappedRwLockGuard::Read(_) => None,
            MappedRwLockGuard::Write(ret) => Some(ret),
        }
    }

    pub fn downgrade<U: ?Sized, F>(
        guard: RwLockUpgradableReadGuard<'a, T>,
        f: F,
    ) -> MappedRwLockGuard<'a, U>
    where
        F: FnOnce(&T) -> &U,
    {
        MappedRwLockGuard::Read(RwLockReadGuard::map(
            RwLockUpgradableReadGuard::downgrade(guard),
            f,
        ))
    }

    pub fn upgrade<U: ?Sized, F>(
        guard: RwLockUpgradableReadGuard<'a, T>,
        f: F,
    ) -> MappedRwLockGuard<'a, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        MappedRwLockGuard::Write(RwLockWriteGuard::map(
            RwLockUpgradableReadGuard::upgrade(guard),
            f,
        ))
    }
}

/// Opaque struct with space for storing one handle.
///
/// A handle, identifying a socket in an Interface.
///
/// The [`new`] method can be used to bind a unique index id to a handle,
/// which is usually the index generated when it is added to a socket set
/// so that it can be retrieved from the socket set. Of course, external
/// relationships can also be provided to index the corresponding socket.
///
/// For simplicity, we do not set the field `handle_id` as a generic input.
/// When customizing the [`AnySocketSet`] implementation, external relations
/// need to decide the conversion themselves.
///
/// [`new`]: SocketHandle::new
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketHandle(usize);

impl SocketHandle {
    #[inline]
    pub fn new(handle_id: usize) -> Self {
        Self(handle_id)
    }

    #[inline]
    pub fn handle_id(&self) -> usize {
        self.0
    }
}

/// Opaque struct with space for storing one socket.
///
/// This is public so you can use it to allocate space for storing
/// sockets when creating an Interface.
// #[derive(Debug)]
pub struct SocketStorage<'a> {
    meta: RwLock<Meta>,
    socket: RwLock<Socket<'a>>,
}

impl<'a> SocketStorage<'a> {
    #[inline]
    pub fn new(handle: SocketHandle, socket: Socket<'a>) -> Self {
        let mut meta = Meta::default();
        meta.handle = handle;
        Self {
            meta: RwLock::new(meta),
            socket: RwLock::new(socket),
        }
    }

    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta.read().handle
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn meta(self) -> Meta {
        self.meta.into_inner()
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn try_meta_lock(&self) -> Option<RwLockUpgradableReadGuard<Meta>> {
        self.meta.try_upgradable_read()
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn try_meta_ref(&self) -> Option<RwLockReadGuard<Meta>> {
        self.meta.try_read()
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn try_meta_mut(&self) -> Option<RwLockWriteGuard<Meta>> {
        self.meta.try_write()
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn meta_lock(&self) -> RwLockUpgradableReadGuard<Meta> {
        self.meta.upgradable_read()
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn meta_ref(&self) -> RwLockReadGuard<Meta> {
        self.meta.read()
    }

    #[inline]
    pub(crate) fn meta_mut(&self) -> RwLockWriteGuard<Meta> {
        self.meta.write()
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn meta_with(&self, f: impl FnOnce(&Meta) -> bool) -> RwLockGuard<Meta> {
        let meta = self.meta.upgradable_read();
        if f(&meta) {
            RwLockGuard::upgrade(meta)
        } else {
            RwLockGuard::downgrade(meta)
        }
    }

    #[inline]
    pub fn socket(self) -> Socket<'a> {
        self.socket.into_inner()
    }

    #[inline]
    pub fn try_socket_lock(&self) -> Option<RwLockUpgradableReadGuard<Socket<'a>>> {
        self.socket.try_upgradable_read()
    }

    #[inline]
    pub fn try_socket_ref(&self) -> Option<RwLockReadGuard<Socket<'a>>> {
        self.socket.try_read()
    }

    #[inline]
    pub fn try_socket_mut(&self) -> Option<RwLockWriteGuard<Socket<'a>>> {
        self.socket.try_write()
    }

    #[inline]
    pub fn socket_ref(&self) -> RwLockReadGuard<Socket<'a>> {
        self.socket.read()
    }

    #[inline]
    pub fn socket_lock(&self) -> RwLockUpgradableReadGuard<Socket<'a>> {
        self.socket.upgradable_read()
    }

    #[inline]
    pub fn socket_mut(&self) -> RwLockWriteGuard<Socket<'a>> {
        self.socket.write()
    }

    #[inline]
    pub fn socket_with(&self, f: impl FnOnce(&Socket<'a>) -> bool) -> RwLockGuard<Socket<'a>> {
        let socket = self.socket.upgradable_read();
        if f(&socket) {
            RwLockGuard::upgrade(socket)
        } else {
            RwLockGuard::downgrade(socket)
        }
    }

    #[inline]
    pub fn can_cast<T: AnySocket<'a>>(&self) -> bool {
        T::downcast(&self.socket_ref()).is_some()
    }

    #[inline]
    pub fn downcast<T: AnySocket<'a>>(&self) -> MappedRwLockReadGuard<T> {
        RwLockReadGuard::map(self.socket.read(), |socket| {
            T::downcast(socket).expect("handle refers to a socket of a wrong type")
        })
        // self.socket.read_map(|socket| {
        //     T::downcast(socket).expect("handle refers to a socket of a wrong type")
        // })
    }

    #[inline]
    pub fn downcast_mut<T: AnySocket<'a>>(&self) -> MappedRwLockWriteGuard<T> {
        RwLockWriteGuard::map(self.socket.write(), |socket| {
            T::downcast_mut(socket).expect("handle refers to a socket of a wrong type")
        })
        // self.socket.write_map(|socket| {
        //     T::downcast_mut(socket).expect("handle refers to a socket of a wrong type")
        // })
    }

    #[inline]
    pub fn downcast_with<T: AnySocket<'a>>(
        &self,
        f: impl FnOnce(&T) -> bool,
    ) -> MappedRwLockGuard<T> {
        let socket = self.socket.upgradable_read();
        if f(T::downcast(&socket).expect("handle refers to a socket of a wrong type")) {
            MappedRwLockGuard::upgrade(socket, |socket| {
                T::downcast_mut(socket).expect("handle refers to a socket of a wrong type")
            })
        } else {
            MappedRwLockGuard::downgrade(socket, |socket| {
                T::downcast(socket).expect("handle refers to a socket of a wrong type")
            })
        }
    }
}

impl<'a> core::fmt::Debug for SocketStorage<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InnerSocketStorage")
            .field("meta", &self.meta)
            .field("socket", &self.socket)
            .finish()
    }
}

impl core::fmt::Display for SocketHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

pub trait AnySocketSet<'a> {
    /// Returns an iterator over the items in the socket set, immutable version..
    fn items<'s>(&'s self) -> impl Iterator<Item = &'s SocketStorage<'a>>
    where
        'a: 's;

    /// Returns an iterator over the items in the socket set, immutable version..
    fn filter<'s>(&'s self, kind: SocketKind) -> impl Iterator<Item = &'s SocketStorage<'a>>
    where
        'a: 's;
}

/// A default implementation for [`AnySocketSet`].
mod impl_socket_set {
    use managed::{ManagedSlice, SlotVec};

    use crate::socket::{AnySocket, Socket, SocketKind};

    use super::{AnySocketSet, MappedRwLockGuard, SocketHandle, SocketStorage};
    use super::{MappedRwLockReadGuard, MappedRwLockWriteGuard};

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
            let mut handle = SocketHandle::default();
            let _index = self
                .sockets
                .push_with(|index| {
                    net_trace!("[{}]: adding", index);
                    handle = SocketHandle::new(index);
                    let socket = socket.upcast();
                    SocketStorage::new(handle, socket)
                })
                .expect("adding a socket to a full SocketSet");
            handle
        }

        /// Get a socket from the set by its handle, as immutable.
        ///
        /// # Panics
        /// This function may panic if the handle does not belong to this socket set.
        pub fn get<T: AnySocket<'a>>(&self, handle: SocketHandle) -> MappedRwLockReadGuard<T> {
            self.sockets
                .get(handle.handle_id())
                .map(|item| item.downcast::<T>())
                .expect("handle does not refer to a valid socket")
        }

        /// Get a socket from the set by its handle, as mutable.
        ///
        /// # Panics
        /// This function may panic if the handle does not belong to this socket set
        /// or the socket has the wrong type.
        pub fn get_mut<T: AnySocket<'a>>(&self, handle: SocketHandle) -> MappedRwLockWriteGuard<T> {
            self.sockets
                .get(handle.handle_id())
                .map(|item| item.downcast_mut::<T>())
                .expect("handle does not refer to a valid socket")
        }

        /// Get a socket from the set by its handle and pass it to the closure.
        /// If the closure return true, return write guard, otherwise read guard.
        /// Optimize performances, only upgrade to a writable lock when necessary.
        ///
        /// # Panics
        /// This function may panic if the handle does not belong to this socket set.
        pub fn with<T: AnySocket<'a>>(
            &self,
            handle: SocketHandle,
            f: impl FnOnce(&T) -> bool,
        ) -> MappedRwLockGuard<T> {
            self.sockets
                .get(handle.handle_id())
                .map(|item| item.downcast_with::<T>(f))
                .expect("handle does not refer to a valid socket")
        }

        /// Remove a socket from the set, without changing its state.
        ///
        /// # Panics
        /// This function may panic if the handle does not belong to this socket set.
        pub fn remove(&mut self, handle: SocketHandle) -> Socket<'a> {
            net_trace!("[{}]: removing", handle.0);
            self.sockets
                .remove(handle.handle_id())
                .map(|item| item.socket())
                .expect("handle does not refer to a valid socket")
        }

        /// Checks the handle refers to a valid socket.
        ///
        /// Returns true if the handle refers to a valid socket,
        /// or false if matches any of the following:
        /// - the handle does not belong to this socket set,
        /// - the handle refers to a socket has the wrong type.
        pub fn check<T: AnySocket<'a>>(&self, handle: SocketHandle) -> bool {
            self.sockets
                .get(handle.handle_id())
                .is_some_and(|item| item.can_cast::<T>())
        }
    }

    impl<'a> AnySocketSet<'a> for SocketSet<'a> {
        fn items<'s>(&'s self) -> impl Iterator<Item = &'s SocketStorage<'a>>
        where
            'a: 's,
        {
            self.sockets.iter()
        }

        fn filter<'s>(&'s self, kind: SocketKind) -> impl Iterator<Item = &'s SocketStorage<'a>>
        where
            'a: 's,
        {
            // It's just a simple implmentation, we also could match for `per kind socket set`.
            self.sockets
                .iter()
                .filter(move |i| i.socket_ref().kind() == kind)
        }
    }
}

/// An implementation for `read/write sync lock`.
mod impl_rwlock_api {
    // Type alias can simplify the code, but it will expose more internal details.
    // We only need to expose the public methods specific to guard and lock impl.

    #[cfg(feature = "single-thread")]
    mod single_thread {
        pub(super) struct RwLockImpl<T> {
            inner: core::cell::RefCell<T>,
        }

        impl<T> RwLockImpl<T> {
            pub(super) fn new(value: T) -> Self {
                Self {
                    inner: core::cell::RefCell::new(value),
                }
            }

            pub(super) fn into_inner(self) -> T {
                self.inner.into_inner()
            }

            pub(super) fn read<'a>(&'a self) -> RwLockReadGuard<'a, T> {
                self.inner.borrow().into()
            }

            pub(super) fn read_map<'a, U: ?Sized, F>(&'a self, f: F) -> MappedRwLockReadGuard<'a, U>
            where
                F: FnOnce(&T) -> &U,
            {
                self.read().map(f)
            }

            pub(super) fn write<'a>(&'a self) -> RwLockWriteGuard<'a, T> {
                self.inner.borrow_mut().into()
            }

            pub(super) fn write_map<'a, U: ?Sized, F>(
                &'a self,
                f: F,
            ) -> MappedRwLockWriteGuard<'a, U>
            where
                F: FnOnce(&mut T) -> &mut U,
            {
                self.write().map(f)
            }
        }

        impl<T: core::fmt::Debug> core::fmt::Debug for RwLockImpl<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.inner.fmt(f)
            }
        }

        pub struct RwLockReadGuard<'a, T> {
            inner: core::cell::Ref<'a, T>,
        }

        impl<'a, T> RwLockReadGuard<'a, T> {
            fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockReadGuard<'a, U>
            where
                F: FnOnce(&T) -> &U,
            {
                MappedRwLockReadGuard {
                    inner: core::cell::Ref::map(self.inner, f),
                }
            }
        }

        impl<'a, T> From<core::cell::Ref<'a, T>> for RwLockReadGuard<'a, T> {
            fn from(value: core::cell::Ref<'a, T>) -> Self {
                Self { inner: value }
            }
        }

        impl<'a, T> core::ops::Deref for RwLockReadGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        pub struct RwLockWriteGuard<'a, T> {
            inner: core::cell::RefMut<'a, T>,
        }

        impl<'a, T> RwLockWriteGuard<'a, T> {
            fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockWriteGuard<'a, U>
            where
                F: FnOnce(&mut T) -> &mut U,
            {
                MappedRwLockWriteGuard {
                    inner: core::cell::RefMut::map(self.inner, f),
                }
            }
        }

        impl<'a, T> From<core::cell::RefMut<'a, T>> for RwLockWriteGuard<'a, T> {
            fn from(value: core::cell::RefMut<'a, T>) -> Self {
                Self { inner: value }
            }
        }

        impl<'a, T> core::ops::Deref for RwLockWriteGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        impl<'a, T> core::ops::DerefMut for RwLockWriteGuard<'a, T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }

        pub struct MappedRwLockReadGuard<'a, T: ?Sized + 'a> {
            inner: core::cell::Ref<'a, T>,
        }

        impl<'a, T: ?Sized> MappedRwLockReadGuard<'a, T> {
            #[inline]
            pub fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockReadGuard<'a, U>
            where
                F: FnOnce(&T) -> &U,
            {
                MappedRwLockReadGuard {
                    inner: core::cell::Ref::map(self.inner, f),
                }
            }
        }

        impl<'a, T> core::ops::Deref for MappedRwLockReadGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        pub struct MappedRwLockWriteGuard<'a, T: ?Sized + 'a> {
            inner: core::cell::RefMut<'a, T>,
        }

        impl<'a, T: ?Sized> MappedRwLockWriteGuard<'a, T> {
            #[inline]
            pub fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockWriteGuard<'a, U>
            where
                F: FnOnce(&mut T) -> &mut U,
            {
                MappedRwLockWriteGuard {
                    inner: core::cell::RefMut::map(self.inner, f),
                }
            }
        }

        impl<'a, T> core::ops::Deref for MappedRwLockWriteGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        impl<'a, T> core::ops::DerefMut for MappedRwLockWriteGuard<'a, T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }
    }
    #[cfg(feature = "single-thread")]
    use single_thread::RwLockImpl;
    #[cfg(feature = "single-thread")]
    pub use single_thread::{
        MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard,
    };

    #[cfg(feature = "multi-thread")]
    mod multi_thread {
        pub(super) struct RwLockImpl<T> {
            inner: spin::lock_api::RwLock<T>,
        }

        impl<T> RwLockImpl<T> {
            pub(super) fn new(value: T) -> Self {
                Self {
                    inner: spin::lock_api::RwLock::new(value),
                }
            }

            pub(super) fn into_inner(self) -> T {
                self.inner.into_inner()
            }

            pub(super) fn read<'a>(&'a self) -> RwLockReadGuard<'a, T> {
                self.inner.read().into()
            }

            pub(super) fn read_map<'a, U: ?Sized, F>(&'a self, f: F) -> MappedRwLockReadGuard<'a, U>
            where
                F: FnOnce(&T) -> &U,
            {
                self.read().map(f)
            }

            pub(super) fn write<'a>(&'a self) -> RwLockWriteGuard<'a, T> {
                self.inner.write().into()
            }

            pub(super) fn write_map<'a, U: ?Sized, F>(
                &'a self,
                f: F,
            ) -> MappedRwLockWriteGuard<'a, U>
            where
                F: FnOnce(&mut T) -> &mut U,
            {
                self.write().map(f)
            }
        }

        impl<T: core::fmt::Debug> core::fmt::Debug for RwLockImpl<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.inner.fmt(f)
            }
        }

        pub struct RwLockReadGuard<'a, T> {
            inner: spin::lock_api::RwLockReadGuard<'a, T>,
        }

        impl<'a, T> RwLockReadGuard<'a, T> {
            fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockReadGuard<'a, U>
            where
                F: FnOnce(&T) -> &U,
            {
                MappedRwLockReadGuard {
                    inner: spin::lock_api::RwLockReadGuard::map(self.inner, f),
                }
            }
        }

        impl<'a, T> From<spin::lock_api::RwLockReadGuard<'a, T>> for RwLockReadGuard<'a, T> {
            fn from(value: spin::lock_api::RwLockReadGuard<'a, T>) -> Self {
                Self { inner: value }
            }
        }

        impl<'a, T> core::ops::Deref for RwLockReadGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        pub struct RwLockWriteGuard<'a, T> {
            inner: spin::lock_api::RwLockWriteGuard<'a, T>,
        }

        impl<'a, T> RwLockWriteGuard<'a, T> {
            fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockWriteGuard<'a, U>
            where
                F: FnOnce(&mut T) -> &mut U,
            {
                MappedRwLockWriteGuard {
                    inner: spin::lock_api::RwLockWriteGuard::map(self.inner, f),
                }
            }
        }

        impl<'a, T> From<spin::lock_api::RwLockWriteGuard<'a, T>> for RwLockWriteGuard<'a, T> {
            fn from(value: spin::lock_api::RwLockWriteGuard<'a, T>) -> Self {
                Self { inner: value }
            }
        }

        impl<'a, T> core::ops::Deref for RwLockWriteGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        impl<'a, T> core::ops::DerefMut for RwLockWriteGuard<'a, T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }
        pub struct MappedRwLockReadGuard<'a, T: ?Sized> {
            inner: spin::lock_api::MappedRwLockReadGuard<'a, T>,
        }

        impl<'a, T: ?Sized> MappedRwLockReadGuard<'a, T> {
            #[inline]
            pub fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockReadGuard<'a, U>
            where
                F: FnOnce(&T) -> &U,
            {
                MappedRwLockReadGuard {
                    inner: spin::lock_api::MappedRwLockReadGuard::map(self.inner, f),
                }
            }
        }

        impl<'a, T> core::ops::Deref for MappedRwLockReadGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        pub struct MappedRwLockWriteGuard<'a, T: ?Sized> {
            inner: spin::lock_api::MappedRwLockWriteGuard<'a, T>,
        }

        impl<'a, T: ?Sized> MappedRwLockWriteGuard<'a, T> {
            #[inline]
            pub fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockWriteGuard<'a, U>
            where
                F: FnOnce(&mut T) -> &mut U,
            {
                MappedRwLockWriteGuard {
                    inner: spin::lock_api::MappedRwLockWriteGuard::map(self.inner, f),
                }
            }
        }

        impl<'a, T> core::ops::Deref for MappedRwLockWriteGuard<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        impl<'a, T> core::ops::DerefMut for MappedRwLockWriteGuard<'a, T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }
    }
    #[cfg(feature = "multi-thread")]
    use multi_thread::RwLockImpl;
    #[cfg(feature = "multi-thread")]
    pub use multi_thread::{
        MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard,
    };

    pub struct RwLock<T> {
        inner: RwLockImpl<T>,
    }

    impl<T> RwLock<T> {
        pub fn new(value: T) -> Self {
            Self {
                inner: RwLockImpl::new(value),
            }
        }

        pub fn into_inner(self) -> T {
            self.inner.into_inner()
        }

        pub fn read<'a>(&'a self) -> RwLockReadGuard<'a, T> {
            self.inner.read()
        }

        pub fn read_map<'a, U: ?Sized, F>(&'a self, f: F) -> MappedRwLockReadGuard<'a, U>
        where
            F: FnOnce(&T) -> &U,
        {
            self.inner.read_map(f)
        }

        pub fn write<'a>(&'a self) -> RwLockWriteGuard<'a, T> {
            self.inner.write()
        }

        pub fn write_map<'a, U: ?Sized, F>(&'a self, f: F) -> MappedRwLockWriteGuard<'a, U>
        where
            F: FnOnce(&mut T) -> &mut U,
        {
            self.inner.write_map(f)
        }
    }

    impl<T: core::fmt::Debug> core::fmt::Debug for RwLock<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.inner.fmt(f)
        }
    }
}
